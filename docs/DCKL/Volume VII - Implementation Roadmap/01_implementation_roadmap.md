# Mahina — Implementation Roadmap
**Volume VII · Chapter 1**
**Classification:** Project Management — Living Document
**Status:** Canonical · Updated with every completed milestone

---

## What Mahina v1.0 Is

> **Mahina v1.0 is a complete, installable, bootable operating system that establishes the foundation.**
>
> Not "The AI OS." Not "The Living Interface." Not "The OS that understands you."
>
> Just a solid, usable operating system built on principled engineering — with a local AI presence that is intentionally small and honest about what it is in version 1.

This document is the daily engineering reference. Pick a task from the current version band. Complete it. Mark it done. Move to the next.

If you are writing code not listed here, you are either ahead of schedule or off course.

---

## Version Map

```
v0.1  Bring-up         Kernel boots. luna-init. Console. Logger.
v0.5  Developer Preview GUI. Compositor. Shell. Terminal. lpkg. Networking.
v0.9  Feature Complete  Installer. Core apps. Stable desktop. LUNA runtime (basic).
v1.0  Public Release    Stable. Installable. Daily-usable. Documentation synchronized.

v1.5  (future)          Better desktop experience. Polish. Performance.
v2.0  (future)          The Living OS — LUNA becomes a defining part of the experience.
```

---

## v1.0 Scope Commitment

### What Ships in v1.0

```
Foundation:
  ✅ Boots on real x86_64 UEFI hardware
  ✅ Boots in a VM (QEMU, VirtualBox, VMware)
  ✅ Custom boot branding (Mahina)
  ✅ luna-init as PID 1
  ✅ Logging system
  ✅ Basic service manager
  ✅ Filesystem mounting (Btrfs, DL-027)
  ✅ User login / session
  ✅ Basic shell

Package Management:
  ✅ lpkg (package manager with Ed25519 signing, DL-048)

Graphics:
  ✅ lgp-compositor
  ✅ LunaGUI framework
  ✅ Luna Dock
  ✅ Luna Island (basic version — static, no personality animations)
  ✅ Dark theme (Luna Dark)

Applications:
  ✅ Settings application
  ✅ File manager
  ✅ Terminal
  ✅ Window management

System:
  ✅ Basic networking
  ✅ Audio (PipeWire)
  ✅ Mouse & keyboard (libinput)
  ✅ Installer (luna-installer)

LUNA (intentionally small):
  ✅ Local AI runtime (luna-ai-d, Python, DL-049)
  ✅ First-run model installation (Qwen2.5 3B, DL-046)
  ✅ Basic chat interface
  ✅ System awareness (time, battery, CPU, memory)
  ✅ Permission system
  ✅ AI Degraded Mode if no model installed (DL-047)
```

### What Waits for v2.0

```
  ✗ Full personality engine
  ✗ Advanced context engine
  ✗ Vision model
  ✗ Voice (TTS/STT)
  ✗ Automation / proactive actions
  ✗ Smart memory / predictive behavior
  ✗ Emotional animations and rich expressiveness
  ✗ Companion behavior (popcorn mode, notes, etc.)
  ✗ Advanced performance lab
  ✗ Cross-device features
```

**The rule:** If it belongs in v2, do not implement it in v1. Ship a smaller, more complete thing rather than a larger, half-built one.

---

## v0.1 — Bring-up

> **The machine boots to an interactive root shell. Nothing graphical. Just alive.**

### 0.1 — Build Toolchain

- [ ] Cross-compiler toolchain configured and documented (GCC or Clang, x86-64-linux-gnu)
- [ ] `Makefile` at repo root with `make all`, `make clean`, `make run-qemu` targets
- [ ] QEMU launch command documented in `README.md`
- [ ] Build system produces a bootable image (ISO or raw disk) as an output artifact

**Done when:** `make run-qemu` boots to something.

---

### 0.2 — Bootloader

- [ ] Limine bootloader selected (DL-005)
- [ ] Limine installed and configured: `limine.conf` written
- [ ] Kernel image placed at boot path
- [ ] Bootloader shows Mahina boot entry (no graphical splash yet — text only is fine)
- [ ] UEFI boot entry registered correctly

**Done when:** QEMU boots and the kernel receives control.

---

### 0.3 — Kernel

- [ ] Linux 6.6.x LTS selected as v1 kernel
- [ ] Minimal kernel config written for x86-64 UEFI + QEMU
- [ ] Required features: DRM/KMS, USB HID, Btrfs, virtio (for QEMU), ext4 (for initial rootfs)
- [ ] Kernel compiles from source as part of `make all`
- [ ] Kernel boots in QEMU and reaches PID 1

**Done when:** QEMU shows kernel boot messages and hands off to PID 1.

---

### 0.4 — Root Filesystem & Directory Structure

- [ ] Root filesystem: Btrfs (DL-027)
- [ ] Btrfs subvolumes created: `@` (root), `@home`, `@snapshots`
- [ ] Directory structure matches `Volume II / 09_filesystem.md`:
  - [ ] `/usr/bin`, `/usr/lib`, `/usr/share`
  - [ ] `/etc/luna/` — Mahina configuration root
  - [ ] `/var/log/luna-init/`
  - [ ] `/run/lgp/` — LGP compositor socket
  - [ ] `/home/<user>/.luna/` — user AI data
- [ ] EFI system partition mounted at `/boot/efi`

**Done when:** `ls /` shows the correct Mahina directory structure.

---

### 0.5 — C Runtime

- [ ] glibc version pinned for v1 (musl migration deferred — DL-007)
- [ ] Dynamic linker path: `/lib/ld-linux-x86-64.so.2`
- [ ] A statically-linked and a dynamically-linked C binary both run in QEMU

**Done when:** `./hello` (dynamic) and `./hello-static` (static) both print output in QEMU.

---

### 0.6 — luna-init: PID 1

- [ ] `src/luna-init/` created — C, single file initially
- [ ] Starts as PID 1
- [ ] Mounts: `/proc`, `/sys`, `/dev` (devtmpfs)
- [ ] Pivots from initramfs to real root
- [ ] Sets hostname from `/etc/hostname`
- [ ] Logs to `/var/log/luna-init/boot.log` (format: `[luna-init] [LEVEL] message`)
- [ ] Signal handling: `SIGCHLD` (reap), `SIGTERM` (shutdown), `SIGINT` (reboot)
- [ ] Drops to interactive root shell at end of Stage 3

**Done when:** QEMU shows the Mahina boot log and an interactive root shell.

---

### 0.7 — initramfs

- [ ] initramfs build script written
- [ ] Contents: luna-init binary, busybox, udev/mdev, kernel modules for root disk
- [ ] initramfs mounts real root, execve's `/sbin/luna-init` from real root
- [ ] Full chain: UEFI → limine → kernel → initramfs → real root → luna-init shell

**Done when:** Full boot chain works in QEMU without any manual intervention. `v0.1 complete.`

---

## v0.5 — Developer Preview

> **A graphical desktop appears. The terminal works. The package manager works. You can actually use it.**

### 1.1 — Service Manager (luna-init Stage 4)

- [ ] Service definition format: TOML in `/etc/luna/services/` (DL-008)
- [ ] TOML schema: `name`, `binary`, `args`, `slice`, `restart`
- [ ] luna-init reads service files, forks+execs services in their cgroup slice
- [ ] Cgroup v2 hierarchy:
  - [ ] `luna-compositor.slice` (weight 800)
  - [ ] `luna-shell.slice` (weight 400)
  - [ ] `luna-ai.slice` (weight 200)
  - [ ] `luna-system.slice` (weight 300)
- [ ] Service: D-Bus daemon
- [ ] Service: NetworkManager
- [ ] Service: PipeWire (audio)
- [ ] Service restart on failure: 3 retries then `luna-init` logs FATAL

**Done when:** All system services start in correct dependency order. Killing a service causes it to restart.

---

### 1.2 — Logging Infrastructure

- [ ] Boot log: `/var/log/luna-init/boot.log`
- [ ] Runtime log: `/var/log/luna-init/runtime.log`
- [ ] Log rotation: 10 MB max, `.1` / `.2` suffixes
- [ ] Severity levels: `DEBUG`, `INFO`, `WARN`, `ERROR`, `FATAL`
- [ ] `luna_log.h` C header — shared by all system services

**Done when:** Logs are present after every boot. Rotation triggers at 10 MB.

---

### 1.3 — Networking

- [ ] NetworkManager manages eth0 (QEMU NAT)
- [ ] DHCP address assigned on boot
- [ ] nftables firewall applied at startup (`/etc/nftables.conf`)
- [ ] `ping 8.8.8.8` works from root shell
- [ ] `curl https://example.com` works

**Done when:** Internet connectivity confirmed in QEMU.

---

### 1.4 — Audio (PipeWire)

- [ ] PipeWire starts as a user service (session-level, not system)
- [ ] WirePlumber starts alongside PipeWire
- [ ] `pactl info` succeeds (PulseAudio compatibility layer)
- [ ] Test audio: `paplay` plays a WAV file with audible output in QEMU (virtiofs audio or virtio-sound)

**Done when:** Audio plays. Verified in QEMU.

---

### 1.5 — Input (libinput)

- [ ] libinput linked into the compositor
- [ ] Keyboard events read and dispatched to focused window
- [ ] Pointer (mouse) events tracked and dispatched
- [ ] Input event log verifiable at compositor debug output

**Done when:** Mouse movement and keyboard input produce events in compositor log.

---

### 1.6 — DRM/KMS & Software Renderer

- [ ] Open `/dev/dri/card0`
- [ ] Enumerate KMS connectors and select active display
- [ ] Allocate framebuffer (dumb buffer for software renderer)
- [ ] Fill with LUNA Void color (`#0A0A0F`)
- [ ] Page flip via KMS
- [ ] QEMU window shows solid Mahina background

**Done when:** A solid Mahina-colored rectangle fills the QEMU display.

---

### 1.7 — Boot Splash

- [ ] `src/luna-splash/` — minimal C binary, no dynamic allocation
- [ ] Displays on framebuffer: LUNA Void background + Mahina wordmark (bitmap)
- [ ] Runs from luna-init Stage 3 until compositor takes over
- [ ] luna-init kills luna-splash (SIGTERM) before starting compositor

**Done when:** Boot shows the Mahina branding. Compositor replaces it cleanly.

---

### 1.8 — lgp-compositor (Minimum)

- [ ] `src/lgp-compositor/` created
- [ ] LGP Unix socket: `/run/lgp/compositor.sock`
- [ ] Accepts client connections
- [ ] Messages: `LGP_CREATE_SURFACE`, `LGP_DESTROY_SURFACE`, `LGP_COMMIT_BUFFER`
- [ ] Compositor frame loop: composites all surfaces → presents via KMS
- [ ] Sends readiness signal (D-Bus) on start
- [ ] luna-init Stage 5: starts compositor, waits for readiness

**Done when:** A test client creates a surface and the compositor displays it.

---

### 1.9 — LGP Protocol: Full v1 Implementation

- [ ] TLV binary wire format (DL-025)
- [ ] All protocol messages:
  - [ ] `LGP_CREATE_SURFACE`, `LGP_DESTROY_SURFACE`, `LGP_COMMIT_BUFFER`
  - [ ] `LGP_REQUEST_FRAME` / `LGP_FRAME_CALLBACK`
  - [ ] `LGP_SET_SEMANTIC_COLOR`, `LGP_SET_GEOMETRY`, `LGP_SET_LAYER`
  - [ ] `LGP_SEND_MOTION`, `LGP_INPUT_EVENT`, `LGP_COMPOSITOR_ERROR`
- [ ] Z-order layer system: WALLPAPER → CURSOR (7 layers)
- [ ] Input routing: pointer hit-test, keyboard focus
- [ ] Frame callbacks: vsync-aligned via DRM vblank

**Done when:** Protocol compliance test passes. All message types work. Invalid messages return errors.

---

### 2.0 — LunaGUI Toolkit

- [ ] `src/lunagui/` — C library
- [ ] Widget tree: `LunaWindow`, `LunaPanel`, `LunaLabel`, `LunaButton`, `LunaToggle`, `LunaTextInput`
- [ ] Box model layout: horizontal + vertical containers, grow/shrink
- [ ] Canvas API: `fill_rect`, `stroke_rect`, `fill_text`, `push_clip`, `pop_clip`
- [ ] Semantic color API: `LunaTheme.current().color.*` tokens (no hex values in widget code)
- [ ] Animation API: `widget.animate(LunaAnimation.FadeIn)` → `LGP_SEND_MOTION(Fade)`
- [ ] Event system: `on_click`, `on_change`, `on_submit`, `on_close_request`
- [ ] Text rendering: FreeType + HarfBuzz (DL-029), Bitcount display font (DL-028), Inter body font (DL-050)
- [ ] LunaGUI test application: window with a button and label — runs on compositor

**Done when:** A LunaGUI application compiles, shows a window, and receives click events.

---

### 2.1 — Luna Dark Theme

- [ ] Color Resolver: semantic color code → hex value (Luna Dark palette)
- [ ] All 6 semantic colors correct: LUNA_BLUE, LUNA_GREEN, LUNA_AMBER, LUNA_PINK, LUNA_VOID, LUNA_RED
- [ ] `LunaTheme.current()` returns hardcoded Luna Dark values (TOML-loaded in v1.5)
- [ ] Typography scale: Bitcount for display (18pt+), Inter for body (10–17pt), JetBrains Mono for terminal
- [ ] Phosphor Icons rendered at 24px via SVG pipeline (DL-051)

**Done when:** Entire desktop uses Luna Dark consistently. No hex values in application code.

---

### 2.2 — Animation Engine

- [ ] Integrated into compositor render loop
- [ ] All 9 motion classes with correct easing functions
- [ ] Animation Budget enforcement: auto-complete animations exceeding ceiling
- [ ] Concurrent complexity budget: 30 (default profile)
- [ ] `ease-out-quint` as primary easing curve (DL, Volume III)
- [ ] Timing vocabulary respected: Instant/Fast/Standard/Deliberate/Ambient

**Done when:** All motion classes produce correct on-screen animations within budget.

---

### 2.3 — luna-shell

- [ ] `src/luna-shell/` — C, uses LunaGUI
- [ ] LAYER_WALLPAPER surface: renders LUNA Void background
- [ ] Application launcher: opens on key/click
- [ ] Window management: tracks open windows, brings to front on click
- [ ] Single workspace (multi-workspace in v1.5)

**Done when:** Desktop background appears. Application launcher opens.

---

### 2.4 — luna-bar

- [ ] LAYER_SHELL surface
- [ ] Displays: clock (top-right, updates every second), network status dot, AI status dot
- [ ] Network status: reads NetworkManager D-Bus
- [ ] AI status: green (READY), amber (DEGRADED/MODEL_NOT_INSTALLED), void (offline)

**Done when:** Status bar visible with working clock.

---

### 2.5 — Luna Dock

- [ ] `src/luna-dock/` — C, uses LunaGUI
- [ ] LAYER_SHELL surface (bottom of screen)
- [ ] Pinned application icons (configured in `~/.luna/config/dock.toml`)
- [ ] Click icon → launch application or focus existing window
- [ ] Running indicator: dot beneath icon for running apps
- [ ] Dock uses Phosphor Icons for system apps

**Done when:** Dock is visible. Clicking an icon launches the application.

---

### 2.6 — luna-terminal

- [ ] `src/luna-terminal/` — C, uses LunaGUI
- [ ] Embeds a PTY (openpty/forkpty)
- [ ] Spawns user's default shell (`$SHELL` or bash)
- [ ] VT100/VT220 escape sequence rendering
- [ ] JetBrains Mono monospace font
- [ ] Copy/paste via LGP clipboard extension (DL-033)
- [ ] Configurable: font size, scrollback lines

**Done when:** Terminal opens, bash runs, basic commands work.

---

### 2.7 — lpkg: Core Package Manager

- [ ] `src/lpkg/` — Python v1 (consistent with the tooling approach)
- [ ] Package format: `.lpkg` (tar.zst + `luna.toml` manifest)
- [ ] `lpkg install <name>` — download, verify Ed25519 signature (DL-048), verify SHA-256, install
- [ ] `lpkg remove <name>` — clean removal
- [ ] `lpkg list` — list installed packages
- [ ] `lpkg update` — fetch repository index
- [ ] `lpkg upgrade` — upgrade all packages
- [ ] Package database: SQLite at `/var/lib/lpkg/installed.db`
- [ ] Atomic installs: either all files or none (DL-018)
- [ ] Btrfs snapshot before system-wide installs (DL-027)
- [ ] User-level installs: `~/.local/` (DL-017)
- [ ] Key location: `/etc/luna/keys/mahina-official.pub`

**Done when:** `lpkg install` downloads, verifies, and installs a signed test package.

---

### 2.8 — Basic User Login

- [ ] Login screen (`luna-lock`) renders on compositor at startup
- [ ] Username + password entry
- [ ] PAM authentication
- [ ] On success: launches user session (luna-shell, luna-dock, luna-bar, luna-island)
- [ ] Session teardown on logout

**Done when:** The login screen appears on boot. Correct password launches the desktop. `v0.5 complete.`

---

## v0.9 — Feature Complete

> **Installer works. All core applications exist. LUNA's basic runtime is running. This is what v1.0 will be.**

### 3.1 — Luna Island (Basic)

- [ ] `src/luna-island/` — C, uses LunaGUI
- [ ] `LUNA_ISLAND` surface type, `LAYER_LUNA_ISLAND`
- [ ] Visual: dark rounded pill (top-center screen)
- [ ] Presence dot: ● colored by LUNA state
  - [ ] LUNA_GREEN = READY
  - [ ] LUNA_AMBER = DEGRADED / MODEL_NOT_INSTALLED
  - [ ] LUNA_VOID = offline
- [ ] Click → COMPACT_PANEL expands (shows brief status text, not full chat)
- [ ] Subscribes to `org.mahina.luna.ModeChanged` via D-Bus
- [ ] **No personality animations in v1** — static presence only
- [ ] Always visible, highest Z-order (never covered)

**Done when:** Luna Island is visible, color reflects LUNA state, click shows brief panel.

---

### 3.2 — luna-ai-d: Presence Engine (No LLM Yet)

- [ ] `luna-ai-d/` — Python 3.12+, asyncio throughout (DL-049)
- [ ] Self-contained venv at `/usr/lib/luna-ai-d/`
- [ ] Starts at luna-init Stage 6
- [ ] Context engine: monitors active application (allowed by `observe.toml`, DL-022)
- [ ] Mode detection: AMBIENT, DEVSHELL, FOCUS, STUDY, CREATIVE
- [ ] D-Bus API: `org.mahina.luna.GetMode()`, `org.mahina.luna.ModeChanged` signal
- [ ] Memory dir initialized: `~/.luna/memory/`
- [ ] RAM target: ≤ 80 MB (daemon only)
- [ ] Startup target: ≤ 2 seconds to READY state (presence only, no LLM)
- [ ] Inference Engine wired up but NOT started yet (lazy — waits for next section)

**Done when:** LUNA mode changes based on focused application. Luna Island reflects it within 500ms.

---

### 3.3 — luna-ai-d: LLM Integration (Qwen2.5 3B)

- [ ] Ollama binary included in OS image via lpkg
- [ ] Ollama starts lazily on first conversation request
- [ ] Default model: Qwen2.5 3B Q4_K_M (DL-046), Ollama pull name: `qwen2.5:3b`
- [ ] `MODEL_NOT_INSTALLED` state: Luna Island shows LUNA_AMBER, notification displayed (DL-047)
- [ ] `luna model install qwen2.5:3b` — user-initiated model download
- [ ] D-Bus API: `org.mahina.luna.Chat(prompt)` → response, streaming signal per token
- [ ] OOM behavior: Ollama killed first under memory pressure (highest OOM score)
- [ ] DEGRADED mode on Ollama failure: Presence Engine still runs, LLM unavailable

**Done when:** `luna chat "Hello"` receives a coherent response from the local Qwen2.5 model.

---

### 3.4 — Basic Chat Interface

- [ ] Chat panel in Luna Island (COMPACT_PANEL expands to CONVERSATION view)
- [ ] Input field at bottom, LUNA response rendered above
- [ ] Streaming tokens displayed as they arrive (no waiting for full response)
- [ ] Bitcount font for LUNA responses; Inter for user input
- [ ] Conversation panel closes on Escape or ×
- [ ] **No memory persistence yet** — each session starts fresh (memory engine wired in v1.0 stable)

**Done when:** User can type a message, LUNA responds, conversation is readable.

---

### 3.5 — LUNA System Awareness

- [ ] luna-ai-d reads system stats via `/proc`:
  - [ ] CPU usage (from `/proc/stat`)
  - [ ] RAM usage (from `/proc/meminfo`)
  - [ ] Battery level (from `/sys/class/power_supply/`)
  - [ ] Current time (from system clock)
- [ ] LUNA responds to queries: "What's my RAM usage?" "Is my battery low?"
- [ ] Proactive notification: LUNA notifies (non-blocking) when RAM > 80% for 60+ seconds

**Done when:** Asking LUNA "how's my memory?" returns a real value.

---

### 3.6 — LUNA Permission System

- [ ] Permission Engine: all LUNA actions with system effects require user approval
- [ ] Graphical permission dialog: uses `LAYER_SYSTEM_MODAL` surface
- [ ] Permission categories:
  - [ ] `file_read` — LUNA wants to read a file
  - [ ] `system_change` — LUNA wants to change a setting
  - [ ] `package_install` — LUNA wants to install software
- [ ] All approvals logged to `~/.luna/memory/permissions.log`
- [ ] Permission dialog consistent with Luna Dark visual language

**Done when:** Any LUNA action that requires elevated capability shows the permission dialog first.

---

### 3.7 — luna-notif

- [ ] `src/luna-notif/` — C, uses LunaGUI
- [ ] `LAYER_NOTIFICATION` surface
- [ ] D-Bus API: `org.freedesktop.Notifications.Notify` (standard interface)
- [ ] Notification card: title, body, icon, `Slide` animation in
- [ ] Auto-dismiss after 5 seconds (configurable), `Slide` animation out
- [ ] Multiple notifications stack

**Done when:** `notify-send "Hello" "World"` shows a notification card.

---

### 3.8 — luna-files (File Manager)

- [ ] `src/luna-files/` — C, uses LunaGUI
- [ ] Browsing: list files and directories
- [ ] Navigation: forward, back, up, bookmarks
- [ ] File operations: open (launch associated app), rename, delete, copy, paste
- [ ] Uses Phosphor Icons for file type indicators
- [ ] Keyboard navigation: arrows, Enter to open, Delete to trash

**Done when:** User can browse their home directory and open a file.

---

### 3.9 — luna-settings

- [ ] `src/luna-settings/` — C, uses LunaGUI
- [ ] Sections for v1:
  - [ ] Display (resolution, refresh rate)
  - [ ] Network (active connection, static IP)
  - [ ] Sound (output device, volume)
  - [ ] Users (change password)
  - [ ] AI (model status, install/remove model, DEGRADED mode info)
  - [ ] Privacy (`observe.toml` toggles — what LUNA observes)
- [ ] Changes apply immediately without restart where possible

**Done when:** User can change display resolution and audio output from the Settings app.

---

### 4.0 — Installer (luna-installer)

- [ ] `src/luna-installer/` — C, uses LunaGUI, runs as FULLSCREEN LGP client
- [ ] Screens:
  - [ ] Welcome (Install / Try without installing / Advanced)
  - [ ] Language & locale (auto-detected)
  - [ ] Keyboard layout (with test field)
  - [ ] Disk selection (safety rules: OS detection, "Erase [disk ID] and Install" button)
  - [ ] Partition layout preview (Btrfs: @, @home, @snapshots — DL-027)
  - [ ] Privacy configuration (`observe.toml` toggles — DL-022)
  - [ ] User account creation
  - [ ] Installation summary (review before any disk writes)
  - [ ] Installation progress
  - [ ] First boot setup (model download — online path and offline path per DL-047)
- [ ] LUNA present in installer (INSTALL mode: no LLM, template guidance text only)
- [ ] luna-installer writes ISO image to disk, configures limine, writes fstab

**Done when:** Mahina installs successfully from ISO to a QEMU disk image. Installed system boots.

---

### 4.1 — Keyboard Navigation & Accessibility

- [ ] Tab / Shift-Tab cycles through all interactive LunaGUI widgets
- [ ] Focus ring: 2px LUNA_BLUE border (DL, Visual Language)
- [ ] Enter / Space activates focused widget
- [ ] Escape closes dialogs / panels
- [ ] Reduced-motion mode: all animations snap (config in `~/.luna/config/display.toml`)

**Done when:** Entire desktop navigable without a mouse.

---

### 4.2 — AppArmor Profiles

- [ ] AppArmor profiles written for all system daemons:
  - [ ] luna-init, lgp-compositor, luna-ai-d, luna-shell, luna-dock, luna-bar, luna-island
  - [ ] luna-terminal, luna-files, luna-settings, luna-notif
- [ ] Third-party apps: AppArmor profile generated by lpkg at install time
- [ ] Profiles enforcing mode (not complain mode) in the release build

**Done when:** AppArmor reports no unconfined system processes.

---

### 4.3 — lpkg: Graphical Permission Integration

- [ ] lpkg sends permission request via D-Bus to luna-ai-d
- [ ] LUNA permission dialog appears (LAYER_SYSTEM_MODAL)
- [ ] Allow → install proceeds
- [ ] Deny → install aborted cleanly
- [ ] Privilege escalation for system-wide installs; user-level installs require no dialog

**Done when:** `lpkg install <package>` triggers the graphical permission dialog for system-wide installs.

---

### 4.4 — Session Memory (Basic)

- [ ] `~/.luna/memory/` structure initialized on first luna-ai-d start
- [ ] Session summary written on SIGTERM (shutdown/logout)
- [ ] Summary loaded on next session start (gives LUNA basic continuity)
- [ ] `luna memory --clear` command
- [ ] `luna memory --audit` command (human-readable dump)
- [ ] Memory encryption: `~/.luna/memory/` is `chmod 700` (DL-023)
- [ ] **No predictive behavior** — memory for continuity only in v1

**Done when:** After reboot, LUNA knows the previous session happened. `v0.9 complete.`

---

## v1.0 — Public Release

> **Mahina is stable, installable, and daily-usable for development and experimentation. The foundation is complete.**

### 5.1 — Hardware Compatibility Pass

- [ ] Tested on at least 3 different physical x86-64 machines
- [ ] At least 1 Intel CPU, 1 AMD CPU
- [ ] At least 1 laptop (trackpad via libinput)
- [ ] At least 1 system with integrated graphics only
- [ ] Boot, display, input, networking confirmed on all test machines
- [ ] Known incompatibilities documented in release notes

---

### 5.2 — Performance Verification

All benchmarks from `Volume VI / 04_benchmarks.md` verified on reference hardware:

- [ ] Boot to interactive desktop: ≤ 5 seconds
- [ ] Compositor: stable 60 FPS, ≤ 2% CPU at idle
- [ ] luna-ai-d startup: ≤ 2 seconds to READY state
- [ ] Qwen2.5 3B first token (warm): ≤ 2 seconds (DL-046)
- [ ] System RAM footprint at idle: ≤ 1.5 GB (excluding LLM)
- [ ] Disk footprint (base OS): ≤ 8 GB including default model

---

### 5.3 — Documentation Synchronization

All spec documents updated to reflect as-built reality:

- [ ] `Volume II` — all system architecture docs match the implementation
- [ ] `Volume III` — visual language matches actual rendering output
- [ ] `Volume IV` — LUNA runtime docs match the Python implementation
- [ ] `Volume V` — installer and lpkg docs match actual user experience
- [ ] `Volume VI` — milestones updated, Stage 0–5 exit criteria all verified
- [ ] `Volume VII` — this roadmap updated: all v1.0 items marked `[x]`
- [ ] `decision_log.md` — all accepted decisions reflect as-built state
- [ ] `CHANGELOG.md` — complete history from v0.1 to v1.0

---

### 5.4 — Release Build & Signing

Following the process in `Volume VI / 07_release_process.md`:

- [ ] Clean release build: `make release VERSION=1.0.0`
- [ ] ISO produced: `mahina-1.0.0-x86_64.iso`
- [ ] SHA-256 checksum produced
- [ ] ISO signed with Ed25519 release key (DL-048)
- [ ] Package manifest signed
- [ ] Signatures verified on a clean machine before publishing

---

### 5.5 — GitHub Release

- [ ] Git tag: `git tag v1.0.0`
- [ ] GitHub Release created with:
  - [ ] `mahina-1.0.0-x86_64.iso`
  - [ ] `mahina-1.0.0-x86_64.iso.sig`
  - [ ] `mahina-1.0.0-x86_64.iso.sha256`
  - [ ] Release notes (what's in v1.0, known issues, installation instructions)
  - [ ] Verification instructions (how to verify the ISO signature)
- [ ] README updated with installation instructions anyone can follow

**Done when:** `v1.0.0 is public.`

---

## Progress Tracker

```
v0.1  Bring-up             [ ] Not started
v0.5  Developer Preview    [ ] Not started
v0.9  Feature Complete     [ ] Not started
v1.0  Public Release       [ ] Not started
```

Update this tracker when a version milestone is complete.

---

## v1.0 Marketing Statement

> *"The first public release of Mahina — a new operating system built around presence, local AI, and user control.*
> *This release establishes the foundation; the living interface will evolve in future versions."*

Do not market v1.0 as "The OS that understands you."

That is v2.0's promise. v1.0 delivers the ground it stands on.

---

## Decisions Still Required Before Implementation

| Decision Needed | Blocks | Status |
|---|---|---|
| LGP wire format (TLV binary) | 1.8 — LGP compositor | ✅ Resolved (DL-025) |
| GPU backend (Vulkan + EGL fallback) | 1.6 — DRM/renderer | ✅ Resolved (DL-026) |
| Root filesystem (Btrfs) | 0.4 — filesystem | ✅ Resolved (DL-027) |
| Display font (Bitcount) | 2.0 — LunaGUI text | ✅ Resolved (DL-028) |
| Text rendering library (FreeType + HarfBuzz) | 2.0 — LunaGUI | ✅ Resolved (DL-029) |
| Default AI model (Qwen2.5 3B) | 3.3 — LLM integration | ✅ Resolved (DL-046) |
| First-boot offline behavior | 3.3 — installer | ✅ Resolved (DL-047) |
| lpkg signing (Ed25519) | 2.7 — lpkg | ✅ Resolved (DL-048) |
| luna-ai-d language (Python) | 3.2 — AI daemon | ✅ Resolved (DL-049) |
| Companion font (Inter) | 2.0 — LunaGUI text | ✅ Resolved (DL-050) |
| Icon set (Phosphor Icons) | 2.1 — theme engine | ✅ Resolved (DL-051) |
| AVX2 hardware requirement | 3.3 — LLM performance | ⚠ Needs DL entry |
| Dependency resolution algorithm | 2.7 — lpkg | ⚠ Needs DL entry |

All blocking decisions for v1.0 implementation are resolved. The two remaining open items are not on the critical path for v0.1 bring-up.

---

*Document: `Volume VII / implementation_roadmap.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 2.0 — Fully revised for v1.0 vision*
*Status: Living document — updated with every completed milestone*
*This document is the daily engineering reference. If you are writing code not listed here, you are either ahead of schedule or off course.*
