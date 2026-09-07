use crate::error::ControlError;
use crate::protocol::{BlockDeviceEntry, MountEntry, StorageOverview};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub const DEFAULT_MOUNTS_PATH: &str = "/proc/mounts";
pub const DEFAULT_SYS_BLOCK_PATH: &str = "/sys/block";

#[derive(Debug, Clone)]
pub struct StorageProvider {
    mounts_path: PathBuf,
    sys_block_path: PathBuf,
}

impl StorageProvider {
    pub fn new() -> Self {
        Self {
            mounts_path: PathBuf::from(DEFAULT_MOUNTS_PATH),
            sys_block_path: PathBuf::from(DEFAULT_SYS_BLOCK_PATH),
        }
    }

    pub fn with_paths(mounts_path: impl AsRef<Path>, sys_block_path: impl AsRef<Path>) -> Self {
        Self {
            mounts_path: mounts_path.as_ref().to_path_buf(),
            sys_block_path: sys_block_path.as_ref().to_path_buf(),
        }
    }

    pub fn list_mounts(&self) -> Result<Vec<MountEntry>, ControlError> {
        let file = match File::open(&self.mounts_path) {
            Ok(f) => f,
            Err(_) => return Ok(Vec::new()),
        };

        let reader = BufReader::new(file);
        let mut mounts = Vec::new();

        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => continue,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // /proc/mounts format: device mount_point fs_type options dump_freq pass_num
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }

            let device = parts[0].to_string();
            let mount_point = parts[1].to_string();
            let fs_type = parts[2].to_string();
            let options = parts[3].to_string();

            let mut subvolume = None;
            for opt in options.split(',') {
                if let Some(sub) = opt.strip_prefix("subvol=") {
                    subvolume = Some(sub.to_string());
                    break;
                }
            }

            mounts.push(MountEntry {
                device,
                mount_point,
                fs_type,
                options,
                subvolume,
            });
        }

        Ok(mounts)
    }

    pub fn list_devices(&self) -> Result<Vec<BlockDeviceEntry>, ControlError> {
        if !self.sys_block_path.exists() {
            return Ok(Vec::new());
        }

        let entries = match fs::read_dir(&self.sys_block_path) {
            Ok(e) => e,
            Err(_) => return Ok(Vec::new()),
        };

        let mut devices = Vec::new();

        for entry_res in entries {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let file_name = entry.file_name().to_string_lossy().to_string();
            // Skip loop devices and ram disks for primary storage overview
            if file_name.starts_with("loop") || file_name.starts_with("ram") {
                continue;
            }

            let path = entry.path();
            let size_path = path.join("size");
            let ro_path = path.join("ro");
            let rot_path = path.join("queue/rotational");

            let size_blocks: u64 = fs::read_to_string(&size_path)
                .unwrap_or_default()
                .trim()
                .parse()
                .unwrap_or(0);
            let size_bytes = size_blocks * 512;

            let is_read_only = fs::read_to_string(&ro_path)
                .unwrap_or_default()
                .trim() == "1";

            let is_rotational = fs::read_to_string(&rot_path)
                .unwrap_or_default()
                .trim() == "1";

            devices.push(BlockDeviceEntry {
                name: file_name,
                size_bytes,
                is_rotational,
                is_read_only,
            });
        }

        devices.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(devices)
    }

    pub fn get_overview(&self) -> Result<StorageOverview, ControlError> {
        let mounts = self.list_mounts()?;
        let devices = self.list_devices()?;
        Ok(StorageOverview { mounts, devices })
    }

    pub fn mount(
        &self,
        source: &str,
        target: &str,
        fs_type: Option<&str>,
        options: Option<&str>,
    ) -> Result<String, ControlError> {
        if source.trim().is_empty() || target.trim().is_empty() {
            return Err(ControlError::InvalidParameter {
                param: "source/target".to_string(),
                reason: "Source and target paths cannot be empty".to_string(),
            });
        }

        let mut cmd = std::process::Command::new("mount");
        if let Some(fst) = fs_type {
            cmd.arg("-t").arg(fst);
        }
        if let Some(opts) = options {
            cmd.arg("-o").arg(opts);
        }
        cmd.arg(source).arg(target);

        let output = cmd
            .output()
            .map_err(|e| ControlError::Io(format!("Failed to execute mount command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ControlError::Internal(format!(
                "Mount failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )));
        }

        Ok(format!("Mounted '{}' on '{}' successfully", source, target))
    }
}

impl Default for StorageProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_storage_mounts_parser() {
        let mounts_content = "\
/dev/vda2 / btrfs rw,relatime,ssd,subvolid=256,subvol=@ 0 0
/dev/vda2 /home btrfs rw,relatime,ssd,subvolid=257,subvol=@home 0 0
/dev/vda1 /boot vfat rw,relatime,fmask=0022,dmask=0022 0 0
tmpfs /run tmpfs rw,nosuid,nodev,mode=755 0 0
";

        let dir = std::env::temp_dir();
        let mounts_file = dir.join("test_mounts");
        let sys_block_dir = dir.join("test_sys_block");
        let _ = fs::create_dir_all(&sys_block_dir);

        {
            let mut f = File::create(&mounts_file).unwrap();
            f.write_all(mounts_content.as_bytes()).unwrap();
        }

        let provider = StorageProvider::with_paths(&mounts_file, &sys_block_dir);
        let mounts = provider.list_mounts().expect("list_mounts failed");

        assert_eq!(mounts.len(), 4);

        let root = &mounts[0];
        assert_eq!(root.device, "/dev/vda2");
        assert_eq!(root.mount_point, "/");
        assert_eq!(root.fs_type, "btrfs");
        assert_eq!(root.subvolume, Some("@".to_string()));

        let home = &mounts[1];
        assert_eq!(home.mount_point, "/home");
        assert_eq!(home.subvolume, Some("@home".to_string()));

        let boot = &mounts[2];
        assert_eq!(boot.mount_point, "/boot");
        assert_eq!(boot.fs_type, "vfat");
        assert_eq!(boot.subvolume, None);

        let _ = fs::remove_file(&mounts_file);
        let _ = fs::remove_dir_all(&sys_block_dir);
    }
}
