# MahinaOS — Boot Architecture & Image Tooling Specification

> **Status**: Authoritative Boot Specification — Milestone M1 (Phase 3)  
> **Document Revision**: 1.0.0 (2026-09-08)  
> **Repository**: [MahinaOS](https://github.com/HardikBhaskar2010/MahinaOS)  
> **Target Subsystems**: Limine, Linux 6.6 LTS, `initramfs`, `luna-init`, `scripts/build-image.sh`

---

## 1. Boot Architecture Overview

MahinaOS employs a clean, deterministic, modern UEFI boot pipeline. Legacy BIOS, MBR partition tables, SysV init, and systemd are strictly excluded. The entire execution sequence from power-on to the user desktop is designed to boot in under **1.8 seconds** on modern NVMe hardware.

```
+-------------------------------------------------------------------------+
| [STAGE 0] UEFI FIRMWARE                                                 |
| Hardware initialization, NVRAM variables, loads /EFI/BOOT/BOOTX64.EFI   |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [STAGE 1] LIMINE BOOTLOADER                                             |
| Parses boot/limine.conf; loads vmlinuz-mahina & initramfs-mahina.img    |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [STAGE 2] LINUX KERNEL 6.6 LTS                                          |
| Decompresses; initializes CPU, memory, built-in DRM/KMS, virtio, Btrfs  |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [STAGE 3] EARLY INITRAMFS (/init)                                       |
| Mounts devtmpfs; locates root volume; switch_root to /mnt/root          |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [STAGE 4] MAHINA CORE (luna-init / PID 1)                               |
| Static musl binary; establishes /run/luna-init.sock; resolves DAG;      |
| launches early splash (luna-splash)                                     |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [STAGE 5] MAHINA EXPERIENCE (lgp-compositor & luna-shell-rs)            |
| Direct DRM/KMS dumb buffers; LGP socket; renders desktop UI & Island    |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [STAGE 6] INTELLIGENCE (luna-ai-d)                                      |
| Background daemon starts; validates capabilities; registers presence    |
+-------------------------------------------------------------------------+
```

---

## 2. Detailed Stage-by-Stage Execution

### Stage 0: UEFI Firmware Hand-off
- **Target Platforms**: Physical x86_64 UEFI 2.4+ firmware, or virtualized OVMF/EDK2 (`OVMF_CODE.fd`).
- **Disk Partitioning**: GPT (GUID Partition Table).
  - Partition 1: EFI System Partition (ESP), FAT32, `128 MiB`, type GUID `C12A7328-F81F-11D2-BA4B-00A0C93EC93B`.
  - Partition 2: System Root, Btrfs, remaining disk space, labeled `MAHINA_ROOT`.
- **Execution**: Firmware executes `/EFI/BOOT/BOOTX64.EFI` from ESP.

### Stage 1: Limine Bootloader Protocol
- **Binary**: Limine UEFI loader (`BOOTX64.EFI`).
- **Configuration Locations**: Checked in order:
  - `/boot/efi/limine.conf`
  - `/boot/efi/EFI/BOOT/limine.conf`
  - `/boot/efi/boot/limine/limine.conf`
- **Protocol**: Standard Limine 5/7/8 Linux protocol.
- **Kernel Command Line Parameters**:
  ```text
  console=tty0 console=ttyS0 loglevel=3 quiet root=LABEL=MAHINA_ROOT rootflags=subvol=@ init=/init video=1024x768
  ```
  - `console=tty0 console=ttyS0`: Dual output for virtual console and serial port automation.
  - `loglevel=3 quiet`: Suppresses kernel informational spam, allowing smooth splash presentation.
  - `root=LABEL=MAHINA_ROOT`: Filesystem-agnostic label resolution instead of fragile hardcoded device nodes.

### Stage 2: Linux Kernel 6.6 LTS
- **Architecture**: Monolithic x86_64 bzImage (`/boot/efi/vmlinuz-mahina`).
- **Built-in Drivers (`=y`, no initramfs module load required)**:
  - Storage: `CONFIG_BTRFS_FS=y`, `CONFIG_VIRTIO_BLK=y`, `CONFIG_SCSI=y`, `CONFIG_BLK_DEV_SD=y`
  - Graphics: `CONFIG_DRM=y`, `CONFIG_DRM_VIRTIO_GPU=y`, `CONFIG_DRM_FBDEV_EMULATION=y`, `CONFIG_SYSFB_SIMPLEFB=y`, `CONFIG_DRM_SIMPLEDRM=y`
  - Virtualization & Hardware: `CONFIG_DEVTMPFS=y`, `CONFIG_DEVTMPFS_MOUNT=y`, `CONFIG_VT=y`, `CONFIG_FRAMEBUFFER_CONSOLE=y`
- Kernel mounts initial ramdisk into root memory and executes `/init`.

### Stage 3: Early Initramfs Script (`/init`)
The early initramfs environment is minimal, self-contained, and transient:
1. Mounts essential pseudo-filesystems:
   ```sh
   mount -t proc proc /proc
   mount -t sysfs sys /sys
   mount -t devtmpfs dev /dev
   ```
2. Discovers root filesystem by label (`MAHINA_ROOT`) or fallback device (`/dev/vda2`):
   ```sh
   ROOT_DEV=$(blkid -L MAHINA_ROOT || echo "/dev/vda2")
   ```
3. Mounts real Btrfs root subvolume `@` onto `/mnt/root`:
   ```sh
   mount -t btrfs -o rw,subvol=@ "${ROOT_DEV}" /mnt/root
   ```
4. Cleans up early pseudo-filesystems:
   ```sh
   umount /dev
   umount /sys
   umount /proc
   ```
5. Pivots root and transfers execution to `luna-init` PID 1:
   ```sh
   exec busybox switch_root /mnt/root /sbin/luna-init
   ```

### Stage 4: `luna-init` (PID 1) Service DAG
`luna-init` takes ownership of PID 1:
1. **Mounts Real Hierarchy**: Mounts `/proc`, `/sys`, `/dev`, `/run`, and `/tmp`.
2. **IPC Server Initialization**: Binds to `/run/luna-init.sock` with restrictive `0600` permissions.
3. **Signal & Zombie Reaper**: Installs `SIGCHLD`, `SIGTERM`, `SIGINT`, and `SIGPWR` signal handlers; launches asynchronous non-blocking process reaper (`waitpid(-1, &st, WNOHANG)`).
4. **Early Splash**: Executes `/usr/sbin/luna-splash` to present animated progress on the frame buffer before the compositor takes over.
5. **DAG Resolution**: Reads service manifests in `/etc/luna/services/` (e.g. `10-udev`, `20-dbus`, `30-network`, `40-compositor`, `50-shell`, `60-ai`) and starts them in topological dependency order.

### Stage 5: Compositor & Desktop Startup
1. `luna-init` launches `lgp-compositor`:
   - Acquires `/dev/dri/card0` via DRM master.
   - Sets up page flipping and double dumb-buffers.
   - Binds UNIX domain socket `/run/lgp.sock` with group `video` (`lookup_video_gid()`).
2. `luna-init` launches `luna-shell-rs`:
   - Connects to `/run/lgp.sock` via `lgp-rs`.
   - Creates root desktop surface, taskbar, dock, and status tray.
3. `luna-island-rs` launches as a floating status HUD.

### Stage 6: Intelligence Daemon (`luna-ai-d`)
1. Runs under unprivileged identity `luna-ai:luna-ai` (UID/GID 950).
2. Connects to `/run/mahina/control.sock` to listen for system context changes.
3. Binds to `/run/mahina/ai.sock` to service shell and UI intent queries.

---

## 3. Image Tooling Architecture & Auditing

The repository contains three primary shell tools for generating and validating bootable disk images:

### 3.1 `scripts/build-initramfs.sh`
- **Output**: `build/initramfs-mahina.img`
- **Audit Findings & Enhancements**:
  - **Static Linking Requirement**: All utilities placed in `sbin/` and `bin/` inside the initramfs must be statically linked. Currently, `luna-init` and `busybox` are static.
  - **Dynamic Root Parsing**: Replaced hardcoded `/dev/vda2` with dynamic label lookup (`blkid -L MAHINA_ROOT`).
  - **Fail-Safe Shell**: If the root filesystem fails to mount, execution drops to an interactive busybox shell (`exec busybox setsid busybox cttyhack sh`) for debugging rather than panicking.

### 3.2 `scripts/build-image.sh`
- **Output**: `build/mahina-0.1.0.img` (2 GB raw GPT disk image)
- **Subvolume Structure**:
  - `@`: Root filesystem (OS binaries, `/usr`, `/etc`, `/var`).
  - `@home`: Persistent user directories (`/home/<user>`), untouched during OS upgrades.
  - `@snapshots`: Pre-update recovery points.
  - `@generations`: Staged generational OS images for A/B rollback.
- **Bootloader Deployment**: Installs Limine EFI binaries to `ESP:/EFI/BOOT/BOOTX64.EFI` and mirrors `boot/limine.conf` across both `/boot/efi/` and `/boot/efi/EFI/BOOT/`.

### 3.3 `scripts/run-qemu.sh` (Headless and Graphical Smoke Testing)
- **Engine**: QEMU x86_64 with KVM acceleration (`qemu-system-x86_64 -machine q35 -enable-kvm`).
- **Firmware**: OVMF UEFI firmware (`/usr/share/OVMF/OVMF_CODE.fd`).
- **Display**: VirtIO GPU (`-vga virtio -display gtk` or `-display none` with serial monitoring).
- **Serial Harness**: `-serial mon:stdio` enables continuous automated parsing of early kernel and `luna-init` boot logs.

---

## 4. Boot Recovery & Failure Mode Matrix

| Failure Mode | Detection Point | Diagnostic Symptom | Remediation / Fallback |
|---|---|---|---|
| **Corrupt Limine Config** | Stage 1 (Bootloader) | Limine interactive boot prompt | Bootloader falls back to built-in default entry or prior config. |
| **Missing Kernel / Initramfs** | Stage 1 (Bootloader) | "File not found: vmlinuz-mahina" | Select fallback entry in Limine menu pointing to previous generation. |
| **Kernel Panic / Hardware Fault** | Stage 2 (Kernel) | Freeze or serial dump before `/init` | Hardware watchdog reboots VM; Limine boot-counter decrements. |
| **Failed Root Mount (Btrfs)** | Stage 3 (Initramfs) | "[initramfs] FATAL: Failed to mount real root" | Initramfs drops to emergency Busybox shell with serial tty. |
| **`luna-init` Crash (PID 1)** | Stage 4 (Mahina Core) | Kernel panic: "Attempted to kill init!" | Watchdog reboots into recovery generation (`@generations/previous`). |
| **Compositor Startup Failure** | Stage 5 (Experience) | Black screen or `/dev/dri` busy | `luna-init` restarts compositor up to 3 times, then falls back to text VT. |
| **`luna-ai-d` Crash** | Stage 6 (Intelligence) | Desktop remains functional; Island shows offline | `luna-init` isolates daemon; user desktop is completely unaffected. |

---

## 5. Milestone M1 Verification Criteria

A build satisfies the **Milestone M1 (Bootable Mahina)** gate if and only if:
1. `scripts/build-initramfs.sh` executes without error, producing a valid cpio archive.
2. `scripts/build-image.sh` produces a valid 2GB GPT disk image with EFI ESP and Btrfs root.
3. In headless QEMU, the image boots through Limine, executes `vmlinuz-mahina`, runs `initramfs`, mounts `@`, transitions root, and executes `luna-init`.
4. Serial console output confirms `[luna-init] Starting service DAG` and reaches steady state without hanging.
