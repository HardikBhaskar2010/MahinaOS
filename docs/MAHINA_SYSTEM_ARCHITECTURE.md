# MahinaOS — Canonical System Architecture Specification

> **Status**: Authoritative Architectural Baseline — Milestone M0 (Phase 0)  
> **Document Revision**: 1.0.0 (2026-09-08)  
> **Repository**: [MahinaOS](https://github.com/HardikBhaskar2010/MahinaOS)  
> **Target Kernel**: Linux 6.6 LTS (x86_64)  
> **Target Bootloader**: Limine (UEFI x86_64)

---

## 1. The North Star

MahinaOS is **not** defined as:

> *"Mahina has a custom kernel."*

MahinaOS is strictly defined as:

> **MahinaOS is a stable, usable, Linux-based operating system platform where the User owns the machine and LunaAI is a permissioned system-level partner.**

This definition organizes all engineering work into three distinct, non-negotiable layers:

1. **FOUNDATION: Make the machine trustworthy.**  
   Engineering integrity, truthful unit and integration testing, deterministic builds, pinned toolchains, fail-closed security primitives, and a clean UEFI $\to$ Limine $\to$ Linux 6.6 $\to$ `luna-init` boot pipeline.
2. **PLATFORM: Make Mahina a real operating-system environment.**  
   A dedicated control plane, generational updates with guaranteed rollback, explicit multi-user identity and capability boundaries, driver and hardware usability (DRM/KMS, libinput), software distribution via signed `lpkg` packages, and an agency-grade, visually captivating human-interface design system.
3. **INTELLIGENCE: Make LunaAI a first-class participant in that environment.**  
   A local background inference daemon (`luna-ai-d`) operating under fine-grained, kernel-enforced and IPC-mediated capabilities (`files.read`, `apps.launch`, `packages.install`), with self-modification (Engineering Mode) arriving strictly after rollback and generational recovery are proven.

```
+-------------------------------------------------------------------------+
|                              INTELLIGENCE                               |
|   LunaAI System Partner  *  Capability Permissions  *  Engineering Mode  |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                                PLATFORM                                 |
|   Control Plane  *  Generational Recovery  *  LGP Desktop  *  lpkg      |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                               FOUNDATION                                |
|   UEFI 2.x  *  Limine  *  Linux 6.6 LTS  *  luna-init (PID 1)  *  musl  |
+-------------------------------------------------------------------------+
```

---

## 2. Core Architectural Invariants

Every engineer, agent, and autonomous subsystem modifying MahinaOS must uphold these rules without exception:

1. **Strict Sequencing of Dependencies**:  
   Never build features ahead of the platform abstraction they depend on:
   - No autonomous LunaAI actions before capability permissions and audit trails exist.
   - No self-modification (Engineering Mode) before reproducible generation building and automated boot rollback exist.
   - No application ecosystem before package management (`lpkg`) and system control APIs stabilize.
   - No visual styling ornaments ahead of protocol stability and robust input/compositor foundations.
2. **Fail-Closed Security by Default**:  
   If a cryptographic signature cannot be verified (e.g. missing `/etc/luna/lpkg.pub`), if an IPC message lacks an authorization token, or if a capability check is ambiguous, the system must immediately and safely abort the operation with a descriptive diagnostic.
3. **No NSS in Early Static Binaries**:  
   Binaries that run as PID 1 (`luna-init`) or in early boot (`luna-splash`) must be statically linked against musl libc. They must **never** link or call glibc Name Service Switch (NSS) functions (`getpwnam`, `getgrnam`, `getaddrinfo`, etc.), as dynamic shared-library lookups in static contexts cause segmentation faults or break in minimal chroots. `/etc/passwd` and `/etc/group` are parsed directly through linear scan routines (`find_uid_in_file`, `lookup_video_gid`).
4. **Clean-Slate Display Stack**:  
   MahinaOS does **not** run X11, Wayland, Xwayland, GNOME, or KDE. Display management is implemented directly on Linux Direct Rendering Manager / Kernel Mode Setting (`/dev/dri/card0`, DRM/KMS dumb buffers) through the custom Luna Graphics Protocol (`LGP`) compositor.
5. **Generational Immutability**:  
   System updates never overwrite running files in place. New system states are staged as discrete read-only Btrfs subvolume generations. Boot validation must succeed before a generation becomes the permanent default.

---

## 3. Full Vertical Stack Diagram

The MahinaOS system stack is partitioned into 7 vertical tiers, tracing execution from bare metal or virtualized UEFI firmware up to ambient intelligence:

```
+-----------------------------------------------------------------------+
| 0. HARDWARE                                                           |
|    x86_64 CPU (SSE4.2+, AVX2 recommended), ACPI, UEFI 2.4+ firmware,   |
|    Storage (NVMe / SATA / virtio-blk), Display (DRM/KMS Intel/AMD/virtio)|
+-----------------------------------------------------------------------+
                                  |
                                  v
+-----------------------------------------------------------------------+
| 1. FIRMWARE                                                           |
|    UEFI 2.x compliant firmware (EDK2 / OVMF for VMs, vendor firmware)  |
|    Loads EFI system partition (ESP) FAT32 at /boot/efi                 |
+-----------------------------------------------------------------------+
                                  |
                                  v
+-----------------------------------------------------------------------+
| 2. BOOTLOADER                                                         |
|    Limine Bootloader (UEFI x86_64, Limine Protocol)                   |
|    Configuration: /boot/limine.conf                                   |
|    Loads: /boot/vmlinuz-6.6-mahina and /boot/initramfs-mahina.img      |
+-----------------------------------------------------------------------+
                                  |
                                  v
+-----------------------------------------------------------------------+
| 3. LINUX KERNEL                                                       |
|    Linux 6.6 LTS (Minimal Mahina .config)                             |
|    Built-in: Btrfs, ext4, devtmpfs, cgroups v2, eBPF, DRM/KMS, evdev  |
|    Command line: init=/init console=tty1 quiet loglevel=3             |
+-----------------------------------------------------------------------+
                                  |
                                  v
+-----------------------------------------------------------------------+
| 4. MAHINA CORE                                                        |
|    ├── luna-init (PID 1, static musl, process supervisor, DAG manager) |
|    ├── luna-init-ctl (UNIX socket /run/luna-init.sock client)         |
|    ├── luna-splash (direct /dev/fb0 or DRM splash animation)          |
|    ├── storage & mounts (Btrfs subvolumes: @, @home, @var, @generations)|
|    ├── identity & permissions (/etc/passwd, /etc/group direct parser) |
|    ├── package manager (lpkg: Ed25519 signed tar.gz, index.toml)      |
|    └── control plane (/run/mahina/control.sock daemon router)         |
+-----------------------------------------------------------------------+
                                  |
                                  v
+-----------------------------------------------------------------------+
| 5. MAHINA EXPERIENCE (Visual Platform & Desktop)                      |
|    ├── lgp-compositor (DRM/KMS dumb buffers, double-buffering, LGP IPC)|
|    ├── lgp-rs (safe Rust client protocol bindings for LGP)            |
|    ├── lunagui-rs (retained-mode 2D GUI toolkit, canvas primitives)   |
|    ├── luna-shell-rs (desktop shell, task switcher, status tray)      |
|    ├── luna-island-rs (dynamic status island HUD, modal notifier)     |
|    ├── luna-terminal (hardware-accelerated VT100/ANSI terminal)       |
|    └── luna-settings-rs & core utility applications                   |
+-----------------------------------------------------------------------+
                                  |
                                  v
+-----------------------------------------------------------------------+
| 6. LUNAAI (Partner Intelligence)                                      |
|    ├── luna-ai-d (local inference daemon, llama.cpp / REST backends)   |
|    ├── Capability API (scoped token: files.read, apps.launch, etc.)    |
|    ├── System Awareness Bus (read-only telemetry & context tracker)   |
|    └── Engineering Mode (candidate generation patcher & test runner)  |
+-----------------------------------------------------------------------+
```

---

## 4. Subsystem Classification Matrix

Every subsystem across the MahinaOS repository is explicitly categorized under one of four classifications:

- **`[Mahina-owned]`**: Built in this repository; MahinaOS maintainers hold 100% architectural authority.
- **`[Linux-provided]`**: Standard Linux kernel or upstream userland capability leveraged intentionally.
- **`[External dependency]`**: Deliberately selected third-party library or tool pinned to a specific version.
- **`[Future]`**: Planned subsystem with defined boundaries, currently in specification or staging.

| Subsystem / Component | Classification | Language / Target | Boundary & Interface | Current State & Invariant |
|---|---|---|---|---|
| **`luna-init`** | `[Mahina-owned]` | C17 (musl static) | PID 1; `/run/luna-init.sock`; DAG service engine | Fully working; zero glibc NSS calls; reaps zombies; manages service lifecycles. |
| **`luna-init-ctl`** | `[Mahina-owned]` | C17 (musl static) | CLI tool sending structured IPC packets to `luna-init` | Fully working; controls start/stop/status/reload of daemons. |
| **`luna-splash`** | `[Mahina-owned]` | C17 (musl static) | Framebuffer (`/dev/fb0`), DRM/KMS dumb buffer fallback | Fully working; zero dynamic allocations in render loop; 30fps animation. |
| **`lgp-compositor`** | `[Mahina-owned]` | C17 (C11/POSIX) | `/run/lgp.sock` (LGP wire protocol); DRM/KMS dumb buffers | Working; handles surface tree, window z-order, mouse/keyboard routing via evdev. |
| **`lgp-rs`** | `[Mahina-owned]` | Rust 2021 | Rust safe crate wrapping LGP wire protocol | Protocol client crate; serialization, socket I/O, shared memory shm buffers. |
| **`lunagui-rs`** | `[Mahina-owned]` | Rust 2021 | Rust 2D retained-mode UI toolkit over `lgp-rs` | Retained scene graph; event dispatch; double-bezel card rendering; anti-aliasing. |
| **`luna-shell-rs`** | `[Mahina-owned]` | Rust 2021 | Desktop shell, taskbar, dock, application launcher | Renders window list, clock, status tray, app menu; talks to compositor. |
| **`luna-island-rs`** | `[Mahina-owned]` | Rust 2021 | Floating status HUD ("Dynamic Island") | Renders ambient system state, AI thoughts, volume/brightness/battery pills. |
| **`luna-setup-rs`** | `[Mahina-owned]` | Rust 2021 | Interactive disk installer & first-boot provisioning | Partitioning, Btrfs subvolume layout, base image extraction, Limine boot install. |
| **`lpkg`** | `[Mahina-owned]` | C17 / Rust | Package archive format (`.lpkg`), index parser, crypto | Ed25519 signature verification; fails closed if `/etc/luna/lpkg.pub` missing. |
| **`luna-ai-d`** | `[Mahina-owned]` | Rust 2021 (Tokio) | Local intelligence daemon; `/run/mahina/ai.sock` | Context monitor, model backend client, capability token validator. |
| **Mahina Control Plane** | `[Future]` | Rust 2021 | Unified system daemon; `/run/mahina/control.sock` | Centralized broker for configuration, hardware events, permissions, updates. |
| **Generation Manager** | `[Future]` | Rust 2021 | Btrfs snapshot manager; `/boot/limine.conf` updater | Manages candidate generations, automatic watchdog reboot, and instant rollback. |
| **Engineering Mode** | `[Future]` | Rust 2021 | Sandboxed self-modification engine | Clones repo into scratch generation, compiles, executes test gates, promotes on green. |
| **Linux Kernel** | `[Linux-provided]` | C | Syscalls, DRM/KMS, evdev, cgroups v2, Btrfs, eBPF | Pinned to 6.6 LTS; minimal config without legacy bloat (no systemd, no pulseaudio). |
| **DRM / KMS Subsystem**| `[Linux-provided]` | Kernel C / ioctl | `/dev/dri/card0`, dumb buffer allocation, page flipping | VSync synchronization, zero tearing, direct hardware scanning. |
| **evdev / udev** | `[Linux-provided]` | Kernel C / `/dev` | `/dev/input/event*`, keyboard, pointer, touch | Raw input streams routed to `libinput` or processed directly in compositor. |
| **Btrfs Filesystem** | `[Linux-provided]` | Kernel C | Subvolumes, copy-on-write (CoW), snapshots | Atomic generational state management; `@` (system), `@home` (user), `@generations`. |
| **Limine Bootloader** | `[External dependency]` | C / Assembly | UEFI PE/COFF, Limine boot protocol | Simple, fast, modern bootloader supporting direct kernel loading and snapshots. |
| **libsodium** | `[External dependency]` | C | Cryptographic primitives (Ed25519 signatures) | Pinned dependency used by `lpkg` for package verification and token signing. |
| **libinput / libudev** | `[External dependency]` | C | Pointer acceleration, gesture recognition, hotplug | Provides smooth touchpad and mouse translation for `lgp-compositor`. |
| **musl libc** | `[External dependency]` | C | Standard C library for static targets | Clean static linking for `luna-init` and `luna-splash` without NSS bloat. |
| **stb_image** | `[External dependency]` | C (header-only) | Vendored image decoder (`src/luna-splash/stb_image.h`)| Isolated under `VENDOR_CFLAGS` to eliminate external compiler warnings. |
| **Unity** | `[External dependency]` | C (test framework) | Vendored C test framework (`tests/unit/unity/`) | Isolated under `VENDOR_CFLAGS`; runs 14/14 green unit tests in `make test-unit`. |

---

## 5. Visual Design System Architecture

MahinaOS rejects the generic, flat, templated aesthetic of conventional Linux distributions and boilerplate AI dashboards. Guided by installed design skills (`frontend-design`, `high-end-visual-design`, `minimalist-ui`, `ui-ux-pro-max`), the Mahina Experience is built upon a **hardware-machined, utilitarian editorial aesthetic**.

### 5.1 Design Philosophy: Utilitarian Luxury

1. **Precision Machining Over Blurs**:  
   Rather than overwhelming the screen with cheap, blurry glassmorphism, Mahina uses crisp geometry, razor-sharp 1px border lines, and micro-hairlines. Surfaces feel cut from physical anodized aluminum and deep obsidian glass.
2. **Concentric Curvature ("Doppelrand")**:  
   Nested surfaces (such as a button inside a card, or a card inside a window) must never share identical corner radii. Inner radius is mathematically derived from outer radius and padding:  
   $$R_{\text{inner}} = \max(0, R_{\text{outer}} - \text{padding})$$  
   This creates continuous, harmonious optical curvature across all UI surfaces.
3. **Restraint Over Decoration**:  
   No arbitrary gradients, drop shadows, or floating glows. Visual hierarchy is established strictly through typographic scale, weight contrast, and subtle surface luminance.

### 5.2 Color Semantic Tokens

The visual foundation is anchored in deep OLED blacks, subtle translucent grays, and muted, non-fatiguing semantic pastels:

```
/* Base Surfaces */
--surface-void:        #0A0A0F;  /* Deepest background, OLED power-off black */
--surface-canvas:      #121218;  /* Root workspace canvas */
--surface-card:        #1A1A24;  /* Window cards, elevated tiles */
--surface-overlay:     #242432;  /* Menus, dialogs, popovers */
--surface-border:      rgba(255, 255, 255, 0.08); /* Precision 1px outline */
--surface-hairline:    rgba(255, 255, 255, 0.04); /* Sub-element divider */

/* Typography & Contrast */
--text-primary:        #FFFFFF;  /* Headers, active window titles, emphasis */
--text-secondary:      #9E9EA8;  /* Body text, inactive titles, tooltips */
--text-muted:          #5E5E6C;  /* Breadcrumbs, disabled states, key hints */

/* Semantic State Accents (Muted Pastels) */
--state-success:       #5BB98C;  /* Muted Emerald: Ready, Verified, Clean */
--state-warning:       #E5A440;  /* Muted Amber: Rebuilding, Pending auth */
--state-error:         #E55C5C;  /* Muted Ruby: Refused, Rollback triggered */
--state-accent:        #7E8CE0;  /* Muted Periwinkle: User active focus */
--state-ai:            #A57CE0;  /* Muted Violet: LunaAI ambient presence */
```

### 5.3 Typographic Scale & Rhythm

- **Display & Headlines**: Clean, high-legibility neo-grotesque sans-serif (Inter / System Sans).
- **Code & Diagnostics**: Monospaced tabular figures with clear differentiation between `0`, `O`, `1`, `l`, `I` (JetBrains Mono / Fira Code).
- **Scale Hierarchy**:
  - `Display`: 28px / 34px line-height (bold 700)
  - `Title`: 18px / 24px line-height (semi-bold 600)
  - `Body`: 13px / 18px line-height (regular 400)
  - `Caption`: 11px / 14px line-height (medium 500)
  - `Code`: 12px / 16px line-height (mono 400)

### 5.4 Kinetic Motion & Physics

All UI animations in `lunagui-rs` and `luna-shell-rs` follow deterministic kinetic motion curves:
- **Default Easing**: Fast-out, slow-settle easing curve:  
  `cubic-bezier(0.16, 1, 0.3, 1)`
- **Durations**:
  - Micro-interactions (hover, click, state toggle): `120ms`
  - Window open / modal reveal / drawer slide: `240ms`
  - Dynamic Island morphing / state transitions: `320ms`
- **Page Flipping**: Directly synchronized to the display's vertical blanking interval (`DRM_EVENT_FLIP_COMPLETE`) to guarantee 0% tearing and 60fps/120fps fluid responsiveness.

---

## 6. Subsystem Boundaries and Interfaces

### 6.1 `luna-init` (PID 1 Service Supervisor)
- **Role**: Replaces `systemd` and SysV init with a minimal, audit-friendly, static C17 process supervisor.
- **Responsibilities**:
  1. Mounts early virtual filesystems (`/dev`, `/proc`, `/sys`, `/run`).
  2. Parses `/etc/luna/services.d/` dependency DAG.
  3. Launches daemons in parallel topological order.
  4. Collects and reaps orphaned child processes (`waitpid(-1, &st, WNOHANG)`).
  5. Listens on UNIX socket `/run/luna-init.sock` for client management commands.
- **Invariant**: Must never call NSS functions (`getpwnam`, `getgrnam`, `getaddrinfo`). User lookups are done via `find_uid_in_file("/etc/passwd", ...)` and `inet_pton` for address bindings.

### 6.2 `lgp-compositor` (Display Server)
- **Role**: Native display server communicating directly with Linux DRM/KMS.
- **Responsibilities**:
  1. Opens `/dev/dri/card0` and performs atomic/dumb-buffer mode setting.
  2. Allocates front and back dumb buffers for seamless double-buffering.
  3. Listens on UNIX socket `/run/lgp.sock` for client window connections.
  4. Collects input events from `/dev/input/event*` and translates them via `libinput`.
  5. Composites surfaces in z-order onto the back buffer and submits page flips.
- **Wire Protocol (`LGP`)**:
  - Packet format: 8-byte header (`uint16_t msg_type`, `uint16_t client_id`, `uint32_t payload_len`) followed by payload.
  - Opcodes: `CREATE_SURFACE`, `DESTROY_SURFACE`, `COMMIT_BUFFER`, `MAP_SURFACE`, `INPUT_EVENT`, `DAMAGE_REGION`.

### 6.3 `lpkg` (Package Distribution Engine)
- **Role**: Cryptographically verifiable software packaging and dependency resolver.
- **Responsibilities**:
  1. Bundles package files into tarballs with metadata manifest (`index.toml`).
  2. Signs packages using Ed25519 private keys.
  3. Verifies package signatures against trusted system keys stored in `/etc/luna/lpkg.pub`.
  4. Installs binaries to `/usr/bin` and `/usr/share` with file manifests for clean removal.
- **Invariant**: **Fail Closed**. If `/etc/luna/lpkg.pub` is missing, corrupted, or does not match the package signature, `lpkg` refuses installation immediately with code 1.

### 6.4 `luna-ai-d` (Partner Intelligence Daemon)
- **Role**: Background service facilitating AI assistance, context awareness, and intent routing.
- **Responsibilities**:
  1. Listens on `/run/mahina/ai.sock` for user prompts and shell integration.
  2. Operates as an unprivileged service user (`luna-ai:luna-ai`).
  3. Queries system telemetry via read-only endpoints on the Mahina Control Plane.
  4. Requests elevated actions exclusively via the Capability API.
  5. Renders status to `luna-island-rs` via IPC notifications.
- **Invariant**: Zero direct root or filesystem write execution outside designated sandboxes. Every privileged command requires an authorized Capability Token.

---

## 7. Capability Milestones (M0 — M7)

The implementation roadmap is partitioned into eight milestone gates:

```
[M0: Baseline] --> [M1: Bootable] --> [M2: Usable] --> [M3: Platform]
                                                            |
[M7: Self-Evolving] <-- [M6: Engineering] <-- [M5: LunaAI] <-- [M4: Freedom]
```

- **M0: Engineering Baseline (CURRENT)**  
  Clean repo, zero `-Werror`, zero Clippy warnings, truthful test harness (14/14 green tests), hardened CI Gate 1, pinned Rust toolchain, and frozen system architecture documentation.
- **M1: Bootable Mahina**  
  Security model specification, boot architecture specification, automated initramfs builder, Limine ISO generation, and headless QEMU boot test to PID 1 shell.
- **M2: Usable Mahina**  
  Interactive `luna-setup-rs` installer, multi-user identity (`/etc/passwd`, `/etc/group`), input drivers, double-bezel window desktop (`luna-shell-rs`), and terminal emulator.
- **M3: Mahina Platform**  
  Unified Mahina Control Plane daemon (`/run/mahina/control.sock`), Btrfs subvolume layout (`@`, `@home`, `@generations`), and tested generation rollback.
- **M4: Software Freedom**  
  Standard Linux POSIX compatibility, complete `lpkg` package manager distribution lifecycle, and userland developer tools.
- **M5: LunaAI System Partner**  
  Local `luna-ai-d` daemon, Capability Token validation, Dynamic Island visual HUD (`luna-island-rs`), and prompt-to-action intent routing.
- **M6: Engineering Mode**  
  Sandboxed self-modification: user-approved changes compile into a candidate generation, pass test suites, and deploy with automatic bootloader fallback on failure.
- **M7: Self-Evolving Mahina**  
  Progressive expansion of self-modification authority under explicit user control and complete cryptographic verification.

---

## 8. Skills Catalog & Operational Guidelines

To preserve code quality, avoid regressions, and adhere to industry-standard patterns, the following installed skills are required for all implementation tasks:

### 8.1 Systems Engineering Skills (`.agents/skills/`)
- **`rust-engineer`**: Enforce toolchain pinning (`rust-toolchain.toml`), workspace dependency consistency, and safe C-to-Rust FFI boundaries.
- **`rust-async-patterns`**: Architect asynchronous Tokio runtimes, non-blocking UNIX socket loops, streaming IPC decoders, and cancellation handling for `luna-ai-d` and `luna-shell-rs`.
- **`rust-best-practices`**: Enforce Apollo GraphQL Rust standards: borrow instead of clone, zero unwraps/panics in production code, type-state pattern for window lifecycles, and `Result<T, E>` error propagation.
- **`rust-testing`**: Drive Test-Driven Development (TDD) for all new components; maintain high test coverage, property-based testing, and regression suites.
- **`rust-patterns`**: Make invalid system states unrepresentable via algebraic data types; use `thiserror` for library crates and `anyhow` for top-level binaries.

### 8.2 UI/UX Design Skills (`.agents/skills/`)
- **`frontend-design`**: Guide aesthetic choices away from generic defaults; design distinctive typography, spacing rhythm, and purposeful component hierarchy.
- **`high-end-visual-design`**: Implement hardware-machined double-bezel curvature (`Doppelrand`), kinetic micro-interactions, and tailored cubic-bezier animation curves.
- **`minimalist-ui`**: Enforce utilitarian editorial minimalism, monochrome surfaces, high-contrast text, razor-thin borders, and zero clutter.
- **`ui-ux-pro-max`**: Reference 119 UX guidelines, 192 color palettes, and accessibility metrics (WCAG 4.5:1) for all interactive components.

---

## 9. Conclusion

This architecture document is the **single source of truth** for MahinaOS development. No feature, service, or tool may violate the sequencing, security invariants, or design standards set forth herein.
