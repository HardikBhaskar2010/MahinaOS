# Mahina — Milestones & Stage Definitions
**Volume VI · Chapter 06**
**Classification:** Development Bible — Project Management
**Status:** Canonical · This is the authoritative definition of what each stage means and what "done" looks like

---

## Purpose

This document defines what each implementation stage of Mahina means, what its exit criteria are, and what must be true before the next stage begins.

The roadmap (`Volume VII / implementation_roadmap.md`) is the daily work checklist. This document is the *definition of done* — the engineering contract that governs when a stage is complete.

Read this before reading the roadmap. If the roadmap says a task is done but the exit criteria here are not met, the task is not done.

---

## Stage Overview

```
v0.1  Bring-up
  Goal: The machine boots to an interactive root shell.
  No graphics. No networking. No AI. Just: alive.

v0.5  Developer Preview
  Goal: A graphical desktop appears and responds to input.
  Compositor, shell, terminal, package manager, and networking are functional.
  This is the first version a developer can work in.

v0.9  Feature Complete
  Goal: All v1.0 features exist and work.
  Installer works. Core apps are present. LUNA runtime is running (basic).
  This is the version tested before the public release.

v1.0  Public Release
  Goal: Mahina is stable, installable, and daily-usable.
  Documentation synchronized. All milestones verified on real hardware.
  The foundation is complete.

v1.5  (future) Better desktop experience. Polish. Performance.
v2.0  (future) The Living OS — LUNA becomes a defining part of the experience.
```

---

## Stage 0 — Bare Metal

> **The machine boots, mounts the root filesystem, and gives an interactive root shell.**

### Exit Criteria

Every item must be verifiable before Stage 1 begins:

```
[ ] Cross-compiler toolchain produces a working x86-64 Linux binary
[ ] Kernel boots in QEMU — reaches /sbin/init (or equivalent)
[ ] Root filesystem is Btrfs, properly mounted at /
[ ] /proc, /sys, /dev are mounted correctly
[ ] A root shell is accessible via the QEMU console
[ ] Boot completes in < 30 seconds in QEMU (non-optimized, debug build)
[ ] Kernel panic does not occur on clean boot
[ ] Git repository has a documented `make` command that produces a bootable image
```

### Stage 0 is NOT Complete Until

- The bootloader (limine) is confirmed to load the kernel correctly
- `/etc/fstab` is written and mounts are verified
- The standard directory structure (`/usr`, `/bin`, `/etc`, `/var`) is in place

---

## Stage 1 — Init System

> **luna-init manages a full service lifecycle on a Mahina system.**

### Exit Criteria

```
[ ] luna-init starts as PID 1 and does not crash
[ ] luna-init reads service definitions from /etc/luna/services/
[ ] Services start in dependency order (verified by log timestamps)
[ ] A crashing service is restarted automatically (verified by killing it manually)
[ ] System shuts down cleanly on SIGTERM to PID 1 (all services stop, filesystems unmount)
[ ] Zombie reaping works (no defunct processes accumulate)
[ ] luna-init logs to /var/log/luna-init/ with timestamps
[ ] Boot reaches shell login in < 8 seconds (QEMU, debug build)
```

### Stage 1 is NOT Complete Until

- `luna-init` handles the following signals correctly: `SIGTERM`, `SIGCHLD`, `SIGHUP`
- Service dependency failures are logged — a failed dependency blocks dependent services
- Integration test: kill a running service 5 times; verify it restarts each time

---

## Stage 2 — Display & Shell

> **A graphical desktop appears and responds to keyboard and mouse input.**

### Exit Criteria

```
[ ] lgp-compositor starts and opens a DRM/KMS framebuffer
[ ] A window can be created via the LGP protocol (TLV wire format, DL-025)
[ ] luna-shell starts as an LGP client and renders a visible shell surface
[ ] Luna Dock renders its icon bar
[ ] Luna Bar renders with clock (updating every second)
[ ] Mouse cursor renders and tracks hardware mouse movement
[ ] Keyboard input reaches the focused window correctly
[ ] Window focus changes on click (compositor owns focus, DL-033)
[ ] Luna Island is visible in AMBIENT state (static, not animated — animation is Stage 3)
[ ] Screenshot of the desktop can be captured via LGP protocol
[ ] Frame rate is stable at 60 FPS on reference hardware under idle load
```

### Stage 2 is NOT Complete Until

- Clipboard works between two test applications via LGP extension (DL-033)
- D-Bus compositor readiness signal fires on startup (DL-031)
- luna-lock starts on idle timeout and requires password to unlock
- At least two native applications launch and render correctly (luna-terminal, luna-files)

---

## Stage 3 — Presence

> **LUNA is alive. The interface has personality and responds to context.**

### Exit Criteria

```
[ ] luna-ai-d starts successfully in READY or DEGRADED mode
[ ] Luna Island animates (PULSE_GENTLE breathing animation)
[ ] Luna Island color changes with semantic meaning (LUNA_GREEN / LUNA_AMBER / LUNA_PINK)
[ ] LUNA mode changes when applications change (DEVSHELL, FOCUS, AMBIENT)
[ ] LUNA_BLUE highlight appears on focused interactive elements
[ ] Font rendering: Bitcount for display text, Inter for body text, JetBrains Mono for terminal
[ ] Phosphor Icons render correctly at 24px (SVG pipeline, DL-051)
[ ] Theme engine applies Luna Dark correctly to all surfaces
[ ] Glass effect: surfaces use dark tint (no backdrop blur in Stage 2 — enabled Stage 3+)
[ ] Animation timing matches the timing vocabulary (Instant/Fast/Standard/Deliberate/Ambient)
[ ] AppArmor profiles active for all system daemons
```

### Stage 3 is NOT Complete Until

- Luna Island responds to short click (COMPACT_PANEL) and long press (FULL_CONVERSATION state, even if no LLM yet)
- Context Engine observes active application and updates LUNA mode within 500ms
- LUNA's AMBIENT animation uses ≤ 1% CPU (GPU-composited)
- Backdropblur enabled on Vulkan backend and visually correct on test hardware

---

## Stage 4 — Intelligence

> **LUNA holds real conversations. Memory persists. The LLM is integrated.**

### Exit Criteria

```
[ ] Ollama starts lazily when the first conversation is initiated
[ ] Default model (Qwen2.5 3B Q4_K_M, DL-046) loads and responds correctly
[ ] First token latency < 3 seconds on reference hardware (cold start < 5s)
[ ] LUNA responds in character (personality engine active)
[ ] Memory engine writes session summary to ~/.luna/memory/ on session end
[ ] Memory engine loads previous session context on startup
[ ] LUNA enters DEGRADED mode when Ollama dies (OOM kill test)
[ ] LUNA recovers from DEGRADED when RAM becomes available
[ ] MODEL_NOT_INSTALLED state displays correct notification (DL-047)
[ ] Permission Engine approves / denies LUNA actions requiring elevated access
[ ] luna-ai-d Python daemon RAM usage ≤ 80 MB (excluding Ollama)
[ ] luna-ai-d startup time ≤ 2 seconds to READY state
```

### Stage 4 is NOT Complete Until

- The Automation Engine fires at least 3 proactive suggestions in a real workflow (git push, build failure, memory warning)
- LUNA conversation panel opens and closes with correct animations (DL-034 expansion model)
- At least 10 full conversation sessions complete without memory corruption

---

## Stage 5 — Distribution

> **Mahina can be installed on real hardware by someone who did not build it.**

### Exit Criteria

```
[ ] ISO image builds reproducibly from source
[ ] luna-installer runs to completion on target hardware (no QEMU)
[ ] Btrfs partitioning completes with @, @home, @snapshots subvolumes
[ ] observe.toml privacy configuration screen appears and writes correctly
[ ] System boots after installation without manual intervention
[ ] First boot: LUNA_AMBER displayed correctly if no model installed (DL-047)
[ ] lpkg install works for a test package (signed with Ed25519, DL-048)
[ ] lpkg creates a Btrfs snapshot before installation
[ ] lpkg rollback restores the system to the pre-install state
[ ] luna-updater checks for and applies a test system update
[ ] Hardware compatibility: boots on at least 3 different physical machines
[ ] Total installation time: < 10 minutes on a modern SATA SSD
```

### Stage 5 is NOT Complete Until

- The README contains instructions that a technically-literate person can follow to install Mahina
- At least one external tester has installed Mahina successfully without assistance
- The GitHub repository has a proper release with the ISO attached

---

## Stage Transition Protocol

When a stage is complete:

1. **Verify all exit criteria** — check every item in this document, not just the roadmap
2. **Run the full test suite** — `make test-unit && make test-integration`
3. **Record in the Decision Log** — add a DL entry marking the stage as complete with date
4. **Create a Git tag** — `git tag stage-N-complete`
5. **Update the roadmap** — mark all Stage N items as `[x]` in `implementation_roadmap.md`
6. **Write a brief retrospective** — one paragraph in a new Architecture Review: what worked, what didn't, what changed

Do not begin Stage N+1 until Stage N exit criteria are all verified.

---

## Current Stage Status

| Stage | Status | Date Completed |
|---|---|---|
| Stage 0 | Not started | — |
| Stage 1 | Not started | — |
| Stage 2 | Not started | — |
| Stage 3 | Not started | — |
| Stage 4 | Not started | — |
| Stage 5 | Not started | — |

*Update this table when a stage is completed.*

---

*Document: `Volume VI / 06_milestones.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 1.0*
*Depends on: 04_benchmarks.md, Volume VII/implementation_roadmap.md, decision_log.md*
*Informs: All implementation work — read this before picking up a task*
