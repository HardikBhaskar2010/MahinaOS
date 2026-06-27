# Architecture Review Session 6

## Project: Mahina

## Topic: AI Models, Runtime Language, Visual Identity & Package Security

---

# Session Information

**Session ID:** AR-006

**Status:** Accepted

**Project Phase:** Stage 2 → Stage 3 Readiness

**Purpose**

This Architecture Review formally resolves all five categories of pending decisions identified in the Mahina Documentation Maintenance Sprint Final Report. All decisions recorded here supersede any deprecated or draft-status entries in earlier documents. Each decision becomes a Decision Log entry (DL-046 through DL-051).

---

# Background

The previous sprint (Sprint Final Report, 2026-06-27) completed Volumes I–VII and identified the following unresolved decision categories:

1. Default AI model — DL-006 deprecated, no replacement
2. First-boot offline behavior — no documented fallback
3. lpkg signing algorithm — referenced but no formal DL entry
4. luna-ai-d implementation language — C vs Python unresolved
5. Companion reading font and icon set — both marked "Pending DL entry"

This session resolves all five categories in a single review.

---

# Decision 1 — Default LLM: Qwen2.5 3B

## Decision

**Qwen2.5 3B Q4_K_M via Ollama**

This supersedes DL-006 (Phi-3 Mini as default, status: Deprecated).

## Reasoning

Qwen2.5 3B was selected over Phi-3 Mini 3.8B and other candidates for the following reasons:

* Strong instruction following at its size class
* Smaller footprint than Phi-3 Mini at equivalent quality
* Ollama distribution available — no additional integration work
* Apache 2.0 license — fully permissive for distribution
* Runs comfortably within the 8GB RAM minimum hardware target

The model's instruction-following quality is particularly important for LUNA's personality engine, which relies on structured prompt templates. Qwen2.5 3B follows these reliably without requiring the larger 7–8B class models.

## Resource Profile

```
Model: Qwen2.5 3B (4-bit quantized, Q4_K_M)
Source: Alibaba (Apache 2.0)
Format: GGUF (via Ollama)
Quantization: Q4_K_M

Resource requirements:
  RAM:    ~2.0 GB (loaded, 4-bit quantized)
  Disk:   ~1.7 GB
  CPU:    Inference via llama.cpp (CPU or GPU)
  VRAM:   Optional — GPU offload if available (DL-026)
```

---

# Decision 2 — First-Boot Offline Behavior

## Decision

If no AI model is installed on first boot and the system is offline:

* LUNA starts immediately in **DEGRADED mode**
* No error is shown. Luna Island displays the AMBIENT state with LUNA_AMBER color
* LUNA sends a single non-blocking notification: *"AI model not installed. Run 'luna model install' when connected to set up LUNA."*
* All non-AI features of Mahina function normally
* When internet connectivity is later detected, luna-ai-d does **not** automatically download the model — the user installs manually or confirms via a notification prompt

## Reasoning

Blocking first boot on internet availability violates the principle of user control. Bundling a model in the ISO would increase the image size by over 1.7 GB, which is unacceptable for a v1 distribution. Silent auto-download without user confirmation is a privacy concern.

DEGRADED mode is a designed and tested operational state — not a failure state. Making it the first-boot fallback is architecturally correct and philosophically consistent with Control Over Possession.

---

# Decision 3 — lpkg Package Signing: Ed25519

## Decision

The `lpkg` package manager and model verification system shall use **Ed25519** as the signing algorithm for all package manifests, model manifests, and repository metadata.

## Reasoning

* Ed25519 is already referenced in the model security specification (`10_ai_models.md` — `/etc/luna/models.toml` signed manifest)
* Modern, fast, small keys (64-byte signatures, 32-byte public keys)
* No key size debates — Ed25519 has a single standardized key size
* Supported natively by OpenSSL, libsodium, and the Linux kernel's signing infrastructure
* GPG (RSA-4096) carries unnecessary complexity for a custom package manager

The lpkg signing infrastructure shall be implemented using **libsodium** for key generation, signing, and verification. GPG is not used anywhere in the Mahina toolchain.

---

# Decision 4 — luna-ai-d v1 Language: Python

## Decision

`luna-ai-d` shall be implemented in **Python** for version 1.

**Planned migration path:** Rust for version 2.

## Reasoning

* The AI daemon's bottleneck is Ollama inference latency (2–4 seconds per response) — not the daemon's own processing overhead
* Python dramatically reduces development time for the complex AI logic: personality engine, context engine, conversation management, memory engine
* Ollama's REST API is already Python-friendly — the `ollama` Python library is the reference client
* D-Bus integration is well-supported via `dbus-python` or `dasbus`
* Python's dynamic nature suits the personality engine's template system and rule-based logic
* A Python daemon with `asyncio` can comfortably handle the IO-bound workload (D-Bus events, Ollama REST calls, file reads)

This is consistent with the lpkg precedent: Python for v1 tooling, then migrate to a systems language once the architecture is proven.

## Constraints

* `luna-ai-d` must be packaged as a self-contained Python environment — no system Python dependency
* Must use `asyncio` throughout — no blocking calls on the main event loop
* Memory target: python daemon process ≤ 80 MB RAM (excluding model RAM which is owned by Ollama)
* Startup time target: luna-ai-d process ready in ≤ 2 seconds

---

# Decision 5 — Companion Reading Font: Inter

## Decision

The companion reading font for Mahina is **Inter**.

Usage domains:
* Body text in all productivity applications
* Terminal emulator UI chrome (not the monospace buffer itself)
* Settings and configuration panels
* Notification text
* Installer and first-run wizard
* Documentation viewer
* All text rendered at sizes below 18px where readability is the priority

## Reasoning

Inter was designed explicitly for screen readability at small sizes. Its large x-height, open apertures, and generous letter-spacing make it among the most legible sans-serif fonts in existence for on-screen use. It carries no visual identity that competes with Bitcount's personality — it is functionally neutral, which is exactly what a reading font should be.

* Open source (SIL OFL 1.1 license)
* Exceptionally well-hinted for all pixel densities
* Ships with Variable Font axes (weight, optical size) — a single file covers all weights
* 30+ languages supported — future internationalization ready

## Typography Split

```
Bitcount    — Display personality
              Use for: boot, login, dock labels, Luna Island, LUNA responses,
              major headings, marketing/branding contexts

Inter       — Reading clarity
              Use for: body text, labels, form fields, productivity apps,
              any text longer than a headline

JetBrains Mono — Code and terminal
              Use for: terminal buffer, code editors, all monospace contexts
              (DL-023, unchanged)
```

---

# Decision 6 — System Icon Set: Phosphor Icons

## Decision

The Mahina system icon set for v1 is **Phosphor Icons**.

## Profile

```
Name:     Phosphor Icons
License:  MIT
Style:    Line icons (Regular weight), 24×24px base grid
Scalable: 16 / 20 / 24 / 32 / 48px
Source:   https://phosphoricons.com
Format:   SVG (rendered via LunaGUI's SVG icon pipeline)
Count:    1,200+ icons — covers all system UI needs
```

## Reasoning

Phosphor Icons aligns precisely with Mahina's icon design rules (2px stroke weight, rounded corners matching radius-1, line style not solid). It is MIT-licensed with no attribution requirement, making it suitable for bundling in the distribution. The 24px base grid matches Mahina's icon grid exactly.

Custom Mahina-specific icons (Luna Island dot, LUNA expressions, lpkg status symbols) will be created as a supplementary set layered on top of Phosphor Icons. These custom icons are not Phosphor forks — they are purpose-built SVGs following the same stroke rules.

## Usage Rules

```
Phosphor Icons — Regular weight — used for:
  System UI: luna-bar, luna-shell menus, settings
  Notifications: status icons
  File manager: file type indicators
  Applications: toolbar icons

Phosphor Icons — Bold weight — used for:
  Emphasis: primary action buttons
  Alerts: warning and error indicators

Custom Mahina icons — used for:
  Luna Island AMBIENT dot (filled circle)
  LUNA expression indicators
  Mahina logo mark
  lpkg package status symbols
```

---

# New Architectural Principle — Graceful Capability Degradation

LUNA's AI capability is a layer above a fully functional operating system.

When any AI component is unavailable, the operating system is not degraded — LUNA's intelligence is temporarily unavailable.

Mahina must always be a usable, stable desktop environment without AI.

LUNA's presence is the enhancement. Not the foundation.

---

# Session Summary

Architecture Review Session 6 resolves the final pending decisions blocking Stage 3 readiness:

* Default LLM finalized — Qwen2.5 3B Q4_K_M replaces deprecated DL-006
* First-boot offline behavior — DEGRADED mode with a single notification
* lpkg signing — Ed25519 via libsodium
* luna-ai-d language — Python for v1, Rust migration for v2
* Reading font — Inter (SIL OFL 1.1)
* Icon set — Phosphor Icons (MIT, line-style, 24px)

These decisions become Decision Log entries DL-046 through DL-051.

---

# Session Status

**Status:** Accepted

**Architecture Confidence:** High

**Next Step:**

Decision Log entries DL-046 through DL-051 recorded in `decision_log.md`. Affected documents in Volume III, IV, and V updated to reflect accepted decisions.
