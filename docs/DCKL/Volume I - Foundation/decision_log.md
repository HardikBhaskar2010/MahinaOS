# LunaOS — Decision Log
**Volume I · Chapter 6**
**Classification:** Foundation Document — Architecture Record
**Status:** Append-only. Decisions are never deleted, only superseded.

---

## Purpose

This document records every significant architectural, design, or strategic decision made during LunaOS development. It exists so that:

1. Future contributors understand *why* things are the way they are
2. AI tools (Claude Code, etc.) have full context when making changes
3. When a decision needs to be revisited, the original reasoning is available
4. We don't debate the same question twice

Format per entry:
```
## [DL-XXX] Decision Title
Date: YYYY-MM-DD
Status: ACCEPTED | REJECTED | SUPERSEDED by DL-XXX | PENDING
Decided by: Hardik Bhaskar

### Question
What were we deciding?

### Options Considered
What alternatives did we evaluate?

### Decision
What did we choose?

### Reasoning
Why this choice over alternatives?

### Consequences
What does this lock in or rule out?
```

---

## [DL-001] No Upstream Distro Base

**Date:** Project start
**Status:** ACCEPTED — Constitutional (see Core Laws)

### Question
Should LunaOS base on an existing distribution (Arch, Debian, Void) or build fully from scratch?

### Options Considered
- **Arch Linux base** — Maximum flexibility, AUR available, familiar
- **Debian base** — Maximum stability, huge package ecosystem
- **Void Linux** — Already uses runit, musl-capable, lightweight
- **Linux From Scratch approach** — Full ownership, no inherited decisions

### Decision
Linux From Scratch approach. No upstream base.

### Reasoning
The entire premise of LunaOS is that every layer is owned and understood. Using Arch or Debian would mean inheriting thousands of decisions we didn't make and can't fully account for. Specifically:
- We want to write `luna-init` — using Arch would mean coexisting with or replacing systemd, not starting clean
- We want to control the kernel config precisely — distro kernels include hundreds of drivers and options we don't need
- The narrative of "built from scratch" is central to the project's identity

### Consequences
- Full responsibility for system stability and package availability
- `lpkg` must handle everything systemd, pacman, etc. would have handled
- Build system complexity is significantly higher
- Cannot use AUR, PPA, or similar community package sources directly

---

## [DL-002] Init System: luna-init in C

**Date:** Project start
**Status:** ACCEPTED

### Question
Which init system should LunaOS use?

### Options Considered
- **systemd** — Industry standard, feature-complete, complex
- **OpenRC** — Simpler, shell-based, used by Gentoo/Alpine
- **runit** — Minimal, reliable, used by Void
- **s6** — Extremely minimal, mathematically correct supervision
- **luna-init (custom)** — Written in C, TOML service files

### Decision
`luna-init` — custom init written in C with TOML-based service files.

### Reasoning
- systemd violates DL-001 (not understood at every layer, massive complexity)
- OpenRC is tempting but still "someone else's" init
- runit/s6 are excellent but we'd still be configuring someone else's tool
- Writing `luna-init` means we understand PID 1 completely
- C is the right language for PID 1 — minimal runtime, direct syscalls, crash-proof profile
- TOML service files are human-readable and version-controllable

### Consequences
- luna-init must handle: zombie reaping, filesystem mounting, service deps, shutdown sequencing
- We own every bug in PID 1 — this is a significant responsibility
- Phase 0: shell script init → Phase 1: C implementation

---

## [DL-003] Package Manager: lpkg in Python (v1)

**Date:** Project start
**Status:** ACCEPTED — with planned evolution

### Question
What language for `lpkg`?

### Options Considered
- **Python** — Fast to write, we know it well, readable codebase
- **Rust** — Faster execution, impressive on GitHub, type-safe
- **C** — Minimal runtime, fits the "no deps" philosophy

### Decision
Python for v1. Rust rewrite planned for v2 if performance matters.

### Reasoning
- We know Python deeply (FastAPI/Veronica experience)
- `lpkg` is not performance-critical — it runs occasionally, not continuously
- A working Python implementation in 2 weeks beats a partially working Rust implementation in 8
- SQLite + Python for the package database is a natural fit

### Consequences
- Python 3.12+ is a system dependency
- lpkg is installed before Python is managed by lpkg (bootstrapping problem — documented separately)
- v2 Rust rewrite is on the roadmap; Python codebase should be clean enough to reference

---

## [DL-004] Compositor: Hyprland (v1) → wlroots custom (v2)

**Date:** Project start
**Status:** ACCEPTED

### Question
Which Wayland compositor strategy for LunaOS?

### Options Considered
- **Write custom compositor from scratch using wlroots**
- **Use Hyprland with deep custom config + IPC hooks**
- **Use Sway (i3-compatible, more stable)**
- **Use KWin (KDE's compositor)**

### Decision
Ship Hyprland for v1. Build wlroots-based custom compositor for v2.

### Reasoning
- Writing a custom compositor from scratch is months of work that doesn't ship v1
- Hyprland has excellent IPC — LUNA.AI can hook into window events without compositor code
- Hyprland's animation system is already hardware-accelerated and configurable
- From a user perspective, a deeply configured Hyprland *looks* completely custom
- v2 gives us time to learn wlroots properly while users get a real desktop in v1

### Consequences
- v1 has a Hyprland dependency — must write LBUILD for it
- Hyprland config changes can break LunaOS shell behavior
- IPC socket path is Hyprland-specific — abstraction layer needed for v2 migration
- Custom compositor in v2 is not optional — it's needed for Luna Island proper layer implementation

---

## [DL-005] Bootloader: limine (v1)

**Date:** Project start
**Status:** ACCEPTED — GRUB2 deferred

### Question
Which bootloader for LunaOS?

### Options Considered
- **GRUB2** — Universal, extensive theming examples, complex config
- **limine** — Modern, simpler config, UEFI-first, cleaner aesthetic
- **systemd-boot** — Not applicable (we don't use systemd)

### Decision
limine for v1.

### Reasoning
- limine has a cleaner configuration format
- Better suited for the cyberpunk boot screen aesthetic
- Less legacy baggage — designed for modern UEFI systems
- Simpler to build into our ISO creation pipeline

### Consequences
- Must write limine config in our `build-iso.sh`
- Some older BIOS systems may have compatibility questions — document minimum hardware
- Theming resources are less available than GRUB2 — we write our own anyway

---

## [DL-006] AI Runtime: Ollama

**Date:** Project start
**Status:** ACCEPTED

### Question
How should LUNA.AI serve local model inference?

### Options Considered
- **Ollama** — Simple REST API, model management built-in, cross-platform
- **llama.cpp direct** — Lower overhead, more control, requires more code
- **Hugging Face Transformers** — Python-native, huge model support, heavy

### Decision
Ollama for v1. llama.cpp direct for v2 if overhead is measurable.

### Reasoning
- Ollama's REST API matches our FastAPI daemon architecture perfectly
- `POST /api/generate` is one function call
- Model management (pull, list, delete) is handled for us
- llama.cpp direct would require us to maintain model loading code

### Consequences
- Ollama binary is a system dependency
- LBUILD file needed for Ollama
- Port: 11434 (Ollama default, internal only)
- Memory usage: Ollama holds model in RAM — must be accounted for in minimum specs

---

## [DL-007] C Library: glibc → musl (planned)

**Date:** Project start
**Status:** ACCEPTED — phased migration

### Question
Which C library for LunaOS userspace?

### Options Considered
- **glibc** — Maximum binary compatibility, heavy, complex
- **musl libc** — Minimal, clean, static-linking friendly, some compat issues

### Decision
glibc for v1. musl migration planned for v2.

### Reasoning
- Fighting libc compatibility and building the OS at the same time is two battles
- glibc gives us access to more pre-compiled software during development
- musl's benefits (small footprint, static linking) become relevant once the stack is stable

### Consequences
- v1 images will be larger than they need to be
- musl migration is a breaking change — requires rebuilding all packages
- Plan musl as a v2.0 (New Moon) milestone

---

## [DL-008] Config Format: TOML

**Date:** Project start
**Status:** ACCEPTED

### Question
What config format for luna-init service files, lpkg manifests, luna.toml?

### Options Considered
- **TOML** — Readable, typed, python-toml library solid
- **YAML** — Familiar, but edge cases are notorious
- **INI** — Simple but no types
- **JSON** — Not human-writable

### Decision
TOML for all LunaOS config files.

### Reasoning
- YAML's whitespace sensitivity causes hard-to-debug errors
- TOML is typed, readable, and has no surprising edge cases
- `tomllib` is in Python 3.11+ stdlib — no external dep for parser

### Consequences
- All service files use `.toml` extension
- All user config in `~/.luna/` is TOML
- Documentation must provide TOML examples, not YAML

---

## [DL-009] Kernel Version: Linux 6.6.x LTS

**Date:** Project start
**Status:** ACCEPTED — version pinned, upgrade on new LTS

### Question
Which Linux kernel version?

### Decision
6.6.x LTS. Track security patches only, not feature releases.

### Reasoning
- LTS kernels receive 6 years of support
- 6.6 supports: cgroups v2, BPF, io_uring, Wayland DRM, all hardware we care about
- Chasing latest kernel introduces instability during OS development phase

---

## [DL-010] LUNA.AI Port: 7734

**Date:** Project start
**Status:** ACCEPTED

### Question
What port does luna-ai-d listen on?

### Decision
`localhost:7734`. Internal only. Not exposed to network.

### Reasoning
- 7734 is not assigned by IANA to any standard service
- Memorable: 7734 upside down in a calculator spells "hELL" — cyberpunk.
- Firewall rules block this port from any external interface by default

---

## Pending Decisions

### [DL-P01] LUNA's name for users
Should LUNA address the user by: system username | user-set name | never | system-detected name
**Target:** Before v1 beta

### [DL-P02] Sound design
Does LunaOS have UI sounds? Boot chime? Notification audio?
**Target:** Before v1 release

### [DL-P03] Default wallpaper
Original commission vs. generative art vs. procedural generator?
**Target:** Before v1 release

### [DL-P04] License
MIT vs. GPL v3
**Current leaning:** MIT — maximum adoption over control
**Target:** Before first public commit

### [DL-P05] Public release timing
Phase 2 (booting to desktop) for early community vs. Phase 4 (polished) for big launch?
**Target:** Phase 2 decision point

---

*Document: `00_Foundation/decision_log.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*This document is append-only. Add new entries at the top of the numbered section.*
