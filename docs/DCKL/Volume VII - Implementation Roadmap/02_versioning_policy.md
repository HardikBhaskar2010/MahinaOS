# Mahina — Versioning Policy
**Volume VII · Chapter 2**
**Classification:** Project Management — Release Engineering
**Status:** Canonical · This document governs all versioning decisions across Mahina

---

## Purpose

This document defines how Mahina assigns version numbers to the OS, its APIs, and its components. It is the authoritative reference for any versioning decision.

Inconsistent versioning erodes trust. A user who upgrades Mahina must be able to predict, from the version number alone, whether their applications will still work and whether their data is safe.

---

## OS Version Numbering

Mahina follows **Semantic Versioning (SemVer 2.0.0)**:

```
MAJOR.MINOR.PATCH[-qualifier]

Examples:
  1.0.0           — first stable release
  1.0.1           — patch: security fix
  1.1.0           — minor: new application added
  2.0.0           — major: LGP wire format changed
  0.9.0-rc.1      — release candidate 1 of 0.9.0
  0.5.0-dp        — developer preview of 0.5.0
```

### When to Increment MAJOR

MAJOR increments when the change is **incompatible with the previous version**. Specifically:

- The LGP wire protocol format changes (DL-025) — applications compiled against the old protocol will not work
- The D-Bus API surface changes incompatibly — method signatures removed or renamed
- The lpkg package format changes — old packages cannot be installed on the new version
- The luna.toml manifest schema changes in a breaking way

MAJOR increments are rare. They require an Architecture Review and a DL entry.

### When to Increment MINOR

MINOR increments when the change **adds capability without breaking existing applications**:

- A new version milestone completes (v0.5 → v0.9 → v1.0)
- A new application is added to the default install
- A new D-Bus API method is added (not changed or removed)
- A new LGP message type is added (not changed or removed)
- A new lpkg feature is added (download mirrors, for example)

### When to Increment PATCH

PATCH increments for **backward-compatible fixes**:

- Security vulnerability patched
- Crash fix
- Performance regression fixed
- Documentation-only release that corrects wrong behavior descriptions

### Qualifiers

```
-dp       Developer Preview — not stable, bugs expected
-rc.N     Release Candidate N — feature complete, final testing
(none)    Stable release
```

---

## API Versioning

### LGP Protocol Version

The LGP wire format carries an explicit version field in the `LGP_HELLO` handshake:

```c
struct lgp_hello {
    uint8_t  magic[4];    // "LGPX"
    uint16_t major;       // incompatible change → MAJOR increments
    uint16_t minor;       // additive change → MINOR increments
};
```

- **Same MAJOR** → compositor and client are compatible
- **Different MAJOR** → compositor refuses connection with `LGP_VERSION_MISMATCH`
- **Client MINOR > compositor MINOR** → compositor accepts but may not support new features the client uses

Current LGP version: **1.0** (v1.0 release)

### LunaGUI C API

```
LunaGUI follows the same MAJOR.MINOR scheme as the OS.

Guarantees:
  - No symbol removed within a MAJOR version
  - No function signature changed within a MAJOR version
  - New symbols added in MINOR versions are additive only

Exported symbols are tagged: LUNAGUI_API void luna_widget_show(LunaWidget *w);
The luna-abi-check tool validates compiled binaries against the current ABI.
```

### D-Bus API

```
D-Bus interface names carry a version suffix when a breaking change occurs:

  org.mahina.luna     — v1 interface (stable from v1.0)
  org.mahina.luna.v2  — future: added when v1 interface is superseded

Within a named interface:
  - Methods are never removed (marked deprecated, then removed in next MAJOR)
  - Signal signatures are never changed
  - New methods and signals are added freely
```

### lpkg Package Format

```
Package format version is declared in luna.toml:

  [package]
  format_version = 1

lpkg checks format_version at install time:
  - format_version == current_version → install proceeds
  - format_version > current_version → error: "lpkg upgrade required"
  - format_version < current_version → install proceeds (backward compatible)
```

---

## Component Versioning

Individual Mahina components (luna-init, lgp-compositor, luna-ai-d, etc.) are versioned independently:

```
Component version format:  COMPONENT_MAJOR.COMPONENT_MINOR.PATCH

Examples:
  luna-init 1.2.0
  lgp-compositor 1.0.3
  luna-ai-d 1.1.0

Rules:
  - Component version is independent of OS version
  - Component versions are tracked in their own lpkg manifests
  - The OS release pins specific component versions (via locked dependency manifest)
  - Components must not be independently updated outside of lpkg
    (prevents version mismatch between components)
```

---

## Versioning for the v0.x Pre-Release Series

During the pre-release period (v0.1 through v0.9), versioning is relaxed:

```
v0.x Pre-Release Rules:
  - No API stability guarantees between v0.x releases
  - Applications built for v0.5 are not guaranteed to work on v0.9
  - The LGP protocol may change between v0.x releases
  - Version numbers in this series communicate progress, not compatibility
  - All pre-release versions are labeled -dp (developer preview)
    unless they are an explicit release candidate for v1.0

"Stable" begins at v1.0.0.
```

---

## Long-Term Support (LTS) Policy

```
v1.0  LTS candidate — security patches for 2 years from release date
      If v2.0 ships, v1.x continues receiving security patches for 1 additional year.

v1.5  Standard release — security patches for 1 year
      (Unless designated as LTS at release time)

v2.0  LTS candidate — evaluated at time of release
```

LTS designation is announced at release. It is not retroactive.

---

## What Never Changes Within a MAJOR Version

The following are **frozen** between MAJOR releases:

| Component | What is frozen |
|---|---|
| LGP wire format | TLV structure, message opcodes, field layout |
| D-Bus interface names | `org.mahina.*` names stay constant |
| D-Bus method signatures | Parameters and return types cannot change |
| lpkg package format | `luna.toml` required fields and types |
| Filesystem layout | `/etc/luna/`, `/var/log/luna-init/`, `~/.luna/` structure |
| Semantic color codes | `LUNA_BLUE`, `LUNA_GREEN`, etc. — never renamed or removed |

---

*Document: `Volume VII / 02_versioning_policy.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 1.0*
*Depends on: Volume VI/07_release_process.md, DL-025 (LGP format)*
*Informs: All engineering decisions involving API design and compatibility*
