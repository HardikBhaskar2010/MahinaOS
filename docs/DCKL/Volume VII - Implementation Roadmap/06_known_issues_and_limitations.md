# Mahina v1.0 — Known Issues & Limitations
**Volume VII · Chapter 6**
**Classification:** Project Management — Release Transparency
**Status:** Living Document · Updated with every release

---

## Purpose

This document is the honest statement of what Mahina v1.0 does not do, does not do well, or does not do yet. It is published alongside the release.

Transparency about limitations is not weakness. It is how users trust a project. A user who discovers a limitation after installing an OS is frustrated. A user who knew about it before installing made an informed choice.

> **Rule:** Do not hide limitations. Document them. Users who need something we don't do yet can plan around it. Users who discover undocumented limitations become former users.

---

## v1.0 Scope Statement

> Mahina v1.0 is a **foundation release**. It is complete, stable, and installable — but it is intentionally limited in scope. The goal is to ship a solid base, not an overpromised experience.

The features below are not bugs. They are deliberate scope decisions that reflect the v1.0 philosophy: ship less, ship right.

---

## Known Limitations by Category

### Hardware

| Limitation | Detail | Workaround | Target |
|---|---|---|---|
| NVIDIA GPU: no proprietary driver | Nouveau open-source driver only. LLM GPU acceleration unavailable on NVIDIA hardware. | Use CPU inference (slower but functional) | v1.5 |
| Secure Boot | Not supported. Secure Boot must be disabled in UEFI firmware settings. | Disable Secure Boot | v2.0 |
| Legacy BIOS | Limine requires UEFI. Systems without UEFI cannot boot Mahina. | Purchase a UEFI-capable machine or use QEMU | v1.5 |
| Hi-DPI > 2x scaling | Display scaling not validated above 2x. UI elements may be too small on very high DPI displays. | Set OS to 1x or 2x scaling | v1.5 |
| Broadcom Wi-Fi | Many Broadcom Wi-Fi chips require proprietary firmware not included in the default image. | Install firmware via lpkg post-boot | v1.5 |
| ARM64 / other architectures | x86-64 only | No workaround in v1.0 | v2.0 (investigation) |

### AI / LUNA

| Limitation | Detail | Workaround | Target |
|---|---|---|---|
| LUNA has no personality in v1 | LUNA does not express emotion, curiosity, or character in v1. Responses are functional, not expressive. | This is a design decision, not a bug. Personality arrives in v2.0. | v2.0 |
| No voice input or output | TTS and STT are not implemented. LUNA cannot speak or listen in v1. | Text-only interaction | v1.5 |
| No vision | LUNA cannot see the screen, analyze images, or read documents. | No workaround | v2.0 |
| No automation | LUNA cannot take autonomous actions (run commands, manage files, open applications). | No workaround | v2.0 |
| No predictive behavior | LUNA does not learn from patterns or predict what you will do next. | No workaround | v2.0 |
| Memory is limited | LUNA retains a session summary between reboots but does not have long-term persistent memory. | No workaround | v2.0 |
| Model download requires internet | First-run model install requires internet access (~1.7 GB download). | Use offline mode (LUNA in DEGRADED mode) until connectivity is available | Permanent (by design, DL-047) |
| NVIDIA: no GPU LLM acceleration | LLM inference on NVIDIA hardware uses CPU only in v1.0. | Use AMD or Intel GPU hardware for GPU acceleration | v1.5 |
| First token latency | On minimum hardware (i5, no GPU): 3–4 seconds to first token from Qwen2.5 3B. | Use recommended hardware (16 GB RAM, modern CPU) | Ongoing optimization |

### Desktop Environment

| Limitation | Detail | Workaround | Target |
|---|---|---|---|
| Single workspace only | Multiple workspaces (virtual desktops) are not implemented. | Use window management to organize windows | v1.5 |
| No live theme switching | Changing the theme requires a session restart. Luna Dark is the only theme in v1.0. | Live theme switching planned for v1.5 | v1.5 |
| No tiling window management | Only floating windows. No automatic tiling layouts. | Manual window placement | v1.5 |
| No HiDPI mixed-density monitors | Multi-monitor setups with different DPI are not validated. | Use monitors with the same DPI | v1.5 |
| Limited multi-monitor support | Compositor supports multiple outputs but the layout is not configurable in the Settings UI. | Edit display config directly | v1.5 |
| No window snapping | Drag-to-snap window positioning not implemented. | Manual positioning | v1.5 |

### Package Management

| Limitation | Detail | Workaround | Target |
|---|---|---|---|
| No Flatpak or AppImage support | lpkg installs native Mahina packages only. Third-party app compatibility is limited. | Build applications for the lpkg format | v1.5 |
| No AUR-equivalent | No community package repository in v1.0. Only the official first-party repository. | Build and install packages manually | v1.5 |
| Simple dependency resolution | lpkg uses greedy dependency resolution. Complex dependency trees with conflicts may not resolve correctly. | Manually install conflicting dependencies | v1.5 |

### Applications

| Limitation | Detail | Workaround | Target |
|---|---|---|---|
| No web browser | No browser ships with Mahina v1.0. | Install via lpkg (if a compatible browser package exists) | v1.5 |
| No office suite | No word processor or spreadsheet application. | Use terminal applications or install via lpkg | v1.5 |
| No media player | Audio and video playback applications not included. | Use terminal-based players (mpv, etc.) via lpkg | v1.5 |
| No email client | No email application. | Use a terminal mail client via lpkg | v1.5 |
| luna-files: basic feature set | File manager supports browse, open, rename, delete, copy. No archive extraction, no remote filesystems, no advanced filtering. | Use the terminal for advanced file operations | v1.5 |

### Security

| Limitation | Detail | Workaround | Target |
|---|---|---|---|
| No full-disk encryption | The root filesystem is unencrypted. Only LUNA memory (`~/.luna/memory/`) is encrypted. | Use an encrypted external storage device for sensitive data | v1.5 |
| Secure Boot not supported | Cannot run with Secure Boot enabled. | Disable Secure Boot | v2.0 |
| NVIDIA driver: security implications | Nouveau driver has a different security surface than the proprietary driver. | Prefer AMD or Intel hardware for security-critical deployments | v1.5 |

### Installer

| Limitation | Detail | Workaround | Target |
|---|---|---|---|
| No dual boot | Cannot install alongside Windows or another Linux distro in v1.0. | Use a dedicated drive for Mahina | v1.5 |
| No full-disk encryption option | Installer does not offer LUKS encryption. | Not available in v1.0 | v1.5 |
| No OEM mode | Installer cannot pre-configure images for hardware vendors. | Not available in v1.0 | v2.0 |
| Manual partitioning: basic | Manual partition editor exists but is not fully specified. | Use automatic partitioning mode | v1.5 |

---

## Performance Baseline

These are the realistic performance numbers on **minimum specified hardware** (Intel Core i5 8th gen, 8 GB RAM, SATA SSD, no GPU):

```
Boot to login screen:         4–6 seconds
Login to interactive desktop: 2–3 seconds
luna-ai-d startup (presence): ~2 seconds
LLM first token (warm):       2–3 seconds
LLM first token (cold start): 4–6 seconds (model load from SATA SSD)
LLM sustained generation:     2–3 tokens/second (CPU only)
System RAM at idle:           ~1.2–1.5 GB (without LLM loaded)
System RAM with LLM loaded:   ~3.2–3.5 GB
Compositor idle CPU:          ~1–2%
```

On **recommended hardware** (Ryzen 5 5600X, 16 GB RAM, NVMe SSD, AMD RX 6700 XT):

```
Boot to login screen:         2–3 seconds
LLM first token (warm):       <1 second (GPU)
LLM sustained generation:     15–25 tokens/second (GPU)
```

---

## What v1.5 Will Improve

The following improvements are **planned for v1.5** (not guaranteed, but committed to investigation):

- Multiple workspaces
- Live theme switching and additional themes
- NVIDIA GPU driver support (proprietary driver packaging)
- Dual boot support in the installer
- Full-disk encryption (LUKS) option in the installer
- Flatpak support in lpkg
- Hi-DPI calibration improvements
- Better multi-monitor layout configuration
- Window snapping and tiling options
- Expanded application library

## What v2.0 Will Bring

v2.0 is "The Living OS" — where LUNA becomes a defining part of the experience:

- Full personality engine and emotional expressiveness
- Advanced context engine and long-term memory
- Voice input and output (TTS/STT)
- Vision capabilities
- Automation engine (LUNA takes actions)
- Predictive behavior and proactive suggestions
- Rich companion behaviors

---

## Reporting Issues

If you discover an issue not listed here:

1. Check if it's a known issue in the table above
2. Check the GitHub Issues tracker for existing reports
3. If not found, file a new issue with:
   - Mahina version (`mahina --version`)
   - Hardware (`lscpu`, `lsmem`, `lspci`)
   - Steps to reproduce
   - Expected behavior vs. actual behavior
   - Log output if available (`/var/log/luna-init/runtime.log`)

---

*Document: `Volume VII / 06_known_issues_and_limitations.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 1.0*
*This document is updated with every release. Check the version in the release notes to confirm you are reading the correct edition.*
