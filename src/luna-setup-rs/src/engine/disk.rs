use std::process::Command;
use nix::mount::{mount, MsFlags};
use std::path::Path;
use std::fs;

#[derive(Debug)]
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

    // Format Root
    let out_root = Command::new("mkfs.ext4")
        .arg("-F") // Force
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

    // Create target dir
    if !Path::new(target_dir).exists() {
        fs::create_dir_all(target_dir).map_err(|e| DiskError::MountFailed(e.to_string()))?;
    }

    // Mount root
    mount(
        Some(part2.as_str()),
        target_dir,
        Some("ext4"),
        MsFlags::empty(),
        None::<&str>,
    ).map_err(|e| DiskError::MountFailed(format!("Failed to mount root: {}", e)))?;

    // Mount ESP
    let boot_dir = format!("{}/boot", target_dir);
    if !Path::new(&boot_dir).exists() {
        fs::create_dir_all(&boot_dir).map_err(|e| DiskError::MountFailed(e.to_string()))?;
    }

    mount(
        Some(part1.as_str()),
        boot_dir.as_str(),
        Some("vfat"),
        MsFlags::empty(),
        None::<&str>,
    ).map_err(|e| DiskError::MountFailed(format!("Failed to mount ESP: {}", e)))?;

    Ok(())
}
