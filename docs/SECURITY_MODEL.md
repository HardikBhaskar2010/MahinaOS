# MahinaOS — Security Model & Capability Specification

> **Status**: Authoritative Security Baseline — Milestone M1 (Phase 2)  
> **Document Revision**: 1.0.0 (2026-09-08)  
> **Repository**: [MahinaOS](https://github.com/HardikBhaskar2010/MahinaOS)  
> **Target Subsystems**: `luna-init`, `lpkg`, `luna-ai-d`, `lgp-compositor`, `mahina-control-plane`

---

## 1. Executive Summary & Security Philosophy

The central premise of MahinaOS is that **the human user owns the machine**, and **LunaAI is a permissioned system-level partner**, not an omnipotent root supervisor.

Conventional operating systems suffer from two primary security failures:
1. **Coarse-Grained Privilege Escalation (`sudo` / root)**: Once an application or assistant has root access, it possesses total, irreversible authority over the entire filesystem, kernel, hardware, and user privacy.
2. **Optimistic Fail-Open Fallbacks**: Incomplete configurations, missing verification keys, or parsing errors frequently default to allowing execution rather than terminating safely.

MahinaOS replaces these vulnerabilities with three foundational security pillars:
1. **Tiered Identity Architecture**: Strict separation of actors into 6 distinct tiers with discrete cryptographic and operating-system boundaries.
2. **Granular Capability Matrix**: All elevated and cross-boundary actions require signed, time-limited, scoped Capability Tokens.
3. **Absolute Fail-Closed Enforcement**: If an identity cannot be verified, if a cryptographic key is missing, or if an IPC contract is violated, the system immediately aborts the operation and logs a forensic trace.

---

## 2. The 6 Identity Tiers

Every process and actor within MahinaOS executes within one of six explicit identity tiers:

```
+-------------------------------------------------------------------------+
| [Tier 0] USER (The Human)                                               |
| Supreme authority over data, approvals, and machine ownership.          |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [Tier 1] ADMINISTRATOR (Elevated Human)                                 |
| Explicit, transient escalation for system reconfiguration.               |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [Tier 2] SYSTEM (Core Platform / PID 1)                                 |
| Minimal supervisor (luna-init), kernel interface, mount manager.        |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [Tier 3] SERVICE (Background Daemons)                                   |
| Dedicated unprivileged accounts (lgp, luna-net, luna-sound, etc.).      |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [Tier 4] APPLICATION (User Software)                                    |
| Sandboxed user processes with isolated home and device views.           |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| [Tier 5] LUNAAI (Partner Intelligence)                                  |
| Unprivileged daemon (luna-ai:luna-ai), zero root, capability-mediated.   |
+-------------------------------------------------------------------------+
```

### 2.1 Tier Specifications

| Identity Tier | UID / GID Range | Boundary Definition | Allowed Capabilities | Escalation Path |
|---|---|---|---|---|
| **Tier 0: User** | UID `1000+`<br>GID `1000+` | The human owner. Owns `/home/<user>` and all personal data. | Full userland control; discretionary access to own files and devices. | Promotes to Tier 1 via biometric/passphrase authentication. |
| **Tier 1: Administrator** | UID `0` (transient)<br>or `wheel` | Temporary administrative state for hardware/system maintenance. | Modifies system configs, partitions disks, signs new system generations. | Time-limited session; auto-expires after inactivity timeout. |
| **Tier 2: System** | UID `0`<br>GID `0` | `luna-init` (PID 1) and kernel helpers. Minimal footprint. | Mounts filesystems, spawns services, handles signals, reaps zombies. | N/A (immutable PID 1). No network listeners or user shell. |
| **Tier 3: Service** | UID `100-999`<br>GID `100-999` | Dedicated service accounts (`lgp`, `luna-net`, `luna-audio`). | Scoped to specific hardware resources (e.g. `video` GID for `/dev/dri/card0`). | Cannot escalate. Sandboxed via cgroups v2 and separate user namespaces. |
| **Tier 4: Application** | UID `1000+`<br>(app sandbox) | User applications, utilities, third-party packages. | Access to display socket (`/run/lgp.sock`) and granted subdirectories. | Must request user approval via system dialog for sensitive permissions. |
| **Tier 5: LunaAI** | UID `950`<br>GID `950` (`luna-ai`) | Dedicated background intelligence daemon (`luna-ai-d`). | Telemetry read-only; LLM inference; capability requests. | **Never granted direct root**. All actions mediated by Capability Tokens. |

---

## 3. The Capability Permission Matrix

LunaAI and untrusted applications never interact directly with system management APIs. All actions outside their standard sandbox require a cryptographically signed **Capability Token**.

### 3.1 Permission Scopes

| Capability Scope | Name | Risk Level | Description | Grant Authority |
|---|---|---|---|---|
| `files.read` | Scoped Read | Low | Read access to designated user directories (e.g. `~/Documents`). | User prompt (once / session / persistent) |
| `files.write` | Scoped Write | Medium | Write or modify files in designated user directories. | User prompt with diff preview |
| `apps.launch` | Launch App | Low | Launch installed applications inside the user's desktop session. | User configuration / ambient policy |
| `packages.install` | Package Install | Medium | Install signed `.lpkg` packages into `/usr/local` or user root. | User confirmation dialog |
| `system.modify` | System Modify | High | Modify system services, hardware settings, or network configuration. | Tier 1 authentication (passphrase) |
| `security.modify` | Security Modify | Critical | Alter security policies, rotate keys, or modify user credentials. | Master Recovery Key or hardware token |
| `boot.modify` | Bootloader Modify | Critical | Update kernel command line, default boot entry, or Limine configs. | Tier 1 authentication + Generation sign |
| `source.modify` | Engineering Mode | Critical | Edit operating system source code, compile new system generation. | Explicit Engineering Mode toggle + test pass |

### 3.2 Dual-Agency Approval Flow

When LunaAI identifies an action requiring elevated capability, it follows this strict protocol:

```
[User Request / Goal]
        |
        v
[LunaAI: Synthesizes Action Plan]
        |
        v
[Control Plane: Evaluates Action vs Capability Matrix]
        |
        +---- Is Capability Already Granted & Valid?
        |         |
        |      YES: Execute action in sandbox
        |
        NO
        |
        v
[Visual Dialog: Mahina Experience (luna-island-rs / Modal)]
        |
        +-- Displays exact command / diff / target
        +-- Highlights Risk Level (Low / Medium / High / Critical)
        |
[User Decides]
        |
        +-- REJECT: Operation cancelled, zero side effects
        +-- APPROVE: Control plane issues single-use Capability Token
                |
                v
        [Action Executed via Worker]
                |
                v
        [Token Revoked / Invalidated]
```

---

## 4. Fail-Closed Invariants

MahinaOS enforces fail-closed behavior across all core subsystems. "Fallback" must never mean "allow".

### 4.1 Package Management (`lpkg`)
- **Key Requirement**: Every `.lpkg` archive must carry an Ed25519 digital signature.
- **Fail-Closed Rule**: If the system public key `/etc/luna/lpkg.pub` is missing, unreadable, or corrupted:
  ```c
  /* src/lpkg/verify.c */
  FILE *key_file = fopen(TRUSTED_KEY_PATH, "rb");
  if (!key_file) {
      fprintf(stderr,
          "lpkg: refuse: trusted public key '%s' not found.\n"
          "      Install your distribution's public key before installing packages:\n"
          "      cp <keyfile> %s\n",
          TRUSTED_KEY_PATH, TRUSTED_KEY_PATH);
      return false; /* FAIL CLOSED */
  }
  ```
- **Result**: Refusal with exit code `1`. No unverified package may ever be unpacked or executed.

### 4.2 Static Binaries & Name Service Switch (NSS)
- **Problem**: glibc's `getpwnam()`, `getgrnam()`, and `getaddrinfo()` dynamically load shared libraries (`libnss_files.so`, `libnss_dns.so`) at runtime. When statically compiled or executing in a minimal initramfs, these calls crash or silently fail.
- **Fail-Closed Rule**: PID 1 (`luna-init`), splash (`luna-splash`), and early daemons must **never** invoke NSS.
- **Implementation**:
  - `supervisor.c`: Direct linear parsing of `/etc/passwd` using `find_uid_in_file()`. If user is missing, process launch fails closed.
  - `socket_server.c`: Direct linear parsing of `/etc/group` using `lookup_video_gid()`. If `video` group is missing, socket creation fails closed or falls back safely to root GID without memory corruption.
  - Readiness probes: `inet_pton(AF_INET, "127.0.0.1", ...)` directly on `struct sockaddr_in`; `#include <netdb.h>` and `getaddrinfo` are permanently banned from static binaries.

---

## 5. Storage Security & Generational Isolation

To guarantee that system failures and self-modification errors cannot brick the machine, MahinaOS isolates state across structured Btrfs subvolumes:

```
/ (Root Btrfs Volume)
├── @generations/
│   ├── 101/ (Generation 101: Read-Only System Snapshot)
│   ├── 102/ (Generation 102: Read-Only System Snapshot — ACTIVE)
│   └── current -> 102
├── @home/ (User Data — Read/Write, Never Overwritten by Updates)
│   ├── user1/ (0700 permissions)
│   └── user2/ (0700 permissions)
├── @var/ (Transient Logs, Caches, Runtime State — Read/Write)
└── @snapshots/ (Automated pre-update user snapshots)
```

1. **Read-Only System Generations**: `/usr`, `/lib`, `/bin`, and `/sbin` in active generations are mounted read-only.
2. **Atomic Upgrades**: Updates produce generation `N+1` as a separate subvolume. If generation `N+1` fails boot validation, Limine automatically boots generation `N`.
3. **User Isolation**: User home directories are mounted with `0700` POSIX permissions. Applications and daemons run under independent UIDs and cannot traverse another user's directory without an explicit Capability Token.

---

## 6. Audit & Forensic Integrity

Every privileged action, capability grant, signature check, and authentication event is appended to an append-only system audit log:
- **Location**: `/var/log/mahina/audit.log`
- **Format**: Structured JSON Lines (timestamp, actor tier, UID, capability, target resource, outcome).
- **Integrity**: Log entries are hashed sequentially using HMAC-SHA256 with a kernel-held session key.
