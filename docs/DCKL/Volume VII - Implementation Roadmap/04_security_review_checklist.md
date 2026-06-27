# Mahina — Security Review Checklist
**Volume VII · Chapter 4**
**Classification:** Project Management — Security Engineering
**Status:** Canonical · Must be completed before every stable release

---

## Purpose

This document is the pre-release security checklist for Mahina. It must be completed — item by item, with evidence — before any stable release is published.

Security is not a feature that gets added at the end. This checklist verifies that all security properties defined in the architecture documents are actually present in the release candidate.

> **Rule:** No item on this checklist may be marked complete without evidence. "Looks fine" is not evidence. A test result, a log output, or an audit report is evidence.

---

## Security Properties Required for v1.0

These properties are the security foundation of Mahina. They must all hold before v1.0 ships.

```
S1 — Process isolation via AppArmor
S2 — Least-privilege execution (no unnecessary root processes)
S3 — Package integrity via Ed25519 signing (DL-048)
S4 — AI model integrity via SHA-256 hash verification
S5 — Memory privacy (LUNA memory readable only by luna-ai-d)
S6 — No cleartext credentials anywhere on disk
S7 — Firewall active (nftables) with deny-by-default inbound
S8 — External input parsers fuzz-tested (Volume VI/05)
S9 — No world-readable sensitive files in /etc
S10 — No deprecated or known-vulnerable cryptographic algorithms
```

---

## Section 1: Process Isolation

### 1.1 — AppArmor Status

**Requirement:** All system daemons run under enforcing AppArmor profiles.

```
Verification:
  Run: sudo aa-status
  Expected output: All Mahina processes listed under "processes in enforce mode"
  No process should be listed under "processes in complain mode" in a release build.

Evidence required: Output of `aa-status` on the release candidate build.
```

- [ ] `luna-init` — enforce profile present and active
- [ ] `lgp-compositor` — enforce profile present and active
- [ ] `luna-ai-d` — enforce profile present and active
- [ ] `luna-shell` — enforce profile present and active
- [ ] `luna-dock` — enforce profile present and active
- [ ] `luna-bar` — enforce profile present and active
- [ ] `luna-island` — enforce profile present and active
- [ ] `luna-notif` — enforce profile present and active
- [ ] `luna-terminal` — enforce profile present and active
- [ ] `luna-files` — enforce profile present and active
- [ ] `luna-settings` — enforce profile present and active
- [ ] `lpkg` daemon — enforce profile present and active
- [ ] `ollama` — enforce profile present and active

### 1.2 — Root Process Audit

**Requirement:** The only root processes at runtime are luna-init (PID 1) and hardware-required kernel services.

```
Verification:
  Run: ps aux | awk '$1 == "root"' | grep -v '\[kernel\]'
  Expected: luna-init only. All other Mahina processes run as the logged-in user.

Evidence required: Output of the ps command above after full system boot.
```

- [ ] No application-level process runs as root
- [ ] luna-ai-d runs as user (not root)
- [ ] lgp-compositor runs as user (not root)
- [ ] lpkg daemon: runs as root only during system-wide installs, immediately drops privileges after

---

## Section 2: Package and Model Integrity

### 2.1 — Ed25519 Package Signing (DL-048)

**Requirement:** Every official package and repository index is signed with the Mahina release Ed25519 key.

```
Verification:
  Run: lpkg install <test-package> --verbose
  Expected output must include: "Ed25519 signature verified."
  Expected output must NOT include any signature bypass or skip messages.

Test with a tampered package:
  Modify one byte of an official .lpkg archive.
  Run: lpkg install <tampered-package>
  Expected: "ERROR: Package signature verification failed. Installation aborted."
```

- [ ] `lpkg install` verifies Ed25519 signature before any file extraction
- [ ] Tampered package is rejected with a clear error
- [ ] Unsigned package (no .sig file) is rejected with a clear error
- [ ] Public key location verified: `/etc/luna/keys/mahina-official.pub` exists and is the correct key
- [ ] Repository index signature verified: `lpkg update` verifies index signature

### 2.2 — AI Model Hash Verification

**Requirement:** Model downloads are verified against the SHA-256 hash from the signed model manifest.

```
Verification:
  Run: luna model install qwen2.5:3b --verbose
  Expected output: "SHA-256 hash verified: [hash]"

Test with corrupted download:
  Interrupt and partially corrupt a downloaded model blob.
  Expected: "ERROR: Model hash verification failed. Download aborted. File removed."
```

- [ ] Model download verified against SHA-256 from signed manifest
- [ ] Corrupted model triggers hash mismatch error and file deletion
- [ ] Model file is never loaded if hash verification fails

---

## Section 3: Memory and Data Privacy

### 3.1 — LUNA Memory Permissions (DL-023)

**Requirement:** `~/.luna/memory/` is accessible only by the owning user. No group read. No world read.

```
Verification:
  Run: stat ~/.luna/memory/
  Expected: mode 700 (drwx------)

  Run: ls -la ~/.luna/memory/
  All files inside must be 600 (user read/write only).
  Databases (.db files) must be 600.
  Keys and encrypted summaries must be 600.
```

- [ ] `~/.luna/memory/` is `chmod 700`
- [ ] All files inside `~/.luna/memory/` are `chmod 600`
- [ ] luna-ai-d is the only process that opens `~/.luna/memory/*.db`
- [ ] No other process (luna-shell, luna-terminal, etc.) has file descriptors to these files

### 3.2 — observe.toml Privacy Compliance

**Requirement:** LUNA only observes what the user explicitly enabled during the installer privacy screen.

```
Verification:
  Check: ~/.luna/config/observe.toml
  Verify: only enabled fields have observe = true
  Verify: disabled fields have observe = false (not absent — must be explicit)

Test: disable "window title observation" in settings.
  Confirm: luna-ai-d no longer includes window titles in context data.
  Verify via: luna debug context (shows what LUNA can currently see)
```

- [ ] `observe.toml` accurately reflects the user's installer choices
- [ ] Disabling observation in Settings immediately takes effect (no restart required)
- [ ] LUNA cannot observe clipboard contents under any configuration
- [ ] LUNA cannot observe keystroke content under any configuration

---

## Section 4: Network Security

### 4.1 — Firewall Verification

**Requirement:** nftables firewall is active with deny-by-default inbound policy.

```
Verification:
  Run: sudo nft list ruleset
  Expected:
    - Default INPUT policy: drop
    - RELATED, ESTABLISHED traffic: accepted
    - ICMP: accepted (ping works)
    - All other inbound: dropped

  External test:
    From another machine: nmap -sS <mahina-ip>
    Expected: All ports closed or filtered (no open ports visible)
```

- [ ] nftables starts before NetworkManager (service dependency order)
- [ ] Default inbound policy: DROP
- [ ] No inbound ports open by default
- [ ] Outbound traffic unrestricted (user-initiated connections work)
- [ ] Firewall rules survive reboot (persisted in `/etc/nftables.conf`)

### 4.2 — Ollama Network Exposure

**Requirement:** Ollama listens on `127.0.0.1:11434` only. It must NOT be accessible from other machines.

```
Verification:
  Run (on Mahina): ss -tlnp | grep 11434
  Expected: 127.0.0.1:11434 only. NOT 0.0.0.0:11434.

  External test (from another machine):
    curl http://<mahina-ip>:11434/api/version
    Expected: Connection refused or timeout.
```

- [ ] Ollama bound to `127.0.0.1:11434` only
- [ ] Ollama port not reachable from external hosts
- [ ] Ollama service definition enforces bind address: `OLLAMA_HOST=127.0.0.1`

### 4.3 — No Cleartext Network Credentials

**Requirement:** No credentials, API keys, or authentication tokens are transmitted or stored in cleartext.

- [ ] No hardcoded credentials in any source file (scan: `grep -rn "password\|api_key\|secret" src/`)
- [ ] Repository index downloaded over HTTPS (not HTTP)
- [ ] Model downloads use HTTPS
- [ ] NetworkManager stores Wi-Fi credentials in `/etc/NetworkManager/system-connections/` with `chmod 600`

---

## Section 5: Input Parser Security

### 5.1 — Fuzz Test Results

**Requirement:** All external input parsers have been fuzz-tested and no crashes remain unfixed. (Volume VI / 05_testing_standards.md)

Parsers requiring fuzz coverage:

- [ ] LGP TLV parser — no crashes in 5 minutes of AFL++ fuzzing
- [ ] luna.toml package manifest parser — no crashes
- [ ] observe.toml config parser — no crashes
- [ ] TOML config parser (general) — no crashes
- [ ] Display mode negotiation parser — no crashes
- [ ] D-Bus message parser — no crashes

```
Verification:
  Run: make test-fuzz (all parsers, 5 minutes each minimum)
  Expected: No new crashes. All known corpus inputs pass.
  Evidence: AFL++ run log showing 0 crashes per parser.
```

### 5.2 — Integer Overflow Review

**Requirement:** All parsers that read length fields from external data must validate lengths before allocation.

```
Code review check for every parser:
  - Length fields read from wire data are bounds-checked before malloc/alloc
  - No cast from signed to unsigned without range check
  - Buffer size calculations use size_t and check for overflow
```

- [ ] LGP TLV length field: validated (`len < MAX_MESSAGE_SIZE`)
- [ ] lpkg archive extraction: file sizes validated before extraction
- [ ] luna.toml string values: length-bounded before copying

---

## Section 6: Cryptographic Standards

### 6.1 — Algorithm Audit

**Requirement:** No deprecated or broken cryptographic algorithms are used anywhere.

```
Forbidden algorithms (must not appear in any Mahina component):
  MD5         — collision attacks known
  SHA-1       — deprecated for security use
  DES / 3DES  — deprecated
  RSA < 2048  — key size insufficient
  RC4         — broken stream cipher
  ECB mode    — deterministic block cipher mode
```

| Component | Algorithm Used | Status |
|---|---|---|
| lpkg package signing | Ed25519 via libsodium | ✅ Approved |
| Model manifest signing | Ed25519 via libsodium | ✅ Approved |
| Model download integrity | SHA-256 | ✅ Approved |
| Memory encryption | AES-256-GCM (libsodium) | ✅ Approved |
| OS release signing | Ed25519 | ✅ Approved |
| HTTPS (system-wide) | TLS 1.2+ (libcurl default) | ✅ Approved |

- [ ] All cryptographic algorithm uses audited against this table
- [ ] No MD5 or SHA-1 usage found in source code
- [ ] libsodium version pinned and not known-vulnerable

---

## Section 7: Pre-Release Final Scan

### 7.1 — Secret Scanner

```
Run: git secrets --scan (or truffleHog / gitleaks)
Expected: Zero secrets found in any committed file
```

- [ ] No hardcoded passwords in source code
- [ ] No API keys committed to the repository
- [ ] No private keys committed to the repository (only public keys in `/distribution/keys/`)

### 7.2 — Dependency Vulnerability Scan

- [ ] All C library dependencies checked against CVE database (see `04_dependency_audit.md`)
- [ ] All Python dependencies (luna-ai-d) checked: `pip-audit` shows no critical CVEs
- [ ] Ollama version pinned to a release with no known critical CVEs

### 7.3 — Installer Security

- [ ] Installer partition overwrite requires typing the disk identifier (not just "yes")
- [ ] Installer creates user password with minimum strength (8+ characters)
- [ ] Installer does not transmit any data over the network without user action
- [ ] First-boot AI model download: user explicitly clicks "Download" — no auto-download (DL-047)

---

## Sign-Off

This checklist must be signed off by the Principal Engineer before publication:

```
Checklist completed for release: _______________________
Date: _______________________
Engineer: Hardik Bhaskar (Luna Kitsune)
Signature: _______________________

All items above marked [x] with evidence documented.
No item is marked [x] without verifiable evidence.
```

---

*Document: `Volume VII / 04_security_review_checklist.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 1.0*
*Depends on: DL-048 (Ed25519), DL-023 (memory encryption), Volume VI/05_testing_standards.md*
*Must be completed: Before every stable release. Before every RC candidate publication.*
