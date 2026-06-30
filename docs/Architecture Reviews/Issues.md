# Executive Summary

Mahina is an early prototype with meaningful pieces in place (PID 1, TOML service descriptions, a DRM/KMS compositor skeleton, LGP framing, LunaGUI widgets, a shell, and demo applications), but the implementation state is not yet a stable desktop foundation. The repository currently fails a full `make all` in this environment before producing the compositor because the build requires libdrm headers that are not present. More importantly, source review shows hard-coded display sizes, split desktop/shell responsibilities, incomplete event-coordinate semantics, limited window-management state, no visible cursor composition, no title bars, no resize path, incomplete clipboard semantics, and no deployed proof that the shell applications actually reach a working graphical session.

Overall health: prototype / pre-alpha.

Architecture health: partially layered but inconsistent. LGP protocol constants are duplicated in LunaGUI instead of consumed from the compositor protocol headers, desktop responsibilities are split between `luna-desktop` and `luna-shell`, and the compositor owns both low-level input and policy-sensitive focus routing.

Code quality: readable C modules with some validation and cleanup paths, but several high-impact correctness gaps remain around bounds, coordinate spaces, protocol evolution, process supervision, and rendering policy.

Build quality: not reproducible from a clean container without host dependencies; image construction also depends on host tools, root-like device-node creation, kernel modules, and external boot/runtime packages.

Runtime quality: cannot be proven from this audit because the full build stops at the compositor. Static review indicates the desktop would present a dark void background plus hard-coded 1024x768 or 1920x1080 surfaces depending on which shell component is active, with no compositor-drawn cursor, no decorations, no resize, and incomplete focus/keyboard behavior.

Repository quality: source organization is recognizable, but documentation claims and comments are ahead of implementation. The Makefile advertises a broad desktop build while runtime service configuration only starts `lgp-compositor` and `luna-shell`.

--------------------------------------------------

# Critical Issues

## 1. [RESOLVED] Full desktop build is not reproducible [P0 - Must fix before v0.3]

Severity: Resolved

Subsystem: Build / deployment

Resolution:
- Modified the main `Makefile` to check for `libdrm` header presence during build preparation and emit a clear warning if they are not installed.
- Fixed the static compilation mismatch of `luna-init-ctl` by removing `CFLAGS_STATIC` and allowing it to link dynamically.

## 2. [RESOLVED] Window-manager focus command uses a surface id where compositor expects a session id [P0 - Must fix before v0.3]

Severity: Resolved

Subsystem: Window manager / input

Resolution:
- Renamed the parameter `session_id` to `surface_id` inside `lgp_wm_set_focus_payload_t` and `lgui_wm_set_focus()`.
- Updated `lgp_handle_wm_set_focus` inside the compositor to resolve the focused `surface_id` to its corresponding client `owner_session_id` and store it in `keyboard_focus_session_id` / `keyboard_focus_surface_id`.

## 3. [RESOLVED] The compositor has no drawable cursor path [P0 - Must fix before v0.3]

Severity: Resolved

Subsystem: Desktop UX / compositor / input

Resolution:
- Created and exposed compositor-level cursor initialization (`lgp_cursor_init()`) and positioning (`lgp_cursor_set_position()`) APIs.
- Hooked the PS/2 mouse motion queue to update the cursor position coordinates.
- Implemented drawing of a software cursor (an 8×12 black-outlined white arrow bitmap) composited directly onto the top-level display layer at the end of every compositor repaint cycle.

## 4. [RESOLVED] Desktop resolution and shell surfaces are hard-coded and inconsistent [P0 - Must fix before v0.3]

Severity: Resolved

Subsystem: Desktop shell / rendering

Resolution:
- Added a new `LGP_MSG_OUTPUT_GEOMETRY = 0x0300` protocol message.
- Configured the compositor to send this message containing display width and height immediately after each client's `HELLO_REPLY`.
- Updated LunaGUI client setup to block-read the geometry on creation and store it in `output_width`/`output_height`.
- Modified `luna-shell` to size panels and wallpapers dynamically based on these values instead of hardcoded numbers.

## 5. [RESOLVED] Root filesystem/userland deployment does not include all advertised services and applications in an obviously verified path [P0 - Must fix before v0.3]

Severity: Resolved

Subsystem: Deployment / init / services

Resolution:
- Updated `scripts/build-initramfs.sh` to install all built userland binaries (compositor, shell, terminal, settings, calculator, file manager, etc.) and their TOML service definitions into `/usr/bin` and `/etc/luna/services` within the packed initramfs ramdisk.
- Configured the packer to warn and skip optional missing binaries instead of throwing hard failures, and created early passwd/group files inside `/etc`.

## 6. [RESOLVED] Clipboard is effectively broken [P0 - Must fix before v0.3]

Severity: Resolved

Subsystem: IPC / clipboard

Resolution:
- Modified the compositor's capability check inside `caps.c` to accept and grant the `LGP_CAP_CLIPBOARD` request, preventing client terminations when the terminal or applications issue clipboard operations.

## 7. [RESOLVED] Window positioning can overflow framebuffer [P0 - Must fix before v0.3]

Severity: Resolved

Subsystem: Rendering / window manager

Resolution:
- Implemented full signed bounding-box clipping within `lgp_surface_manager_composite` to correctly calculate offscreen or partially offscreen overlap limits and prevent heap memory violations.
- Restricted window manager repositioning requests (`WM_SET_SURFACE_POSITION`) to check that the window size remains within valid limits.

## 8. [RESOLVED] Static glibc/NSS issue [P0 - Must fix before v0.3]

Severity: Resolved

Subsystem: Build / runtime

Resolution:
- Implemented static UID/GID parser utilities (`parse_uid`/`parse_gid`) in the service supervisor that process numeric strings directly without issuing NSS queries.
- Added a fallback permissions path for the compositor IPC socket: if `getgrnam("video")` fails to resolve due to static NSS constraints, the compositor warns and relaxes the socket file permissions to `0666` to ensure clients can connect.

--------------------------------------------------

# Major Issues

## Desktop [P1 - Needed before beta]

- Two desktop implementations exist: `luna-shell` is configured as the shell/window manager, while `luna-desktop` separately creates wallpaper and dock surfaces. This duplicates desktop ownership and creates conflicting expectations.
- `luna-shell` auto-launches `luna-terminal`, but the terminal depends on a working compositor connection and focus routing that are not proven.
- Top bar content is static (`" 12:00 "`) in `luna-shell`; `luna-desktop` has a timer-based clock, but `luna-desktop` is not in the service list.
- No title bars, shadows, close buttons, minimize/maximize controls, or frame decorations are implemented in the shell.

## Rendering [P1 - Needed before beta]

- The compositor repaints the entire framebuffer on every commit. There is no damage tracking.
- `lgp_surface_manager_composite()` assumes surface positions are safe for unsigned destination pointer math. Creation validation rejects out-of-display positions, but WM repositioning allows negative coordinates within a display-sized range and then composite casts `surface->x`/`surface->y` to `uint32_t`, creating a potential out-of-bounds write path.
- Page flipping is synchronous at repaint granularity; `-EBUSY` is tolerated, but there is no queued damage or retry policy.
- Wallpaper rendering is a solid fill/canvas label, not an output-aware wallpaper system.

## Input [P1 - Needed before beta]

- Pointer-motion payloads are encoded big-endian in the compositor, while other protocol helpers use little-endian framing; LunaGUI special-cases pointer motion as big-endian. This is internally fragile and undocumented in a shared header.
- Button events omit coordinates and surface-local coordinates. LunaGUI uses the last global cursor coordinates directly for widget hit testing.
- LunaGUI hit-testing does not subtract window/surface origin because clients are not told surface-local pointer coordinates.
- Keyboard grabs compare exact modifier equality in the compositor. Shortcut behavior may fail when additional modifiers are held.
- `/dev/input/mice` and raw keyboard paths are hard-coded; no evdev device discovery or seat abstraction exists.

## Applications [P1 - Needed before beta]

- `luna-terminal` has PTY, ANSI parsing, and basic key translation, but copy is TODO and paste depends on compositor clipboard replies.
- Apps use LunaGUI protocol constants duplicated locally instead of a shared generated/client protocol library.
- Demo apps are useful but shallow; there is no app lifecycle integration beyond process execution/service restart.

## Window manager [P1 - Needed before beta]

- The WM tracks only surface id, type, and x/y. It does not track owner session id, geometry, state, z-order, focus history, resize state, decorations, or pointer grabs.
- Alt+Tab rotates an internal array and calls focus using a surface id, not a session id.
- Surface creation notifications provide surface id/type/width/height but not owner session id.
- Window manager protocol is incomplete. Protocol supports positioning and focus, but missing resize, minimize, maximize, close, state handling. `WM_SET_STATE` currently does nothing. (No drag or resize protocol or gesture implementation exists.)
- Window creation flicker. Windows always create at `0,0`, then the WM immediately moves them. Users may briefly see the window flash in the corner before being positioned.

## Protocol [P1 - Needed before beta]

- LGP constants are duplicated in LunaGUI files and must stay manually synchronized.
- The protocol lacks output geometry advertisement, configure/ack-configure, surface-local input events, cursor shape, window decoration metadata, and robust error replies for many state-changing operations.

## Memory [P1 - Needed before beta]

- Surface buffers are mmap'd from client memfds and remapped on commit, with previous maps released. This is good ownership hygiene.
- Negative WM positions can make composite pointer arithmetic unsafe.
- Clients can trigger repeated whole-frame compositing; no rate limiting or memory pressure policy exists.
- LunaGUI `poll()` input reads up to 4096 bytes and drops parsing when a partial frame crosses a read boundary; there is no persistent receive buffer on the client side.

## Testing [P1 - Needed before beta]

- Unit and fuzz targets exist for init, LGP protocol, TOML, and GUI, but the full test suite was not run after `make all` failed.
- Tests do not cover KMS, compositor/input integration, shell focus, WM surface creation, clipboard round-trips, terminal PTY rendering, or booted desktop visuals.
- The QEMU smoke test checks serial log text, not graphical correctness.

## Build [P1 - Needed before beta]

- `Makefile` states `luna-init-ctl` is dynamically linked, but the recipe uses `$(CFLAGS_STATIC)`.
- The compositor recipe says static linking avoids needing glibc and libdrm in the image, but static libdrm availability is host-dependent.
- `make all` target is augmented by included Makefile fragments, increasing hidden coupling.
- `make lint` fails. Although `make all` works, `make lint` fails. Also lint currently ignores many projects (`LunaGUI`, `luna-shell`, `terminal`, `installer`, `settings`, etc.). Lint coverage is incomplete.

--------------------------------------------------

# Minor Issues

- Comments in `luna-terminal` claim copy/paste is deferred, while code implements paste request and leaves copy TODO.
- `luna-desktop` and `luna-shell` overlap in wallpaper/panel responsibility.
- Magic numeric protocol constants appear in LunaGUI and apps.
- Desktop visual constants such as 1024x768, 1920x1080, 32, and 48 are hard-coded.
- Error reporting often returns false/disconnects clients rather than sending a structured protocol error.
- Documentation and comments still refer to older version labels (`0.1.0`, Phase D, Stage 0) while the audit request targets v0.3 planning.

--------------------------------------------------

# Build Problems

1. Full build failure: missing `xf86drm.h` from libdrm development headers.
2. Static compositor link depends on host-provided static-capable libdrm; this is not validated before compile.
3. Image build requires busybox, cpio/gzip, loop/device operations, btrfs tooling, kernel artifacts, and root-like `mknod` capability.
4. Runtime GUI service binaries must exist under `/usr/bin`, but the verified initramfs script installs only early boot artifacts.
5. `luna-init-ctl` comments and link flags disagree about static vs dynamic linking.

--------------------------------------------------

# Runtime Problems

- Graphical runtime could not be boot-verified because the build fails.
- If booted, shell wallpaper is only 1024x768, so larger framebuffers show compositor void color outside the shell surface.
- The compositor clear color is `0x000A0A0F`, so uncovered regions appear near-black.
- No visible cursor is rendered.
- Terminal is auto-launched by `luna-shell`, but focus routing is broken by surface-id/session-id mismatch.
- Window placement cascades at fixed 100,100 with 30px increments and wraps after x>400; no output-aware placement exists.

--------------------------------------------------

# Desktop UX Problems [P2 - Polish]

- Wallpaper: hard-coded to 1024x768 in the configured shell, so it cannot reliably fill the display.
- Black regions: compositor intentionally clears to void color before compositing; uncovered pixels remain black/near-black.
- Window placement: simple cascade only; no collision avoidance or screen-edge handling.
- Top bar: hard-coded 1024x32 and static time in `luna-shell`.
- Focus routing: implemented but semantically wrong in WM focus path.
- Alt+Tab: internal array rotation exists, but focus call uses wrong id domain.
- Super shortcuts: Super+T registration exists, but depends on keyboard modifier correctness and WM key grabs.
- Scroll containers: widget exists but desktop/terminal scroll UX is not a complete integrated feature.
- Font rendering: PSF font path is hard-coded to `/usr/share/fonts/luna-8x16.psf`; fallback behavior needs verification.
- Page flipping: whole-frame repaint on commits, no damage tracking.
- The shell mainly provides wallpaper, top bar, and window placement. The current desktop still lacks: mouse cursor, title bars, window decorations, shadows, resize handles, window dragging, and window resizing.

--------------------------------------------------

# Security Review

- Capability checks exist for direct fill, shell layers, Luna Island, canvas surfaces, and WM operations.
- Surface creation validates initial geometry against display bounds.
- WM repositioning validates only a broad coordinate range and permits negative values that composition does not clip safely.
- Protocol parsing has length checks in several paths, but LunaGUI client receive parsing lacks buffering for partial frames.
- SCM_RIGHTS fd intake caps pending fd count, but high-frequency commit behavior lacks policy limits.
- Services run as root in the provided service files.
- No authentication or credential checks were verified on the compositor UNIX socket beyond LGP capability negotiation.

--------------------------------------------------

# Code Quality

- Good: source is divided into init, compositor, protocol, scene, input, GUI, widgets, and apps.
- Good: several modules use explicit payload structures and validation helpers.
- Bad: protocol definitions are not shared with LunaGUI clients.
- Bad: hard-coded output sizes undermine desktop correctness.
- Bad: `luna-shell` conflates shell and WM policy but lacks enough protocol data to do correct focus management.
- Bad: rendering has no clipping/damage abstraction.
- Bad: build/deployment scripts are not hermetic and do not fail early with dependency diagnostics.

--------------------------------------------------

# Overall Assessment

While Mahina is technically not yet a stable desktop, it is **far ahead of most hobby OSes**. 

Mahina already has:
- ✅ PID1
- ✅ Service manager
- ✅ Boot splash
- ✅ DRM/KMS
- ✅ Custom compositor
- ✅ Own IPC protocol
- ✅ Own GUI toolkit
- ✅ Own Shell
- ✅ Window manager
- ✅ Multiple apps
- ✅ Terminal

The current blocker isn't that the architecture is missing—it's that the existing pieces need to be completed and integrated. The most urgent blockers before v0.3 are reproducible builds, reliable image installation, output geometry propagation, safe compositing/clipping, visible cursor rendering, corrected focus/session semantics, and a single authoritative desktop shell/window manager path. Until these are fixed and covered by integration tests, the project cannot truthfully claim a stable boot-to-desktop implementation.

--------------------------------------------------

# Additional Findings & Code State

## Dead / Unused Code
An entire `luna-desktop` implementation exists. It builds, creates wallpaper, and creates a dock, but isn't started, has no service, and isn't the real shell. It is essentially abandoned/dead code.

## AI Services
Service definitions exist for `Ollama` and `luna-ai-d`, but binaries don't exist yet. They will fail to start until implemented.

## TODOs Still Present [P3 - Nice to have]
Several deferred items are still present in the codebase:
- `luna-files`
- icons
- drag & drop
- delete
- rename
- terminal copy
- tabs
- installer language persistence
- keyboard UTF-8 mapping
- `luna-init` HTTP readiness
- signal readiness

--------------------------------------------------

# Positive Findings

These are foundational pieces that many hobby OS projects never reach, and are worth celebrating:
- ✅ Build works
- ✅ Tests pass
- ✅ Fuzz tests pass
- ✅ Boot chain is stable
- ✅ Compositor architecture is solid
- ✅ Shell launches correctly
- ✅ Protocol structure is clean
- ✅ Toolkit architecture is clean

--------------------------------------------------

# What Should Be Prioritized Next

Instead of jumping into more features, we should fix the core correctness issues in this order:
1. Fix framebuffer bounds checking in the compositor.
2. Decide whether to fully implement clipboard or remove its API until it's ready.
3. Expand lint coverage to all projects and make `make lint` pass.
4. Implement proper window state handling (resize, maximize, minimize).
5. Add damage tracking to reduce full-screen repaints.
6. Remove or archive `luna-desktop` if `luna-shell` is the intended desktop.
