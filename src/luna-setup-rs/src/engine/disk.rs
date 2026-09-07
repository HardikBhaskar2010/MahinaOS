use std::process::Command;
use nix::mount::{mount, MsFlags};
use std::path::Path;
use std::fs;

#[derive(Debug)]
#[allow(dead_code)]
pub enum DiskError {
    PartitionFailed(String),
    FormatFailed(String),
    MountFailed(String),
}

pub fn partition_disk(disk_path: &str) -> Result<(), DiskError> {
    // We use parted to create a GPT table, an EFI System Partition, and a Linux Root partition.
    // EFI: 1MiB to 513MiB
    // Root: 513MiB to 100%
    let output = Command::new("parted")
        .arg("-s")
        .arg(disk_path)
        .arg("mklabel")
        .arg("gpt")
        .arg("mkpart")
        .arg("ESP")
        .arg("fat32")
        .arg("1MiB")
        .arg("513MiB")
        .arg("set")
        .arg("1")
        .arg("esp")
        .arg("on")
        .arg("mkpart")
        .arg("primary")
        .arg("ext4")
        .arg("513MiB")
        .arg("100%")
        .output()
        .map_err(|e| DiskError::PartitionFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(DiskError::PartitionFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    
    // Give the kernel time to re-read partition tables
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

pub fn format_partitions(disk_path: &str) -> Result<(), DiskError> {
    // Determine partition names. NVMe drives use 'p1', 'p2'. SDA drives use '1', '2'.
    let (part1, part2) = if disk_path.contains("nvme") {
        (format!("{}p1", disk_path), format!("{}p2", disk_path))
    } else {
        (format!("{}1", disk_path), format!("{}2", disk_path))
    };

    // Format ESP
    let out_esp = Command::new("mkfs.fat")
        .arg("-F32")
        .arg(&part1)
        .output()
        .map_err(|e| DiskError::FormatFailed(e.to_string()))?;

    if !out_esp.status.success() {
        return Err(DiskError::FormatFailed(
            String::from_utf8_lossy(&out_esp.stderr).to_string(),
        ));
    }

    // Format Root as Btrfs
    let out_root = Command::new("mkfs.btrfs")
        .arg("-f")
        .arg("-L")
        .arg("MAHINA_ROOT")
        .arg(&part2)
        .output()
        .map_err(|e| DiskError::FormatFailed(e.to_string()))?;

    if !out_root.status.success() {
        return Err(DiskError::FormatFailed(
            String::from_utf8_lossy(&out_root.stderr).to_string(),
        ));
    }

    Ok(())
}

pub fn mount_target(disk_path: &str, target_dir: &str) -> Result<(), DiskError> {
    let (part1, part2) = if disk_path.contains("nvme") {
        (format!("{}p1", disk_path), format!("{}p2", disk_path))
    } else {
        (format!("{}1", disk_path), format!("{}2", disk_path))
    };

    // Create temporary mount point to initialize Btrfs subvolumes
    let tmp_mnt = "/tmp/mahina_btrfs_init";
    if !Path::new(tmp_mnt).exists() {
        fs::create_dir_all(tmp_mnt).map_err(|e| DiskError::MountFailed(e.to_string()))?;
    }

    // Mount top-level Btrfs volume
    mount(
        Some(part2.as_str()),
        tmp_mnt,
        Some("btrfs"),
        MsFlags::empty(),
        None::<&str>,
    ).map_err(|e| DiskError::MountFailed(format!("Failed to mount Btrfs top-level: {}", e)))?;

    // Create subvolumes: @ (root), @home, @snapshots, @generations
    let _ = Command::new("btrfs").args(["subvolume", "create", &format!("{}/@", tmp_mnt)]).output();
    let _ = Command::new("btrfs").args(["subvolume", "create", &format!("{}/@home", tmp_mnt)]).output();
    let _ = Command::new("btrfs").args(["subvolume", "create", &format!("{}/@snapshots", tmp_mnt)]).output();
    let _ = Command::new("btrfs").args(["subvolume", "create", &format!("{}/@generations", tmp_mnt)]).output();

    let _ = nix::mount::umount(tmp_mnt);

    // Create target dir
    if !Path::new(target_dir).exists() {
        fs::create_dir_all(target_dir).map_err(|e| DiskError::MountFailed(e.to_string()))?;
    }

    // Mount @ subvolume as target root
    mount(
        Some(part2.as_str()),
        target_dir,
        Some("btrfs"),
        MsFlags::empty(),
        Some("subvol=@"),
    ).map_err(|e| DiskError::MountFailed(format!("Failed to mount root subvolume: {}", e)))?;

    // Mount @home subvolume
    let home_dir = format!("{}/home", target_dir);
    if !Path::new(&home_dir).exists() {
        fs::create_dir_all(&home_dir).map_err(|e| DiskError::MountFailed(e.to_string()))?;
    }
    mount(
        Some(part2.as_str()),
        home_dir.as_str(),
        Some("btrfs"),
        MsFlags::empty(),
        Some("subvol=@home"),
    ).map_err(|e| DiskError::MountFailed(format!("Failed to mount @home subvolume: {}", e)))?;

    // Mount ESP at /boot/efi
    let efi_dir = format!("{}/boot/efi", target_dir);
    if !Path::new(&efi_dir).exists() {
        fs::create_dir_all(&efi_dir).map_err(|e| DiskError::MountFailed(e.to_string()))?;
    }

    mount(
        Some(part1.as_str()),
        efi_dir.as_str(),
        Some("vfat"),
        MsFlags::empty(),
        None::<&str>,
    ).map_err(|e| DiskError::MountFailed(format!("Failed to mount ESP: {}", e)))?;

    Ok(())
}
