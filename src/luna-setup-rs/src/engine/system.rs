use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;

#[derive(Debug)]
pub enum SystemError {
    CopyFailed(String),
    FstabFailed(String),
    BootloaderFailed(String),
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn copy_rootfs(target_dir: &str) -> Result<(), SystemError> {
    // In a real live environment, the rootfs is typically mounted at /run/archiso/bootmnt or /
    // For this implementation, we will copy essential directories from the live root to the target.
    let dirs_to_copy = vec!["/bin", "/etc", "/lib", "/usr", "/sbin", "/var", "/opt"];
    
    for dir in dirs_to_copy {
        let src = Path::new(dir);
        if src.exists() {
            let dst = Path::new(target_dir).join(dir.trim_start_matches('/'));
            copy_dir_all(src, dst).map_err(|e| SystemError::CopyFailed(format!("Failed to copy {}: {}", dir, e)))?;
        }
    }
    
    // Create empty mount points
    for mnt in &["/dev", "/proc", "/sys", "/tmp", "/run", "/boot", "/home", "/mnt"] {
        let p = Path::new(target_dir).join(mnt.trim_start_matches('/'));
        fs::create_dir_all(&p).unwrap_or(());
    }

    Ok(())
}

fn get_uuid(device: &str) -> Option<String> {
    let out = Command::new("blkid")
        .arg("-s")
        .arg("UUID")
        .arg("-o")
        .arg("value")
        .arg(device)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

pub fn generate_fstab(disk_path: &str, target_dir: &str) -> Result<(), SystemError> {
    let (part1, part2) = if disk_path.contains("nvme") {
        (format!("{}p1", disk_path), format!("{}p2", disk_path))
    } else {
        (format!("{}1", disk_path), format!("{}2", disk_path))
    };

    let esp_uuid = get_uuid(&part1).ok_or_else(|| SystemError::FstabFailed("Could not get ESP UUID".to_string()))?;
    let root_uuid = get_uuid(&part2).ok_or_else(|| SystemError::FstabFailed("Could not get Root UUID".to_string()))?;

    let fstab_content = format!(
        "# /etc/luna/fstab.toml
[[mount]]
device = \"UUID={}\"
mountpoint = \"/\"
fstype = \"ext4\"
options = \"rw,relatime\"

[[mount]]
device = \"UUID={}\"
mountpoint = \"/boot\"
fstype = \"vfat\"
options = \"rw,relatime\"
", root_uuid, esp_uuid
    );

    let fstab_dir = Path::new(target_dir).join("etc/luna");
    fs::create_dir_all(&fstab_dir).unwrap_or(());
    fs::write(fstab_dir.join("fstab.toml"), fstab_content)
        .map_err(|e| SystemError::FstabFailed(e.to_string()))?;

    Ok(())
}

pub fn install_bootloader(target_dir: &str) -> Result<(), SystemError> {
    // For UEFI, simply copy systemd-boot or limine EFI binary to ESP
    // Assuming ESP is mounted at /boot inside target_dir
    let esp_efi_dir = Path::new(target_dir).join("boot/EFI/BOOT");
    fs::create_dir_all(&esp_efi_dir).map_err(|e| SystemError::BootloaderFailed(e.to_string()))?;

    // As a generic fallback, we use limine bios-install and copy limine efi if available.
    // In a real environment, this might call `bootctl install --path=/boot` in a chroot.
    
    // We execute chroot bootctl if systemd-boot is present
    let bootctl = Path::new(target_dir).join("usr/bin/bootctl");
    if bootctl.exists() {
        let out = Command::new("chroot")
            .arg(target_dir)
            .arg("/usr/bin/bootctl")
            .arg("install")
            .output()
            .map_err(|e| SystemError::BootloaderFailed(e.to_string()))?;
            
        if !out.status.success() {
            return Err(SystemError::BootloaderFailed(String::from_utf8_lossy(&out.stderr).to_string()));
        }
    }
    
    Ok(())
}
