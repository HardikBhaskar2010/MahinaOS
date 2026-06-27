# Mahina — Hardware Compatibility Matrix
**Volume VII · Chapter 3**
**Classification:** Project Management — Hardware Engineering
**Status:** Canonical · Updated as hardware is tested

---

## Purpose

This document defines the hardware requirements for Mahina v1.0 and tracks compatibility with specific hardware configurations. It is the reference for what hardware is officially supported, what is known to work, and what is known not to work.

---

## Hardware Architecture

**Mahina v1.0 targets x86-64 (AMD64) only.**

```
Supported architectures in v1.0:
  x86-64 (AMD64)    ✅ Fully supported and tested

Not supported in v1.0 (future roadmap):
  ARM64 (AArch64)   ✗ v2.0 investigation item
  RISC-V            ✗ Not planned
  x86 (32-bit)      ✗ Will not be supported
```

---

## Minimum Hardware Requirements

These are the absolute minimum specifications for Mahina to function. Performance at minimum spec is acceptable but not ideal.

```
CPU:
  Architecture:  x86-64
  Minimum:       Dual-core, any clock speed
  Recommended:   Quad-core, 2.5 GHz+
  Required:      SSE4.2 support
  Recommended:   AVX2 support (significantly improves LLM performance)

RAM:
  Minimum (no AI):   4 GB
  Minimum (with AI): 8 GB   (OS 4 GB + Qwen2.5 3B 2.0 GB + 2 GB buffer)
  Recommended:       16 GB  (comfortable with LLM + open applications)

Storage:
  Minimum:     20 GB free space
               (OS ~1.4 GB + AI model ~1.7 GB + package cache + user data)
  Recommended: SSD (SATA or NVMe)
  Notes:       HDD supported but AI model load time is significantly slower
               (~15-20 seconds first token on HDD vs ~3-5 seconds on SSD)

Display:
  Minimum:     1024×768 (functional, some UI elements cramped)
  Recommended: 1920×1080 or higher
  DPI:         1x (96 DPI) fully supported
               HiDPI: scaling framework present but not fully validated in v1.0

Graphics:
  Minimum:     Any GPU with DRM/KMS kernel driver
  Software renderer: CPU rendering via dumb framebuffer (Stage 2 fallback)
  Recommended: Intel HD/UHD Graphics (620+), AMD RX 5000+, NVIDIA (see notes)
  GPU RAM:     Not required. 4 GB+ VRAM enables full GPU LLM acceleration.

Network:
  Wired:   Any NIC with Linux kernel driver
  Wi-Fi:   Intel Wi-Fi cards (AX200, AX210) — best supported
           Realtek RTL8821CE, RTL8822CE — supported, may require firmware
           Broadcom: limited support, firmware often required

Firmware:
  Boot:   UEFI 2.0+
  Secure Boot: Not supported in v1.0 (Secure Boot must be disabled)
```

---

## GPU Support Matrix

```
GPU Family          Driver      Status        Notes
---------------------------------------------------------------------------
Intel HD/UHD 620+   i915        ✅ Supported  Reference hardware for v1
Intel Arc (A-series) i915/Xe   🔵 Beta       Xe driver in testing
AMD RX 5000+ (RDNA)  amdgpu    ✅ Supported  GPU LLM acceleration works
AMD Vega / Polaris   amdgpu    ✅ Supported  Older, slower LLM acceleration
AMD RX 6000/7000     amdgpu    ✅ Supported
NVIDIA (any)         nouveau   🟡 Limited    nouveau driver only in v1.0
                               ⚠ Warning    Proprietary driver not in v1.0
                                             LLM GPU acceleration: no
NVIDIA (any)         n/a       ✗ No GPU accel proprietary required for CUDA
VMware (SVGA3D)      vmwgfx    ✅ Supported  Used for VMware testing
VirtualBox (VGA)     vboxvideo ✅ Supported  Software renderer
QEMU (virtio-gpu)    virtio    ✅ Supported  CI test environment
QEMU (VGA / stdvga)  bochs     ✅ Supported  Basic QEMU mode
```

**Note on NVIDIA:** NVIDIA's proprietary driver is not included in Mahina v1.0 for licensing reasons. Users with NVIDIA GPUs can run Mahina using the nouveau open-source driver, but LLM GPU acceleration will not be available. NVIDIA GPU acceleration is a v1.5 investigation item.

---

## CPU Feature Requirements

| Feature | Required | Notes |
|---|---|---|
| x86-64 baseline | ✅ Mandatory | Must support |
| SSE4.2 | ✅ Mandatory | Used by llama.cpp (LLM inference) |
| AVX | Strongly recommended | 2x LLM inference improvement |
| AVX2 | Strongly recommended | Primary llama.cpp optimization path |
| AVX-512 | Optional | Additional improvement on Intel Xeon/Core i9 |
| AES-NI | Recommended | Memory encryption (DL-023) |
| UEFI boot | ✅ Mandatory | Legacy BIOS not supported in v1.0 |
| Secure Boot | ✗ Must be disabled | Not supported in v1.0 |

---

## Tested Hardware Configurations

This section tracks physical hardware that has been tested with Mahina.

### Reference Hardware (CI / Development)

| Machine | CPU | RAM | GPU | Status |
|---|---|---|---|---|
| QEMU 8.x (standard config) | virtual x86-64 | 8 GB virtual | virtio-gpu | ✅ CI passes |
| QEMU 8.x (legacy config) | virtual x86-64 | 4 GB virtual | VGA (bochs) | ✅ Tested |

### Developer-Tested Hardware

*This section is populated as community members report test results. Format:*

| Machine | CPU | RAM | GPU | Boot | Display | Input | Network | AI | Status |
|---|---|---|---|---|---|---|---|---|---|
| *(your machine)* | *(CPU)* | *(GB)* | *(GPU)* | — | — | — | — | — | Report needed |

**To add your machine:** Boot Mahina, run through the Hardware Test checklist below, and submit the results.

---

## Hardware Test Checklist

When testing a new physical machine, verify each item:

```
Boot:
  [ ] UEFI detects the Mahina ISO/USB
  [ ] Limine bootloader screen appears
  [ ] Kernel boots without kernel panic
  [ ] luna-init reaches the login screen

Display:
  [ ] Correct resolution detected automatically
  [ ] Compositor runs at 60 FPS (check luna-perf indicator)
  [ ] Text is legible at the native resolution

Input:
  [ ] USB keyboard works (all keys including modifier keys)
  [ ] USB mouse works (cursor tracks correctly)
  [ ] Laptop trackpad works (basic point and click)
  [ ] Trackpad multi-touch (two-finger scroll): note result

Network:
  [ ] Wired Ethernet: DHCP assigns IP address
  [ ] Wi-Fi (if present): network list appears in Settings
  [ ] Connectivity: can access the internet

Audio:
  [ ] Audio output: test tone plays without crackling
  [ ] Volume control works
  [ ] Audio input (microphone): confirm presence (not required for v1)

AI (if 8+ GB RAM):
  [ ] luna-ai-d starts (check luna-island color: green or amber)
  [ ] If green: basic chat query returns a response
  [ ] If amber: 'luna model install qwen2.5:3b' starts correctly

Battery (laptops only):
  [ ] Battery percentage reported correctly
  [ ] Charging state reported correctly
  [ ] LUNA aware of battery level
```

---

## Known Hardware Incompatibilities

| Hardware | Issue | Workaround | Target Fix |
|---|---|---|---|
| NVIDIA GPU | No proprietary driver → no GPU LLM acceleration | Use CPU inference (slower) | v1.5 |
| Broadcom Wi-Fi | Firmware often not included | Manual firmware install via lpkg | v1.5 |
| UEFI Secure Boot | Not supported | Disable Secure Boot in UEFI settings | v2.0 |
| Legacy BIOS systems | Limine requires UEFI | No workaround in v1.0 | v1.5 |
| Hi-DPI displays > 2x | Scaling not validated | Set display to 1x scaling | v1.5 |

---

## Virtual Machine Support

```
QEMU / KVM:
  Config:  CPU=host (or kvm64), RAM=8GB+, Display=virtio-gpu or VGA
  Status:  ✅ Fully supported — primary development environment

VirtualBox:
  Config:  RAM=8GB+, Display=VirtualBox VGA or VMSVGA
  Status:  ✅ Supported
  Notes:   Use EFI mode. VirtualBox Guest Additions not supported.

VMware Workstation / Fusion:
  Config:  RAM=8GB+, Display=SVGA3D
  Status:  ✅ Supported
  Notes:   vmwgfx driver required, included in kernel config

Hyper-V (Windows):
  Status:  🔵 Untested — investigation item for v1.5

Parallels Desktop (macOS):
  Status:  🔵 Untested — ARM translation layer unknown compatibility
```

---

## AI Performance by Hardware Tier

Performance of the default Qwen2.5 3B model at each hardware tier:

```
Tier          CPU                 RAM   GPU            First Token  Tok/s
───────────────────────────────────────────────────────────────────────────
Minimum       i5-8250U (4c)      8 GB  None (CPU)     3–4 sec      2–3
Recommended   i7-11800H (8c)    16 GB  None (CPU)     2–3 sec      4–6
GPU (iGPU)    Ryzen 7 5800U     16 GB  Radeon (iGPU)  2 sec        5–8
GPU (dGPU)    Ryzen 5 5600X     16 GB  RX 6700 XT     1 sec        15–25
QEMU (CI)     virtual           8 GB   virtio-gpu     4–6 sec      1–2
```

These numbers are estimates. Actual performance depends on CPU cache, memory bandwidth, and thermal conditions.

---

*Document: `Volume VII / 03_hardware_compatibility_matrix.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 1.0*
*Depends on: DL-046 (Qwen2.5 3B resource requirements), Volume VI/04_benchmarks.md*
*Informs: Volume V/06_installer.md (minimum requirements), marketing and release notes*
