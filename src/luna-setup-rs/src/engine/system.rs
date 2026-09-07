use std::process::Command;
use std::path::Path;
use std::fs;
use std::io;

#[derive(Debug)]
#[allow(dead_code)]
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
        "# /etc/fstab: static file system information for MahinaOS
# <file system>             <mount point>   <type>  <options>                       <dump>  <pass>
UUID={}       /               btrfs   rw,subvol=@,relatime            0       0
UUID={}       /home           btrfs   rw,subvol=@home,relatime        0       0
UUID={}       /boot/efi       vfat    rw,relatime                     0       2
", root_uuid, root_uuid, esp_uuid
    );

    let etc_dir = Path::new(target_dir).join("etc");
    fs::create_dir_all(&etc_dir).unwrap_or(());
    fs::write(etc_dir.join("fstab"), fstab_content)
        .map_err(|e| SystemError::FstabFailed(e.to_string()))?;

    Ok(())
}

pub fn install_bootloader(target_dir: &str) -> Result<(), SystemError> {
    // For UEFI, copy Limine EFI loader and config to ESP (/boot/efi/EFI/BOOT)
    let esp_boot_dir = Path::new(target_dir).join("boot/efi/EFI/BOOT");
    fs::create_dir_all(&esp_boot_dir).map_err(|e| SystemError::BootloaderFailed(e.to_string()))?;

    // Copy BOOTX64.EFI from live media or system locations
    let limine_candidates = [
        "/boot/efi/EFI/BOOT/BOOTX64.EFI",
        "/usr/share/limine/BOOTX64.EFI",
        "/build/limine/BOOTX64.EFI",
    ];

    for src in &limine_candidates {
        if Path::new(src).exists() {
            let _ = fs::copy(src, esp_boot_dir.join("BOOTX64.EFI"));
            break;
        }
    }

    // Copy limine.conf to ESP locations
    let limine_conf_candidates = [
        "/boot/efi/limine.conf",
        "/boot/limine.conf",
        "/etc/luna/limine.conf",
    ];

    for src in &limine_conf_candidates {
        if Path::new(src).exists() {
            let _ = fs::copy(src, esp_boot_dir.join("limine.conf"));
            let _ = fs::copy(src, Path::new(target_dir).join("boot/efi/limine.conf"));
            break;
        }
    }

    // Copy kernel and initramfs to /boot/efi
    let efi_dir = Path::new(target_dir).join("boot/efi");
    for k in &["/boot/efi/vmlinuz-mahina", "/boot/vmlinuz-mahina", "/build/vmlinuz-mahina"] {
        if Path::new(k).exists() {
            let _ = fs::copy(k, efi_dir.join("vmlinuz-mahina"));
            break;
        }
    }
    for init in &["/boot/efi/initramfs-mahina.img", "/boot/initramfs-mahina.img", "/build/initramfs-mahina.img"] {
        if Path::new(init).exists() {
            let _ = fs::copy(init, efi_dir.join("initramfs-mahina.img"));
            break;
        }
    }

    Ok(())
}
