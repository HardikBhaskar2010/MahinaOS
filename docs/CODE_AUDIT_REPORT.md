# CODE_AUDIT_REPORT.md — Mahina OS

**Auditor role:** Principal Systems Engineer, pre-merge review
**Scope:** Every implementation file in `src/`, `tests/`, `scripts/`, `kernel/`, `etc/`, `boot/`, `.github/workflows/`, `Makefile`
**Method:** Line-by-line review of source against documented behavior, plus independent correctness/safety analysis
**Repository state audited:** `HardikBhaskar2010/MahinaOS`, default branch, as cloned 2026-06-29

Severity key: 🟢 Minor 🟡 Medium 🟠 High 🔴 Critical

---

## 0. How to read this report

This audit found that the repository's own "living" status documents
(`PROJECT_STATE.md`, `AI_CONTEXT.md`, `ARCHITECTURE_CONSISTENCY_REPORT.md`,
`REPOSITORY_HEALTH_REPORT.md`) are **substantially stale relative to the actual
code**, and in several places **contradict each other and contradict the
ground truth in the same repository**. This report's findings are based on
**reading the source code directly**, not on trusting those documents. Every
finding below cites the exact file, function, and line behavior observed.
Where a documentation claim was checked and found wrong, that is noted
explicitly so the documentation can be fixed (see
`ARCHITECTURE_COMPLIANCE_REPORT.md` for the full doc-vs-code reconciliation).

---

## 1. `src/lgp-compositor/` — LGP Compositor (new subsystem, undocumented as "started")

This entire directory is real, building, milestone-labeled code (`M1`/`M2`/`M3`
comments throughout, wired into `Makefile` via `src/lgp-compositor/Makefile.inc`).
It is **not** "zero code" as `PROJECT_STATE.md` / `AI_CONTEXT.md` claim. It is
early-prototype quality and has the most serious findings in the repo.

### 1.1 🔴 Critical — Wire format violates accepted Decision Log DL-025

**File:** `src/lgp-compositor/protocol/tlv.h`, `tlv.c`
**Function:** `lgp_tlv_decode()`, `lgp_tlv_encode_header()`

- **Expected (DL-025, accepted, AR-004; also restated in
  `docs/DCKL/Volume III - Graphics & Presence/01_lgp.md` line 162 and in
  `AI_CONTEXT.md`'s "Things AI Must NEVER Change"):** TLV header = **1-byte
  type + 4-byte length** (5-byte header), struct `lgp_message_t`.
- **Actual:** `tlv.h` defines `LGP_HEADER_SIZE 6` and a header comment that
  explicitly documents `uint16_t type` + `uint32_t length` (6-byte header),
  struct `lgp_msg_t`. Message type constants (`LGP_MSG_ERROR = 0xFFFF`)
  require two bytes and could not fit the documented 1-byte field.
  `test_client.c` was hand-written to match the *implemented* 6-byte framing,
  not the documented one, confirming this is a sustained design choice, not a
  typo.
- **Risk:** This is a foundational, append-only wire protocol. Every future
  client (LunaGUI, luna-shell, luna-island) will be built against whichever
  framing ships first. Shipping the undocumented 6-byte framing now, then
  "discovering" the conflict with DL-025 later, means either an incompatible
  protocol break or a permanent, silent divergence between the Decision Log
  and reality.
- **Fix:** This needs a human decision, not a silent code change: either (a)
  superseded DL-025 with a new entry (e.g. DL-053) documenting the 2-byte
  type field and updating `01_lgp.md`, or (b) change the implementation to a
  genuine 1-byte type field before any other component is built against the
  current framing. Per `AI_CONTEXT.md` itself, accepted DL entries are
  supposed to be immutable without a superseding entry — right now neither
  the code nor the docs reflect an actual decision.

### 1.2 🟠 High — Client objects are destroyed (and their socket closed) directly after the handshake, contradicting the code's own intent

**File:** `src/lgp-compositor/main.c` (around the `LGP_MSG_HELLO` branch),
`src/lgp-compositor/ipc/client.c` (`lgp_client_destroy`)

```c
lgp_client_t *client = lgp_client_create(fd);
if (client) {
    lgp_hello_handle(client, &msg);
    /* Destroy the client object, but we keep the fd alive for this mock M3 */
    lgp_client_destroy(client);
}
```

`client.h` documents `lgp_client_destroy()` as: *"Close the client fd and
free memory."* — and `client.c`'s implementation does exactly that
(`close(client->fd)`). The comment in `main.c` asserts the fd is kept alive;
the function it calls does the opposite. The practical effect: immediately
after a client completes the `LGP_HELLO` handshake and receives
`LGP_HELLO_REPLY`, the compositor closes its end of the connection. Any
client that waits for the reply before sending its next message (the correct,
documented protocol flow) will find the socket gone. `test_client.c`'s
comment — *"Keep connection open so compositor doesn't tear us down
immediately"* (line 71) — and the fact that it `write()`s its `FILL_RECT`
message immediately after the reply, without delay, is consistent with the
author having worked around exactly this bug via timing rather than fixing
it.
**Fix:** Do not call `lgp_client_destroy()` after a successful handshake.
Track the client struct for the lifetime of the connection (e.g. keyed by
fd) and destroy it only when the fd is actually being torn down.

### 1.3 🟡 Medium — No message reassembly for stream socket reads

**File:** `src/lgp-compositor/main.c` (client read branch)

```c
uint8_t buf[1024];
ssize_t s = read(fd, buf, sizeof(buf));
...
if (lgp_tlv_decode(buf, s, &msg)) { ... }
```

`AF_UNIX SOCK_STREAM` sockets do not guarantee one `write()` maps to one
`read()`. A message split across two TCP-like segments, or a message larger
than 1024 bytes, will either be silently dropped (`lgp_tlv_decode` returns
`false` when `buf_len < length` and the leftover bytes are discarded — there
is no per-fd carry-over buffer) or truncated. Contrast this with
`src/luna-splash/ipc.c::ipc_read_event()`, which correctly accumulates a
per-stream buffer and only consumes complete, newline-delimited messages —
the project already has the right pattern, it just wasn't applied here.
**Fix:** Add a per-client receive buffer (mirroring `ipc.c`'s approach) before
any real client is expected to talk to this socket.

### 1.4 🟡 Medium — Capability negotiation is not enforced after handshake

**File:** `src/lgp-compositor/main.c`, `protocol/caps.c`

`lgp_caps_negotiate()` correctly restricts privileged capabilities
(`LGP_CAP_LAYER_SHELL`, `LGP_CAP_LUNA_ISLAND`) to `uid == 0` clients, using a
real `SO_PEERCRED`-derived UID (`ipc/client.c`) rather than anything
client-supplied — that part is sound. However, because the client object is
destroyed after `HELLO` (1.2) and no per-fd capability state is retained, the
event loop's generic message branch (`LGP_MSG_FILL_RECT`, etc.) processes
messages from **any** connected fd with no check that a handshake occurred or
which capabilities were granted to that specific peer. A client could skip
`LGP_HELLO` entirely and send `LGP_MSG_FILL_RECT` directly. For an early
prototype with one trusted local test client this is harmless, but it means
"capability negotiation" is currently advisory, not enforced — note for
Phase 6/8 before any untrusted client is allowed to connect to the
`root:video 0660` socket.

### 1.5 🟡 Medium — DRM master acquisition is an unverified assumption

**File:** `src/lgp-compositor/drm/device.c::drm_device_open()`

```c
/* We implicitly acquire DRM master by opening the device, assuming
   luna-splash has closed it. ... We assume success for now per
   bring-up plan. */
```

There is no explicit `drmSetMaster()` call, no check for `EBUSY`/`EACCES` on
mode-setting due to lost master status, and no retry/backoff. In the current
single-process, no-display-manager system this likely works by virtue of
being the only opener, but it is an undocumented assumption rather than a
verified handoff, and Phase 6 explicitly calls for verifying this exact
boundary. **Fix:** at minimum, log and fail loudly (already done for open
failure) and add an explicit master-acquisition check rather than relying on
implicit kernel behavior.

### 1.6 🟢 Minor — Detected atomic modesetting capability is never used

**File:** `drm/device.c` (sets `dev->atomic_supported`), `kms/crtc.c` (always
uses legacy `drmModeSetCrtc`)

Harmless for M1 (explicitly commented as legacy-only for now) but the
capability flag is otherwise dead data. Fine to leave as a placeholder for
the planned atomic-KMS work, just flagging so it isn't mistaken for a bug
later when no caller ever branches on it.

---

## 2. `src/luna-init/log.c` — 🔴 Critical: the runtime log file is never opened

**File:** `src/luna-init/log.c`
**Functions:** `luna_log_init()`, `luna_log_switch_to_runtime()`

`log.h` documents `luna_log_switch_to_runtime()` as: *"Close boot log, open
runtime log."* The implementation only does the first half:

```c
void luna_log_switch_to_runtime(void) {
    g_runtime_mode = true;
    g_min_level    = LOG_WARN;
    if (g_boot_fd >= 0) {
        (void)fsync(g_boot_fd);
        (void)close(g_boot_fd);
        g_boot_fd = -1;
    }
}
```

`g_runtime_fd` is declared, checked in `log_write_entry()`
(`fd = (g_runtime_fd >= 0) ? g_runtime_fd : STDERR_FILENO;`), and closed in
`luna_log_close()` — but it is **never assigned anywhere in the file**.
There is no `open()` call against `RUNTIME_LOG_PATH` at all. Compounding this,
`luna_log_init()` receives `runtime_log_path` and explicitly discards it:

```c
(void)runtime_log_path; /* stored externally by caller if needed */
```

— but `luna_log_switch_to_runtime()` takes no arguments (`void`), so there is
no "external" path by which the caller (`main.c`) can hand the path over at
switch time either. **Net effect:** once Stage 5 is reached (which now
happens — see `ARCHITECTURE_COMPLIANCE_REPORT.md`), every subsequent
`LUNA_*` log call for the rest of the system's uptime — including supervisor
restarts, service failures, and shutdown — silently goes to `STDERR` instead
of `/var/log/luna-init/runtime.log`. This is a different and more precise bug
than any of the repo's own status documents describe; none of them caught
that the fd is never opened — they only argued (incorrectly) about whether
the *function* is called.

**Fix:** Either store `runtime_log_path` in a static buffer in
`luna_log_init()` and `open()` it inside `luna_log_switch_to_runtime()`, or
change the function's signature to accept the path explicitly. Add a unit
test asserting `g_runtime_fd >= 0` (or equivalent observable behavior) after
the switch.

---

## 3. `src/luna-init/supervisor.c`

### 3.1 ✅ Verified correct (contradicts stale docs — see compliance report)
- `spawn_service()` does call `setgid()`/`getgrnam()` then `setuid()`/`getpwnam()`
  in the correct order (group before user) before `execve()`.
- Cgroup assignment (`mkdir` + write `cgroup.procs`) is implemented and paired
  with `mount_early()` mounting `cgroup2` at `/sys/fs/cgroup`.
- The supervisor is asynchronous (`supervisor_pump()` driven by a 100ms
  `timerfd` in `main.c`'s epoll loop), not a blocking call.

### 3.2 🟡 Medium — Privilege drop does not clear supplementary groups

**File:** `supervisor.c::spawn_service()`, child branch

The child calls `setgid()` then `setuid()` (correct ordering to avoid losing
the privilege needed for `setgid()`), but never calls `setgroups(0, NULL)` or
`initgroups()`. A process forked from PID 1 (root, supplementary group list
typically just `root`/`0`) and then dropped to e.g. `messagebus`/`pipewire`
via `setuid()`/`setgid()` will **retain root's original supplementary group
list** unless explicitly cleared. This is a standard, well-known privilege-
drop pitfall (the same class of bug CVE advisories are regularly filed for in
daemons that drop privileges). **Fix:** call `setgroups(0, NULL)` after
`setgid()` and before `setuid()`.

### 3.3 🟡 Medium — `supervisor_start_one()`'s return value does not reflect actual start success

**File:** `supervisor.c::supervisor_start_one()`, called from `main.c` Stage 5
for `lgp-compositor`

`supervisor_start_one()` sets state to `PENDING` and calls `supervisor_pump()`
once, then unconditionally `return 0;` (success) unless the service name
wasn't found at all. If `spawn_service()` inside that pump call fails (e.g.
binary missing → `DEGRADED`), the caller in `main.c` (`comp_result`) still
sees `0`, so the log message *"lgp-compositor failed to start — degraded
graphics mode"* will essentially never fire for the failure mode it's meant
to catch. **Fix:** have `supervisor_start_one()` propagate the post-pump
state of the named service (e.g. return `-1` if state is `DEGRADED`/`FAILED`
after the pump call).

### 3.4 🟢 Minor — `mkdir()`/`chmod()`/`chown()` return values not checked in cgroup/socket setup paths

Several `mkdir()`/`chmod()`/`chown()` calls (in `supervisor.c`'s cgroup setup
and `lgp-compositor/ipc/socket_server.c`'s permission setup) ignore return
values beyond the immediately-following code path. Not exploitable as
written (failures degrade gracefully — e.g. a failed `mkdir` simply causes
the subsequent `open()` to fail and is logged), but worth tightening per the
project's own "no silent failures" coding standard.

---

## 4. `src/luna-init/main.c`

- Boot stages 0–4 match the documented sequence and `04_init_system.md`.
- **Stage 5 is now implemented** (`splash_stop()` →
  `supervisor_start_one("lgp-compositor")` → `luna_log_switch_to_runtime()`),
  contradicting `PROJECT_STATE.md`/`AI_CONTEXT.md`'s claim that Stage 5 is an
  empty stub. See `ARCHITECTURE_COMPLIANCE_REPORT.md`.
- 🟡 Medium: Stage 5 does not wait for the LGP compositor's documented D-Bus
  `org.mahina.compositor.Ready` signal (DL-031) before declaring graphics
  stage complete and proceeding to print the welcome banner / drop to shell —
  there is no D-Bus client code anywhere in the compositor or `luna-init` at
  all yet, so this signal cannot exist. This is consistent with "not yet
  built" rather than "built wrong," but it means the Stage 5→6 handoff is
  currently unconditional and not gated on compositor readiness as DL-031
  specifies.
- 🟢 Minor: the welcome-banner shell is forked directly
  (`console_drop_to_shell()`) rather than via `luna-shell`/`luna-island` —
  correctly matches "Stage 6 not implemented."

---

## 5. `src/luna-init/splash.c` / `src/luna-splash/`

- ✅ Verified: the fallback exec path is `./build/luna-splash/luna-splash`
  (correct, matches the Makefile's actual output path) — the "fallback path
  bug" described in three of the four status docs **does not exist in the
  current code**.
- ✅ Verified: `splash_stop()` now sends `SIGTERM` and calls a **synchronous**
  `waitpid()` before returning, specifically to ensure `luna-splash` has
  fully released `/dev/fb0` (including its ~480ms fade-out animation in
  `render_fade_out()`) before `main.c` starts the compositor. This is a
  reasonable, deliberate design for a one-time stage transition; flagging
  only that it does briefly block PID 1's event loop for the duration of the
  fade-out, which is acceptable but worth a one-line comment in the code
  explaining the tradeoff for future maintainers.
- ✅ Verified: `render_logo()` does implement real bitmap rendering of the
  block-drawing "MAHINA" logo via hand-rolled UTF-8 parsing, not a fallback
  to ASCII text — the opposite of what `PROJECT_STATE.md` claims.

### 5.1 🟡 Medium — `render_logo()` / `render_progress()` assume 32bpp and will corrupt or crash on other framebuffer depths

**File:** `src/luna-splash/render.c`

`put_pixel()` correctly branches on `vinfo.bits_per_pixel` (32/16/24).
`render_logo()` and `render_progress()` instead write directly through
`fb_mem` using `(y * line_length / 4) + x` — i.e. they **assume 4
bytes/pixel** unconditionally, with no bpp check. If the framebuffer is not
32bpp (e.g. 16bpp, which `put_pixel()` explicitly supports elsewhere and
which is common on some firmware/virtual framebuffer configurations), this
stride math is wrong: at 16bpp, `line_length` is roughly half of what the
`/4` divisor assumes, and if `line_length` is not evenly divisible by 4 the
integer division silently truncates, causing the effective pixel address to
drift further out of alignment with every scanline. In the worst case this
walks the write pointer past the `mmap()`-mapped region (`screensize =
yres_virtual * line_length`), which is a real out-of-bounds write, not just a
cosmetic glitch. **Fix:** route `render_logo()`/`render_progress()` through
`put_pixel()` (already bpp-aware) instead of touching `fb_mem` directly, or
add the same bpp branch.

### 5.2 🟢 Minor — Burst IPC updates can be visually delayed by one cycle

**File:** `src/luna-splash/main.c`, `ipc.c`

`ipc_read_event()` is correctly written to buffer partial reads and only
return one complete message per call, but `main.c`'s event loop calls it
**once** per `EPOLLIN` event rather than draining all complete buffered
messages in a loop. If two `PERCENT|MSG\n` updates arrive in the same
underlying `read()`, the second is rendered only once a *third* update
arrives (or never, if it was the last update before boot completes). Purely
cosmetic (a progress jump instead of a smooth increment); not a data-loss
bug because the buffering logic itself is correct.

---

## 6. `src/luna-init/toml.c`

Overall this is the best-engineered file in the repository: single `calloc`
for the document, fixed-size line buffer, every string copy bounded, AFL++
harness present (`tests/fuzz/toml/fuzz_toml.c`). Two real parser-leniency
bugs found on close reading:

### 6.1 🟡 Medium — Boolean/keyword matching is a prefix match, not a full-token match

**File:** `toml.c::parse_value()`

```c
if (strncmp(raw, "true", 4) == 0) { ...; return TOML_OK; }
if (strncmp(raw, "false", 5) == 0) { ...; return TOML_OK; }
```

A value like `enabled = trueish` or `flag = falsexyz` is silently accepted as
boolean `true`/`false` because only the first 4/5 characters are checked,
with no check that the token ends there. **Fix:** require the next character
after the prefix to be whitespace, `#`, or end-of-string.

### 6.2 🟡 Medium — Integer parsing accepts trailing garbage

**File:** `toml.c::parse_value()`

```c
long long iv = strtoll(raw, &end, 10);
if (end != raw && errno == 0) { ...; return TOML_OK; }
```

There is no check that `*end` is `'\0'` (end of the trimmed value). A value
like `timeout_ms = 5000garbage` parses as the integer `5000` with the
trailing text silently ignored, rather than being rejected as
`TOML_ERR_UNSUPPORTED`/`TOML_ERR_INVALID`. This directly conflicts with the
project's own documented coding standard ("No silent failures. Every error is
logged or propagated" — `AI_CONTEXT.md` / `01_coding_standards.md`).
**Fix:** add `if (*end != '\0') return TOML_ERR_INVALID;` after the
`strtoll()` call (the value has already been trimmed of trailing
whitespace/comments by `parse_line()`, so this is a precise check).

### 6.3 🟢 Minor — `toml_get()` returns the *first* matching entry, not the last

If a key were ever duplicated within a section (malformed input, but not
rejected by the parser), `toml_get()`'s linear scan returns the first
occurrence. Most config-file conventions treat the last occurrence as
authoritative. Not currently exploitable by any real `.toml` file in the
repo, but worth a one-line doc comment so it isn't assumed otherwise later.

---

## 7. `src/luna-init/service.c`

### 7.1 🟡 Medium — `[service.env]` is documented and consumed downstream, but never parsed

`service.h` documents `env[]`/`env_count` as backing `[service.env]`, and
`supervisor.c::spawn_service()` actively builds `envp[]` from
`svc->env[i].key/val`. But `service_load_one()` in `service.c` has no code
path that populates `env[]`/`env_count` from the TOML document — there is no
`GET_STR`/loop for the `service.env` section at all. Today this is dormant
(none of the eight files in `etc/luna/services/*.toml` define
`[service.env]`), but it will silently and invisibly fail the first time a
future service (e.g. `luna-ai-d`, which plausibly needs an `OLLAMA_HOST` or
similar) declares one. **Fix:** implement the missing parse loop (iterate
TOML entries with `section == "service.env"`, treat `key`/string `value` as
the env pair) before this is relied upon.

### 7.2 🟢 Minor — `parse_signal_name()` defaults silently to `SIGTERM` for unrecognized signal names

A typo in a service file's `stop.signal` (e.g. `"SIGTERMM"`) is silently
treated as `SIGTERM` rather than logged as an unrecognized value. Low risk
(SIGTERM is a safe default) but inconsistent with "no silent failures."

---

## 8. `src/luna-init/depgraph.c` — 🟡 Medium: half the module is dead code

`depgraph_build()` is real, correct, and used. `depgraph_topo_sort()` (Kahn's
algorithm topological ordering) and the `dep_indices[]`/`dep_count` fields it
populates on each `service_t` are **never called or read by any production
code path** — `grep` across `src/` and `tests/` shows the only callers are
`depgraph.c` itself and `depgraph_test.c`. `supervisor.c::supervisor_pump()`
independently re-derives the same dependency information by calling
`service_find()` on the raw `svc->after[]` name strings every pump cycle,
rather than consulting the precomputed graph. This is not a bug today (the
two independent implementations happen to agree), but it is real duplicated
logic and a real architecture-layering smell: the dependency graph module
exists, is unit-tested, and is unused by the one consumer it was built for.
**Fix:** either have `supervisor_pump()` consume `depgraph_topo_sort()`'s
output / `dep_indices[]`, or remove the unused function and fields and
document that dependency resolution is intentionally done ad-hoc in the
supervisor.

---

## 9. `src/luna-init/ctl.c` — 🟠 High: PID 1 can be stalled by a single slow/silent client

**File:** `ctl.c::ctl_server_accept()`

```c
int client = accept4(server_fd, NULL, NULL, SOCK_CLOEXEC);
...
ssize_t n = read(client, buf, sizeof(buf) - 1);
```

The listening socket is created with `SOCK_NONBLOCK` (`ctl_server_init()`),
but the **flag is not requested on the accepted client fd**
(`accept4(..., SOCK_CLOEXEC)` only). Accepted sockets do not inherit
non-blocking mode from the listener; the returned `client` fd is blocking by
default. `ctl_server_accept()` is called directly from `main.c`'s single-
threaded epoll loop, with no timeout. Any local process with permission to
open the `0600` socket (i.e. any root-privileged process — note the socket
permission itself is correctly tight, this is not remotely or unprivileged-
exploitable) that connects and then simply never writes will cause this
`read()` to block **forever**, freezing the entire `luna-init` event loop:
no more signal handling, no zombie reaping, no service supervision, nothing,
until that one connection is closed. **Fix:** add `SOCK_NONBLOCK` to the
`accept4()` flags and register the client fd with `epoll` for an
asynchronous read (with a short idle timeout), rather than reading inline.

---

## 10. `src/luna-init-ctl/main.c`

- ✅ Help/usage text is fully implemented (contradicts
  `IMPLEMENTATION_STATUS.md`'s "Not verified" note for this item).
- 🟢 Minor: the `status`/`start`/`stop`/`restart`/`reload` commands build
  their JSON request via raw `snprintf("...\"name\":\"%s\"...", name)` with no
  escaping of the user-supplied service name. A name containing a `"` would
  corrupt the request sent to `luna-init`'s naive string-search JSON parser
  (`ctl.c::json_get_field`). Low impact — this tool is usable only by
  whoever already has access to the `0600` control socket — but worth fixing
  for robustness.

---

## 11. `src/luna-init/{mount,reaper,signal,shutdown,panic,hostname,console}.c`

All reviewed in full. These are the most mature files in the repository:
bounded buffers throughout, correct `signalfd`+`epoll` usage (no
`signal()`/`sigaction()` race conditions), correct LIFO unmount order on
shutdown, a sensible shell-candidate fallback chain in the panic handler. No
correctness, memory-safety, or resource-leak findings beyond the
`luna_log_close()`-before-further-logging ordering noted informationally in
§12 below. This matches the documentation's praise of `luna-init`'s core as
high quality, and that praise is earned for these specific files.

### 12. 🟢 Minor — Logging continues after the log is explicitly closed during shutdown

**File:** `shutdown.c::shutdown_run()`

`luna_log_close()` is called as "Step 1," before `supervisor_stop_one()` and
`mount_unmount_all()` run — both of which call `LUNA_INFO`/`LUNA_WARN`.
`log_write_entry()` already degrades gracefully to `STDERR_FILENO` when the
fd is `-1`, so this is not a crash risk, just worth noting that shutdown-path
log messages never reach the persistent log file, only the console — for an
init system, the shutdown sequence is exactly the period where a missed log
entry is most likely to matter for postmortem debugging.

---

## Summary table

| # | Area | Severity | One-line summary |
|---|---|---|---|
| 1.1 | lgp-compositor wire format | 🔴 Critical | TLV header is 2B+4B, DL-025 mandates 1B+4B |
| 2 | log.c runtime log | 🔴 Critical | Runtime log fd never opened; all post-boot logs silently go to stderr |
| 1.2 | lgp-compositor client lifecycle | 🟠 High | Client fd closed right after handshake, contradicting its own comment |
| 9 | ctl.c accept handling | 🟠 High | Blocking read with no timeout can freeze PID 1's event loop |
| 1.3 | lgp-compositor TLV framing | 🟡 Medium | No stream reassembly; messages can be dropped/truncated |
| 1.4 | lgp-compositor caps enforcement | 🟡 Medium | Capabilities negotiated but not enforced per-connection afterward |
| 1.5 | lgp-compositor DRM master | 🟡 Medium | Master acquisition assumed, not verified |
| 3.2 | supervisor.c privilege drop | 🟡 Medium | Supplementary groups not cleared on setuid/setgid |
| 3.3 | supervisor.c return value | 🟡 Medium | start_one() doesn't propagate actual spawn failure |
| 5.1 | render.c bpp assumption | 🟡 Medium | Logo/progress bar assume 32bpp; can write out of bounds otherwise |
| 6.1 / 6.2 | toml.c leniency | 🟡 Medium | Boolean/integer parsing accepts trailing garbage |
| 7.1 | service.c env vars | 🟡 Medium | [service.env] documented, consumed downstream, never parsed |
| 8 | depgraph.c | 🟡 Medium | topo_sort()/dep_indices computed but never consumed (dead code) |
| 1.6, 3.4, 5.2, 6.3, 7.2, 10, 12 | various | 🟢 Minor | See individual sections |
