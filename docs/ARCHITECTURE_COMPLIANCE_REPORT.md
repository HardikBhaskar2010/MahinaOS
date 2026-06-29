# ARCHITECTURE_COMPLIANCE_REPORT.md — Mahina OS

**Question this report answers:** Does the implementation match the
documented architecture, the Core Laws, the Decision Log, and the
Architecture Reviews? Where it doesn't, is the deviation a known/accepted gap
or an unflagged contradiction?

**Method:** Every claim in `PROJECT_STATE.md`, `AI_CONTEXT.md`,
`ARCHITECTURE_CONSISTENCY_REPORT.md`, and `REPOSITORY_HEALTH_REPORT.md` that
could be checked against source was checked against source. Findings below
are organized by subsystem; §1 is the most important section in this report
and should be read first.

---

## 1. The repository's own status documents disagree with each other and with the code

All four files in `docs/System Prompts/` carry the same generation date
(**2026-06-28**) and are presented as canonical (`AI_CONTEXT.md`: *"Read this
file before writing any code"*; `PROJECT_STATE.md`: *"Canonical document"*).
Despite sharing a date, they make **mutually contradictory factual claims
about the same code**, and three of the four are demonstrably stale against
the code actually in the repository as of this audit (2026-06-29). This is
the single most important finding of this audit, because it means an AI
coding agent or new contributor following `AI_CONTEXT.md`'s instruction to
treat these documents as ground truth would be acting on false premises for
at least seven distinct, checkable facts.

| Claim | `PROJECT_STATE.md` | `AI_CONTEXT.md` | `ARCH_CONSISTENCY_REPORT.md` | `REPO_HEALTH_REPORT.md` | **Ground truth (verified in source)** |
|---|---|---|---|---|---|
| `run_user`/`run_group` enforced via setuid/setgid? | ❌ Not enforced (blocker #5) | — | Says **both** ✅ "fully consistent" (§1) *and* ❌ "all run as root" (§4) in the same document | ❌ Not implemented (weakness + recommendation #6) | **✅ Implemented.** `supervisor.c::spawn_service()` calls `getgrnam`/`setgid` then `getpwnam`/`setuid`, correct order, before `execve()`. |
| Cgroup assignment implemented? | not mentioned as done | — | Says ✅ implemented (Service Supervisor section) *and* ❌ "documented, not implemented" (§6) in the same document | Says ✅ "Resolved" (Risk #2) | **✅ Implemented.** `supervisor.c::spawn_service()` parent branch creates `/sys/fs/cgroup/luna-system.slice/<svc>.service` and writes `cgroup.procs`; `mount.c::mount_early()` mounts `cgroup2`. |
| Supervisor blocking or async? | — | — | Says ✅ "completely consistent... async state machine" *and* lists "blocking supervisor" as unresolved Architecture Drift, in the same document | Says ✅ "Resolved... refactored to async state machine" (Risk #1) *but also* lists "Implement async supervisor startup" as an outstanding Phase-4 recommendation (#7), in the same document | **✅ Async.** `supervisor_start_all()` only sets state and calls `supervisor_pump()` once; ongoing progress is driven by a 100ms `timerfd` in `main.c`'s epoll loop, not a blocking loop. |
| `luna-splash` fallback path bug (`./build/luna-splash` vs `./build/luna-splash/luna-splash`)? | ❌ Bug present (blocker #4) | — | not directly addressed | ❌ Bug present (Weakness + recommendation #3) | **✅ Already fixed.** `splash.c` line 51 uses `./build/luna-splash/luna-splash`. |
| Block-character MAHINA logo rendered? | ❌ Falls back to ASCII text | — | not directly addressed | not directly addressed | **✅ Implemented.** `render.c::render_logo()` does custom UTF-8 box-drawing-character parsing and pixel rendering. |
| `luna_log_switch_to_runtime()` called? | ❌ Never called (blocker #7) | — | ❌ "Broken Reference... never called" (§9) | ❌ "never called from main.c" (Weakness) | **Called, but doesn't work.** `main.c` Stage 5 *does* call it — but the function never actually opens the runtime log fd (see `CODE_AUDIT_REPORT.md` §2), so the *effect* the four documents worried about (logs not reaching the runtime log file) is real, just misdiagnosed by all four as "never called." |
| `.github/workflows/build.yml` exists and is stale? | ❌ Listed as present, "should be deleted" | not directly addressed | ❌ Listed in Duplicate Content table | ❌ Recommendation #1: "Delete..." | **File does not exist.** Only `ci.yml` is present in `.github/workflows/`. Already deleted. |
| `tests/unit/test_toml.c` / `test_depgraph.c` stale duplicates exist? | ❌ Listed as present in repo tree | not directly addressed | ❌ Listed in Duplicate Content table | ❌ Recommendation #2: "Delete..." | **Files do not exist.** `tests/unit/` contains only the `luna-init/` subdirectory. Already deleted. |
| DL-P04 (license) still Pending? | ❌ "still listed as Pending" | not directly addressed | ❌ "still contains DL-P04 as Pending... should be closed with DL-052" | ✅ correctly notes "Resolved... superseded by DL-052" (Risk #4) | **Already closed.** `decision_log.md`: `[DL-P04] License — *Superseded by DL-052*`; `[DL-052] Project License: MIT — Status: ACCEPTED`, dated 2026-06-28. |
| `lgp-compositor` implementation status | "NOT STARTED... Zero code exists" | "Zero code exists for: lgp-compositor..." | Out of scope ("Stage 0 only") | Out of scope ("absence of any Phase 2+ code to audit") | **In progress.** `src/lgp-compositor/` contains ~14 source files implementing DRM/KMS modesetting, a Unix socket server, and a TLV-based protocol (self-labeled M1–M3). Real, building code — see `CODE_AUDIT_REPORT.md` §1. |
| Unit test count | "4 tests, 1 each" (file tree comment) | not directly addressed | not directly addressed | "4 unit tests total, 1 per file... Zero negative tests" (Weakness) | **8 tests across 4 files.** `depgraph_test.c` has 3 (including a cycle-detection test and a missing-dependency test), `toml_test.c` has 3 (including 2 negative/malformed-input tests), `log_test.c` and `service_test.c` have 1 each. See `TEST_COVERAGE_REPORT.md`. |
| Kernel config: `CONFIG_CGROUP_V2` confusion | not addressed | not addressed | Flags as needing correction in `.config.notes` | not addressed | **Already corrected.** `kernel/.config.notes` line 212 already states cgroup v2 needs no separate kconfig option. |

**Interpretation:** `IMPLEMENTATION_STATUS.md` (not shown in the table because
it agrees with ground truth on every row above) is the only one of the five
documents in `docs/System Prompts/` that reflects the current code with
reasonable accuracy — and even it is wrong about one important thing: it
marks "Runtime log switch ✅ Complete" without catching that the underlying fd
is never opened (`CODE_AUDIT_REPORT.md` §2), and it still reports
`lgp-compositor` at "0% / NOT STARTED" despite real code existing.

**Recommendation:** Before Phase 2 work continues, regenerate
`PROJECT_STATE.md`, `ARCHITECTURE_CONSISTENCY_REPORT.md`, and
`REPOSITORY_HEALTH_REPORT.md` from the actual current source tree (not from
each other or from memory of an earlier session), and establish a process
that prevents "canonical" status documents from drifting apart — e.g.
generating them from a single script that great-greps the source for the
specific facts being asserted, or deleting the redundant ones and keeping
only `IMPLEMENTATION_STATUS.md` plus this audit as the source of truth going
forward.

---

## 2. Decision Log compliance (Phase 3/5)

### 2.1 🔴 Violation — DL-025 (LGP wire format)

Covered in full in `CODE_AUDIT_REPORT.md` §1.1. Restated here because this is
specifically a Decision-Log-level violation, not just a code bug:
`docs/DCKL/Volume I - Foundation/decision_log.md` (lines 748–764) and
`Volume III - Graphics & Presence/01_lgp.md` (line 162) both unambiguously
specify a 1-byte type + 4-byte length header for every LGP message. The
shipped `src/lgp-compositor/protocol/tlv.h`/`tlv.c` implements a 2-byte type +
4-byte length header. This is an accepted decision (`Status: ✅ Accepted`)
being silently superseded by code rather than by a new Decision Log entry —
exactly the failure mode `AI_CONTEXT.md` itself warns against ("Never change
accepted architecture... Do not contradict them or work around them").

### 2.2 No other Decision Log contradictions found

Every other accepted DL entry checked against the corresponding
implementation (DL-002 init in C17/static, DL-005 limine, DL-007
glibc/no-musl-yet, DL-008 TOML-only config, DL-009 kernel config target,
DL-021 lazy AI/Ollama not started at boot — verified true, nothing in
`etc/luna/services/ollama.toml` or `main.c` starts Ollama at boot, DL-026
software-renderer-first via dumb buffers rather than any GPU API call, DL-027
Btrfs referenced in kernel config, DL-032 input/libinput — **not yet
implemented, but also not contradicted**, since no other input backend was
substituted) was either correctly implemented or correctly *not yet*
implemented (a gap, not a contradiction).

### 2.3 ⚠️ Gap, correctly a gap — DL-031 compositor readiness signal not implemented

DL-031 specifies the compositor signals readiness via D-Bus
`org.mahina.compositor.Ready`, and Stage 6 services are meant to block on it.
There is no D-Bus client/server code anywhere in `lgp-compositor` or
`luna-init`. `main.c`'s Stage 5 starts the compositor and proceeds
unconditionally to print the welcome banner and fork a shell, without
waiting for any readiness signal. This is consistent with "not implemented
yet" rather than "implemented wrong," but it means the Stage 5→6 transition
that now actually executes in code has no readiness gate at all yet — worth
tracking explicitly now that Stage 5 has gone from stub to real code, rather
than waiting to discover it later.

### 2.4 ⚠️ Gap — DL-032 libinput not present

No `libinput` reference anywhere in `lgp-compositor`. Expected for the
current milestone (no window/input handling has been built yet); flagged
only so it's tracked as the compositor grows.

---

## 3. Boot pipeline verification (Phase 7)

```
UEFI → Limine → Linux → Initramfs → luna-init → luna-splash → lgp-compositor → (future LunaGUI)
```

| Transition | Verified? | Notes |
|---|---|---|
| Kernel → luna-init (PID 1) | ✅ | `main.c` mounts `/proc` before anything else, matches doc rationale. |
| luna-init → luna-splash | ✅ | `splash_start()` forks with a pipe; child execs `/sbin/luna-splash --fd N` with a clean, minimal envp. |
| luna-splash → lgp-compositor (DRM/framebuffer handoff) | ⚠️ Partially verified | `splash_stop()` now **synchronously** waits for `luna-splash` to exit (including its fade-out and explicit `munmap`/`close(/dev/fb0)`/`KDSETMODE KD_TEXT`) before `main.c` proceeds to start the compositor. This is the right *ordering*. However, `luna-splash` releases the legacy `/dev/fb0` fbdev interface while `lgp-compositor` acquires `/dev/dri/card0` via raw DRM/KMS — see §1.5 in the code audit for the unverified DRM-master assumption on the compositor side. The *order* is correct; the *acquisition* on the new side is not defensively verified. |
| lgp-compositor ownership of hardware | ⚠️ | Compositor cleans up its DRM resources (`kms_dumb_buffer_destroy`, `drm_device_close` with `drmDropMaster`) correctly on normal shutdown via `SIGTERM`/`SIGINT`. No crash-path-specific cleanup beyond normal signal handling was found (no `atexit`/crash handler that releases DRM master if the process dies unexpectedly, e.g. via `SIGSEGV`) — if the compositor crashes hard, the kernel will reclaim the fd on process exit regardless, so this is a low-severity gap, not a leak. |
| → Future LunaGUI | N/A | No code exists yet; correctly absent. |

**No subsystem was found holding hardware longer than intended** in the
normal-operation path. The main verification gap is the untested/unverified
assumption in `drm_device_open()` rather than an actual observed double-
ownership bug.

---

## 4. Graphics verification (Phase 6) — summary

| Question | Answer |
|---|---|
| Does splash release DRM/framebuffer correctly? | Yes, on the *fbdev* interface it actually uses (`/dev/fb0`), with explicit `munmap`/`close`/console-mode restore, and `luna-init` waits synchronously for this before proceeding. |
| Does the compositor acquire DRM correctly? | Functionally yes in the one-process test environment, but via an unverified assumption rather than an explicit master-acquisition check (`CODE_AUDIT_REPORT.md` §1.5). |
| Is the socket lifecycle correct? | Listening socket setup (permissions `0660 root:video`, stale-socket cleanup, `SOCK_NONBLOCK`) is correct. Per-connection lifecycle has a real bug — the client object (and its fd) is destroyed immediately after the handshake (`CODE_AUDIT_REPORT.md` §1.2). |
| Does TLV framing match documentation? | **No — see §2.1 above; this is the headline finding of this report.** |
| Is capability negotiation secure? | The privilege check itself (uid 0 only, via `SO_PEERCRED`) is sound, but it isn't actually enforced on subsequent messages from the same connection (`CODE_AUDIT_REPORT.md` §1.4). |
| Have Wayland assumptions leaked in? | No. No `wayland-client`, `wlroots`, or Wayland protocol references found anywhere in the compositor or its build files. Per AI_CONTEXT.md's instruction, this report does not suggest adopting Wayland/wlroots/Hyprland as a remedy for any finding above. |
| Undocumented shortcuts? | Yes — the wire format (§2.1) and the unverified DRM master acquisition (above) are both undocumented departures from the bring-up plan as written. |

---

## 5. Service file ↔ parser ↔ supervisor cross-check (Phase 3)

All eight `etc/luna/services/*.toml` files were checked field-by-field
against `service.c`'s parser and `supervisor.c`'s consumer logic.

| Field | Parsed | Enforced | Notes |
|---|---|---|---|
| `name`, `binary`, `description`, `args`, `workdir` | ✅ | ✅ | |
| `[service.depends].after` / `.before` | ✅ | ✅ (via `depgraph.c`) | `before` is resolved into edges but never actually consumed by the supervisor's start logic the way `after` is — `supervisor_pump()` only checks `svc->after[]`. No current service file uses `before`, so this is latent, not active; flagged for completeness. |
| `[service.restart]` | ✅ | ✅ | |
| `[service.ready]` (file/socket) | ✅ | ✅ | `http`/`signal` methods are explicitly, loudly stubbed (`LUNA_WARN` on first use) — this is a **documented, intentional** gap, correctly self-flagged in code, not a silent failure. |
| `[service.stop]` | ✅ | ✅ | |
| `[service.identity]` (`user`/`group`) | ✅ | ✅ | Contrary to three of the four status docs — see §1. |
| `[service.env]` | Documented, struct fields exist, consumed by `supervisor.c` | **Not parsed** | See `CODE_AUDIT_REPORT.md` §7.1. Currently dormant because no shipped service file uses it. |

---

## 6. Final architecture-compliance verdict

- **No accepted architecture was found "worked around" or silently replaced**
  in the Stage 0 (`luna-init`/`luna-splash`) code — that code is a genuinely
  faithful, high-quality implementation of `Volume II`.
- **One confirmed, unambiguous Decision Log violation exists in the new
  `lgp-compositor` code** (§2.1) and should be resolved — by decision, not by
  silent code or doc edits — before any second client is built against the
  current wire format.
- **The project's own self-assessment documents cannot currently be trusted
  without independent verification** (§1). This is itself flagged as a
  process risk, separate from any individual code defect: a Documentation-
  First methodology only works if the documentation is re-derived from
  ground truth, and right now most of it is not.
