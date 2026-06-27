# Mahina — Dependency Audit
**Volume VII · Chapter 5**
**Classification:** Project Management — Supply Chain
**Status:** Canonical · Updated when dependencies change

---

## Purpose

This document is the authoritative record of every third-party library, runtime, and tool that Mahina depends on. For each dependency it records: what it is, why it exists, its license, its version, and its security status.

**The rule:** If a dependency is not in this document, it is not an approved dependency. Adding a new dependency requires updating this document and a DL entry if the dependency changes an architectural boundary.

---

## Dependency Categories

```
Runtime dependencies:     Required at runtime — shipped in the Mahina distribution
Build-time dependencies:  Required to compile Mahina — not shipped to end users
Vendored dependencies:    Bundled in the source tree — no external install required
Optional dependencies:    Improve functionality but the system works without them
```

---

## Section 1: System Core Dependencies (C Components)

### Linux Kernel

| Property | Value |
|---|---|
| Component | Base OS |
| Version | 6.6.x LTS |
| License | GPL-2.0-only |
| Source | kernel.org |
| Why | The operating system kernel |
| Runtime | ✅ Yes |
| Vendored | No (downloaded and compiled) |
| CVE watch | kernel.org security advisories |

### glibc (GNU C Library)

| Property | Value |
|---|---|
| Component | C runtime, all C components |
| Version | 2.38+ |
| License | LGPL-2.1 |
| Source | gnu.org/software/libc/ |
| Why | POSIX C standard library (DL-007: musl migration deferred to v2) |
| Runtime | ✅ Yes |
| Vendored | No |
| CVE watch | glibc security advisories |

### libsodium

| Property | Value |
|---|---|
| Component | lpkg (package signing), luna-ai-d (memory encryption) |
| Version | 1.0.18+ |
| License | ISC |
| Source | libsodium.org |
| Why | Ed25519 signing and verification (DL-048), AES-256-GCM for memory |
| Runtime | ✅ Yes |
| Vendored | No |
| CVE watch | libsodium GitHub releases |
| Notes | Only approved cryptography library for Mahina |

### FreeType

| Property | Value |
|---|---|
| Component | LunaGUI (text rendering, DL-029) |
| Version | 2.13+ |
| License | FreeType License (BSD-like) or GPLv2 |
| Source | freetype.org |
| Why | Font rasterization for all LunaGUI text |
| Runtime | ✅ Yes |
| Vendored | No |

### HarfBuzz

| Property | Value |
|---|---|
| Component | LunaGUI (text shaping, DL-029) |
| Version | 8.0+ |
| License | MIT |
| Source | github.com/harfbuzz/harfbuzz |
| Why | Unicode text shaping — required for correct rendering of complex scripts |
| Runtime | ✅ Yes |
| Vendored | No |

### libinput

| Property | Value |
|---|---|
| Component | lgp-compositor (input handling, DL-032) |
| Version | 1.23+ |
| License | MIT |
| Source | gitlab.freedesktop.org/libinput/libinput |
| Why | Unified keyboard, mouse, touchpad, and touch input processing |
| Runtime | ✅ Yes |
| Vendored | No |

### libdrm

| Property | Value |
|---|---|
| Component | lgp-compositor (DRM/KMS display) |
| Version | 2.4.115+ |
| License | MIT |
| Source | gitlab.freedesktop.org/mesa/drm |
| Why | Interface to DRM/KMS for framebuffer management |
| Runtime | ✅ Yes |
| Vendored | No |

### Mesa (GBM / EGL)

| Property | Value |
|---|---|
| Component | lgp-compositor (GPU buffer management) |
| Version | 23.x+ |
| License | MIT |
| Source | mesa3d.org |
| Why | GBM (Generic Buffer Management) for GPU-rendered surfaces |
| Runtime | ✅ Yes |
| Vendored | No |
| Notes | Software renderer does not require Mesa in Stage 2; required for GPU backend |

### D-Bus

| Property | Value |
|---|---|
| Component | All IPC between system services |
| Version | 1.14+ |
| License | AFL-2.1 / GPLv2 |
| Source | dbus.freedesktop.org |
| Why | System and session IPC bus (Volume II / 08_ipc.md) |
| Runtime | ✅ Yes |
| Vendored | No |

### SQLite

| Property | Value |
|---|---|
| Component | lpkg (package database), luna-ai-d (memory engine) |
| Version | 3.42+ |
| License | Public Domain |
| Source | sqlite.org |
| Why | Embedded database for package tracking and LUNA memory |
| Runtime | ✅ Yes |
| Vendored | Yes (amalgamation in `vendor/sqlite/`) |
| Notes | Single-file amalgamation. Pinned version. No external install. |

### limine (Bootloader)

| Property | Value |
|---|---|
| Component | Bootloader (DL-005) |
| Version | 7.x |
| License | BSD-2-Clause |
| Source | github.com/limine-bootloader/limine |
| Why | UEFI bootloader for x86-64 |
| Runtime | Boot only (not a running process) |
| Vendored | Yes (binary in `bootloader/`) |

### NetworkManager

| Property | Value |
|---|---|
| Component | Network configuration service |
| Version | 1.44+ |
| License | GPL-2.0 |
| Source | networkmanager.dev |
| Why | Network device management, DHCP, Wi-Fi |
| Runtime | ✅ Yes |
| Vendored | No |

### nftables

| Property | Value |
|---|---|
| Component | Firewall |
| Version | 1.0.7+ |
| License | GPL-2.0 |
| Source | netfilter.org |
| Why | Firewall with deny-by-default inbound policy |
| Runtime | ✅ Yes |
| Vendored | No |

### PipeWire

| Property | Value |
|---|---|
| Component | Audio and video routing |
| Version | 0.3.80+ |
| License | MIT |
| Source | pipewire.org |
| Why | Audio subsystem — selected over PulseAudio/ALSA for v1 |
| Runtime | ✅ Yes |
| Vendored | No |

### zstd (zstandard)

| Property | Value |
|---|---|
| Component | lpkg (package archive compression) |
| Version | 1.5+ |
| License | BSD / GPLv2 |
| Source | github.com/facebook/zstd |
| Why | Package archive format: tar.zst (fast, high compression ratio) |
| Runtime | ✅ Yes |
| Vendored | No |

---

## Section 2: AI Runtime Dependencies (Python Components)

All Python dependencies belong to the `luna-ai-d` component (DL-049). They are bundled in a self-contained venv at `/usr/lib/luna-ai-d/` and do not pollute the system Python environment.

### Python

| Property | Value |
|---|---|
| Version | 3.12+ |
| License | PSF-2.0 |
| Why | luna-ai-d implementation language (DL-049) |
| Notes | Bundled as a standalone interpreter — not the system Python |

### ollama (Python library)

| Property | Value |
|---|---|
| Package | `ollama` |
| Version | 0.3+ |
| License | MIT |
| Source | pypi.org/project/ollama |
| Why | Ollama REST API client — primary LLM interface |

### dasbus

| Property | Value |
|---|---|
| Package | `dasbus` |
| Version | 1.7+ |
| License | LGPL-2.1 |
| Source | pypi.org/project/dasbus |
| Why | D-Bus integration for asyncio — luna-ai-d D-Bus service |

### aiofiles

| Property | Value |
|---|---|
| Package | `aiofiles` |
| Version | 23.x |
| License | Apache-2.0 |
| Source | pypi.org/project/aiofiles |
| Why | Async file I/O for memory engine operations |

### toml (tomllib)

| Property | Value |
|---|---|
| Package | Python 3.11+ stdlib `tomllib` |
| Version | stdlib |
| License | PSF-2.0 |
| Why | TOML config file parsing (`observe.toml`, `luna.toml`) |
| Notes | No external package required — Python 3.11+ stdlib |

---

## Section 3: Ollama Runtime

### Ollama

| Property | Value |
|---|---|
| Component | LLM inference server |
| Version | 0.4+ |
| License | MIT |
| Source | ollama.com |
| Why | Local LLM runtime — manages llama.cpp and model serving |
| Runtime | ✅ Yes (lazy-started by luna-ai-d) |
| Vendored | No (installed via lpkg) |
| Notes | Ollama bundles llama.cpp internally. llama.cpp is MIT licensed. |

### Qwen2.5 3B (AI Model)

| Property | Value |
|---|---|
| Component | Default LLM (DL-046) |
| Version | Qwen2.5 3B Q4_K_M |
| License | Apache 2.0 |
| Source | Alibaba Cloud via Ollama model registry |
| Why | Default language model for LUNA |
| Runtime | ✅ Yes (loaded by Ollama on demand) |
| Notes | Not bundled in ISO — downloaded on first use or by user request |

---

## Section 4: Build-Time Only Dependencies

These tools are required to compile Mahina but are not shipped to end users.

| Tool | Version | License | Why |
|---|---|---|---|
| GCC or Clang | GCC 13+ / Clang 17+ | GPL-3.0 / Apache-2.0 | C compiler |
| GNU Make | 4.3+ | GPL-3.0 | Build orchestration |
| pkg-config | 0.29+ | GPL-2.0 | Library discovery |
| meson | 1.2+ | Apache-2.0 | Optional (LunaGUI build system) |
| AFL++ | 4.x | Apache-2.0 | Fuzz testing (Volume VI/05) |
| cppcheck | 2.12+ | GPL-3.0 | Static analysis |
| clang-tidy | 17+ | Apache-2.0 | Linting |
| Valgrind | 3.21+ | GPL-2.0 | Memory error detection |
| QEMU | 8.x | GPL-2.0 | CI virtual machine |

---

## Section 5: Vendored Libraries (Bundled in Source Tree)

Libraries vendored in the source tree to eliminate build environment variability:

| Library | Location | License | Why Vendored |
|---|---|---|---|
| Unity (test framework) | `tests/vendor/unity/` | MIT | Single-file, zero friction |
| SQLite amalgamation | `vendor/sqlite/` | Public Domain | Version pinning, embedded use |
| toml-c | `vendor/toml-c/` | MIT | TOML parser for C components |

---

## Section 6: Font Assets

Fonts bundled in the Mahina distribution:

| Font | License | Bundle Path | Usage |
|---|---|---|---|
| Bitcount | Free for use (no commercial license required for v1.0) | `/usr/share/fonts/bitcount/` | Display font (DL-028) |
| Inter Variable | SIL OFL 1.1 | `/usr/share/fonts/inter/` | Body/reading font (DL-050) |
| JetBrains Mono | SIL OFL 1.1 | `/usr/share/fonts/jetbrains-mono/` | Terminal/monospace (DL-023) |

---

## Section 7: Icon Assets

| Icon Set | License | Bundle Path | Usage |
|---|---|---|---|
| Phosphor Icons (Regular + Bold) | MIT | `/usr/share/icons/mahina/` | System icon set (DL-051) |
| Custom Mahina Icons | Proprietary (project assets) | `/usr/share/icons/mahina/custom/` | Luna Island, logo, lpkg |

---

## Section 8: Dependency Update Policy

```
Security patches:
  Critical CVE in a runtime dependency → patch within 48 hours
  High CVE → patch within 7 days
  Medium CVE → patch within 30 days (next patch release)
  Low CVE → document and address in next minor release

Major version updates:
  A dependency MAJOR version update requires:
    1. Review for breaking changes affecting Mahina
    2. Full test suite pass
    3. Update this document
    4. DL entry if the API surface changes

Vendored library updates:
  Vendored libraries are updated at each Mahina MINOR release minimum.
  Security patches are applied immediately to vendored libraries.
```

---

## Section 9: Prohibited Dependencies

The following categories of dependencies are permanently prohibited:

```
Prohibited:
  - systemd (or any component thereof) — DL-009
  - PulseAudio (replaced by PipeWire) — DL-024
  - X11 / Xorg — DL-010
  - Any cloud-dependent SDK or library — Core Law I (locality)
  - Any telemetry or analytics library — Core Law I (privacy)
  - Any advertising or tracking library — Core Law I
  - OpenSSL for signing (use libsodium) — DL-048
  - GPG for signing (use libsodium) — DL-048
  - musl libc in v1 (deferred to v2) — DL-007
```

---

*Document: `Volume VII / 05_dependency_audit.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 1.0*
*Depends on: DL-007, DL-046, DL-048, DL-049, DL-050, DL-051*
*Must be updated: Any time a dependency is added, removed, or version-bumped*
