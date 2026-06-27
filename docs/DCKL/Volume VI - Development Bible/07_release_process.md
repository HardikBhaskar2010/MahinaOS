# Mahina — Release Process
**Volume VI · Chapter 07**
**Classification:** Development Bible — Distribution
**Status:** Canonical · This document governs how Mahina releases are built, signed, and published

---

## Purpose

This document defines the release process for Mahina — from a passing test suite to a signed ISO available to the public.

It answers: how does code become a release? Who signs it? Where does it go? What can go wrong?

---

## Release Types

Mahina uses three release types:

```
Developer Preview (DP)
  Stage: Any stage
  Audience: Contributors, technical testers, documented issue reporters
  Stability: May have known regressions. Not for production use.
  Version format: 0.X.0-dp (e.g., 0.2.0-dp)
  ISO name: mahina-0.2.0-dp-x86_64.iso

Release Candidate (RC)
  Stage: Stage 5+ (installer complete)
  Audience: Early adopters willing to report issues
  Stability: Feature-complete for a defined scope. Known issues documented.
  Version format: 0.X.0-rc.N (e.g., 0.5.0-rc.1)
  ISO name: mahina-0.5.0-rc.1-x86_64.iso

Stable Release (v)
  Stage: Stage 5 fully verified
  Audience: General users
  Stability: All Stage 5 exit criteria passed. No P0/P1 bugs open.
  Version format: 1.0.0 (semantic versioning)
  ISO name: mahina-1.0.0-x86_64.iso
```

---

## Version Numbering

Mahina follows **Semantic Versioning (SemVer)**:

```
MAJOR.MINOR.PATCH[-qualifier]

MAJOR — Incremented when backward-incompatible changes are made to:
         - The LGP wire format (DL-025)
         - The D-Bus API surface
         - The lpkg package format

MINOR — Incremented when new features are added in a backward-compatible manner.
         - A new Stage is completed
         - A new application is added to the default install

PATCH — Incremented for backward-compatible bug fixes only.
         - Security patches
         - Crash fixes
         - Performance regressions

v1.0.0 represents: all Stage 5 exit criteria passed and the system is
considered suitable for daily use as a primary operating system.
```

---

## Release Build Process

### Step 1: Pre-Release Verification

Before beginning a release build, all of the following must be true:

```
[ ] All Stage N exit criteria met (per 06_milestones.md)
[ ] Full CI pipeline passes: make test-unit && make test-integration
[ ] No open P0 bugs (system is unusable or data loss possible)
[ ] No open P1 bugs (major feature completely broken)
[ ] CHANGELOG.md is updated with all changes since previous release
[ ] Version number decided and agreed (no "we'll figure it out later")
```

If any item is unchecked, the release does not proceed.

### Step 2: Build the ISO

```bash
# Clean build environment — no cached artifacts
make clean

# Full release build (optimized, no debug symbols)
make release VERSION=1.0.0

# Output artifacts:
#   build/release/mahina-1.0.0-x86_64.iso        — bootable ISO
#   build/release/mahina-1.0.0-x86_64.iso.sha256  — SHA-256 checksum
#   build/release/mahina-1.0.0-manifest.toml       — package manifest
```

Release builds are compiled with:
```
CFLAGS = -O2 -DNDEBUG -march=x86-64 -mtune=generic
```

No sanitizers. No debug symbols. No assertions.

### Step 3: Sign the Release Artifacts

All release artifacts are signed with Ed25519 (DL-048):

```bash
# Sign the ISO
luna-sign --key /path/to/mahina-release.key \
          --input mahina-1.0.0-x86_64.iso \
          --output mahina-1.0.0-x86_64.iso.sig

# Sign the package manifest
luna-sign --key /path/to/mahina-release.key \
          --input mahina-1.0.0-manifest.toml \
          --output mahina-1.0.0-manifest.toml.sig
```

The `mahina-release.key` private key is:
- Never committed to the repository
- Never stored on a networked machine during signing
- Backed up in at least two physically separate offline locations

The corresponding public key (`mahina-release.pub`) is:
- Committed to the repository at `/distribution/keys/mahina-release.pub`
- Bundled in every Mahina installation at `/etc/luna/keys/mahina-official.pub`
- Published on the Mahina website

### Step 4: Verify the Signed Artifacts

Before publishing, independently verify the signatures on a clean machine:

```bash
luna-verify --key /distribution/keys/mahina-release.pub \
            --input mahina-1.0.0-x86_64.iso \
            --sig mahina-1.0.0-x86_64.iso.sig
# Expected output: "Signature valid. mahina-1.0.0-x86_64.iso is authentic."
```

If verification fails: do not publish. Investigate signing failure.

### Step 5: Physical Hardware Test

Before publishing any release, boot it on **at least 3 different physical machines**:

```
Required hardware diversity for testing:
  - At least 1 Intel CPU system
  - At least 1 AMD CPU system
  - At least 1 laptop (test trackpad via libinput, DL-032)
  - At least 1 system without a GPU (integrated graphics only)

Minimum tests on each machine:
  [ ] Boots to login screen without kernel panic
  [ ] Graphics render at correct resolution
  [ ] Mouse and keyboard input work
  [ ] Network connects (DHCP)
  [ ] luna-ai-d starts in READY or DEGRADED state
  [ ] luna-terminal opens and accepts input
```

### Step 6: Publish

```
Upload to GitHub Releases:
  - mahina-X.X.X-x86_64.iso
  - mahina-X.X.X-x86_64.iso.sig
  - mahina-X.X.X-x86_64.iso.sha256
  - mahina-X.X.X-manifest.toml
  - mahina-X.X.X-manifest.toml.sig

Publish release notes:
  - What's new (from CHANGELOG.md)
  - Known issues (from open P2/P3 bugs)
  - Upgrade instructions (if upgrading from a previous version)
  - Verification instructions (how to verify the ISO signature)

Post announcement to:
  - GitHub Discussions
  - Project social media channels
```

---

## Bug Priority Levels

```
P0 — Critical / Show-stopper
  Definition: The system is unusable or data loss is possible.
  Examples:   Kernel panic on boot, filesystem corruption, installer destroys wrong disk.
  Response:   All work stops. Fix before any release.

P1 — Major
  Definition: A core feature is completely broken with no workaround.
  Examples:   Compositor crashes after 10 minutes, luna-ai-d never starts,
              lpkg fails to install any package.
  Response:   Fix before stable release. May be accepted in Developer Preview with documentation.

P2 — Minor
  Definition: A feature is degraded or has an inconvenient workaround.
  Examples:   Luna Island occasionally shows wrong color, a specific GPU has poor performance.
  Response:   Fix within 2 minor releases. Document in known issues.

P3 — Cosmetic / Nitpick
  Definition: Visual or UX issues that do not affect functionality.
  Examples:   Incorrect font weight in one dialog, icon misaligned by 1px.
  Response:   Fix when time allows. Track in issue tracker.
```

---

## CHANGELOG Format

The CHANGELOG.md file follows the **Keep a Changelog** format:

```markdown
# Changelog

All notable changes to Mahina are documented in this file.

## [Unreleased]

### Added
- Luna Island now responds to scroll wheel gestures

### Fixed
- Fixed lgp-compositor crash on resize of maximized window

### Changed
- Default model changed from Phi-3 Mini to Qwen2.5 3B (DL-046)

## [0.2.0-dp] — 2026-07-15

### Added
...
```

Every PR must include a CHANGELOG entry in the `[Unreleased]` section unless it is:
- A documentation-only change
- A test-only change
- A refactor with no behavior change

---

## Hotfix Process

A hotfix addresses a P0 bug in a released version without waiting for the next planned release.

```
Hotfix process:
  1. Create a branch from the release tag: git checkout -b hotfix/1.0.1 v1.0.0
  2. Apply the minimal fix — no new features, no refactoring
  3. Increment PATCH version: 1.0.0 → 1.0.1
  4. Full test suite must pass
  5. Physical hardware test on 2 machines minimum
  6. Sign and publish: follow Steps 3–6 of the release process
  7. Merge the hotfix back to main: git cherry-pick
```

---

## Release Decision Authority

The release decision — "is this ready to publish?" — rests with the Principal Engineer (Hardik Bhaskar).

No release may be published without explicit sign-off.

This includes automated CI releases — the pipeline may build the artifact but may not publish it without explicit approval.

---

*Document: `Volume VI / 07_release_process.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 1.0*
*Depends on: 04_benchmarks.md, 06_milestones.md, DL-048 (Ed25519 signing)*
*Informs: Distribution infrastructure, GitHub Actions pipeline*
