# Mahina OS

**Version:** Waxing 0.1.0 (Stage 0 — Bring-up)
**Architecture:** x86_64 · UEFI · Linux 6.6.x LTS

> A new operating system built around presence, local AI, and user control.
> This is v0.1 — the boot chain. Nothing graphical yet. Just alive.

---

## What This Is

Mahina is built from scratch (no upstream distro base). This repository contains:

- `luna-init` — PID 1 init system (C17, statically linked)
- `luna-init-ctl` — Runtime control CLI
- Bootloader configuration (limine)
- Kernel configuration (Linux 6.6.x LTS)
- Build and QEMU scripts
- Service definitions (TOML)
- Architecture documentation (`docs/DCKL/`)

See the [DCKL](docs/DCKL/) for the full architecture specification.

---

## Build Requirements

**Build host:** Linux or WSL2 (Ubuntu 22.04+ recommended)

```bash
# Install dependencies (Ubuntu/WSL2)
sudo apt update
sudo apt install -y \
    clang lld llvm \
    make \
    qemu-system-x86 ovmf \
    parted dosfstools btrfs-progs \
    busybox-static \
    cpio gzip \
    clang-tidy \
    afl++
```

---

## Building

```bash
# Build luna-init and luna-init-ctl
make all

# Build disk image (requires sudo for loopback device)
make image

# Run in QEMU (builds image first)
make run-qemu

# Run unit tests (ASan + UBSan enabled)
make test-unit

# Run static analysis
make lint

# Clean build artifacts
make clean
```

---

## QEMU Launch

```bash
make run-qemu
```

Serial console output appears in the terminal. A GTK window shows the display.

**Debug console access:** `Ctrl+Alt+2` in QEMU GTK window switches to QEMU monitor.

**Expected boot output (Stage 0):**
```
[0000] [0] [luna-init] [INFO] PID 1 alive. Mahina Waxing 0.1.0
[XXXX] [2] [luna-init] [INFO] Mounted: /proc
[XXXX] [2] [luna-init] [INFO] Mounted: /sys
[XXXX] [2] [luna-init] [INFO] Mounted: /dev
[XXXX] [2] [luna-init] [INFO] Mounted: /tmp
[XXXX] [2] [luna-init] [INFO] Mounted: /run
[XXXX] [3] [luna-init] [INFO] Hostname: mahinabox
[XXXX] [4] [luna-init] [WARN] Service 'dbus': binary not found — DEGRADED
[XXXX] [4] [luna-init] [WARN] Service 'networkmanager': binary not found — DEGRADED
... (Stage 4 services DEGRADED — expected in v0.1)
[XXXX] [*] [luna-init] [INFO] Boot complete. Status: DEGRADED

Welcome to Mahina.
Architecture Freeze v1
System Initialization Complete.

[root@mahinabox /]#
```

Stage 4 services (D-Bus, NetworkManager, PipeWire) show as DEGRADED in v0.1. Their binaries are not yet installed. This is correct and expected behavior.

---

## Kernel

The kernel is Linux 6.6.x LTS. It is **not** compiled as part of `make all` due to build time.

```bash
# Download and build the kernel (WSL2)
cd /tmp
wget https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.6.tar.xz
tar xf linux-6.6.tar.xz
cd linux-6.6

make ARCH=x86_64 defconfig
scripts/kconfig/merge_config.sh .config /path/to/MahinaOS/kernel/.config
make ARCH=x86_64 olddefconfig
make ARCH=x86_64 -j$(nproc) bzImage

cp arch/x86_64/boot/bzImage /path/to/MahinaOS/build/vmlinuz-mahina
```

---

## Directory Structure

```
MahinaOS/
├── boot/
│   └── limine.cfg          # Bootloader configuration (limine, DL-005)
├── kernel/
│   ├── .config             # Kernel config fragment (merge with defconfig)
│   └── .config.notes       # Per-option justification (Law I)
├── src/
│   ├── luna-init/          # PID 1 init system (C17, statically linked)
│   └── luna-init-ctl/      # Control CLI
├── etc/
│   └── luna/
│       ├── hostname        # System hostname
│       ├── fstab.toml      # Filesystem mount table
│       ├── modules.conf    # Kernel modules to load at boot
│       ├── sysctl.toml     # Kernel parameter overrides
│       └── services/       # luna-init service files (TOML)
├── tests/
│   ├── vendor/unity/       # Unity testing framework (vendored)
│   ├── unit/               # Unit tests (C)
│   └── fuzz/               # Fuzz targets (AFL++)
├── scripts/
│   ├── build-initramfs.sh  # initramfs builder
│   ├── build-image.sh      # Disk image builder
│   └── run-qemu.sh         # QEMU launcher
├── docs/
│   └── DCKL/               # Architecture documentation (7 volumes)
├── Makefile
└── README.md
```

---

## Architecture

Mahina boots in 7 stages. Stage 0 (v0.1) implements stages 0–4:

```
[Stage 0] UEFI → limine → kernel loaded
[Stage 1] Kernel init → luna-init alive as PID 1
[Stage 2] Filesystems mounted (/proc /sys /dev /tmp /run)
[Stage 3] Early hooks (hostname, clock, entropy)
[Stage 4] System services (DEGRADED in v0.1 — binaries not yet present)
[Stage 5] Graphics layer            ← v0.5
[Stage 6] Shell + LUNA Presence     ← v0.5
[Stage 7] Desktop ready             ← v0.5
```

Full architecture specification: [docs/DCKL/Volume II - Architecture/02_boot_flow.md](docs/DCKL/Volume%20II%20-%20Architecture/02_boot_flow.md)

---

## Architecture Decisions

All design decisions are recorded in the Decision Log:
[docs/DCKL/Volume I - Foundation/decision_log.md](docs/DCKL/Volume%20I%20-%20Foundation/decision_log.md)

Key decisions for Stage 0:
- `DL-002` — luna-init is written in C, TOML service files
- `DL-005` — limine bootloader
- `DL-007` — glibc (v1), musl migration planned for v2
- `DL-008` — TOML as universal config format
- `DL-009` — Linux 6.6.x LTS
- `DL-027` — Btrfs root filesystem

---

## Contributing

Read the [DCKL](docs/DCKL/) before contributing. Every architectural decision is documented there.

**The documentation wins.** If code conflicts with documentation, fix the code.

---

*Author: Hardik Bhaskar (Luna Kitsune)*
*License: See LICENSE*
