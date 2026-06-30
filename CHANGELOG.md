# Changelog

## v0.1

- Initial bootloader
- luna-init
- Service Manager
- TOML parser
- Dependency graph
- Framebuffer console

## v0.2

- luna-splash
- Boot progress

## Unreleased

- Started Phase 2 graphics bring-up with `lgp-compositor` DRM/KMS setup, LGP socket lifecycle, 6-byte TLV framing per DL-053, and `LGP_HELLO` handling.
- Fixed compositor client lifecycle so handshaken clients remain tracked until disconnect.
- Added bounded LGP stream reassembly so split socket reads are buffered and parsed as complete TLV frames.
- Added the Phase 2 minimum LGP surface path: `LGP_CREATE_SURFACE`, `LGP_DESTROY_SURFACE`, `LGP_COMMIT_BUFFER`, compositor-owned surface state, `SCM_RIGHTS` shared-memory commits, and a direct-LGP test client.
- Added LGP protocol/surface unit tests and TLV fuzz coverage.
- Documented Mahina setup as GUI-first with terminal setup reserved for emergency and development use.
- Fixed TOML array validation, dependency graph cycle validation, and the clean-tree unit/fuzz test targets.
- Implemented surface Z-order and ARGB8888 software alpha blending in `lgp-compositor`.
- Defined input protocol events (`LGP_MSG_POINTER_MOTION`, etc.) in `tlv.h`.
- Created Phase 3 `LunaGUI` Toolkit (application loop, LGP IPC, window surface wrappers, canvas primitives, widget hierarchy, PSF font rendering).
- Scaffoled Phase 4 `luna-desktop` (shell dock and wallpaper).
- Scaffolded core GUI applications: `luna-installer`, `luna-terminal`, `luna-settings`, `luna-files`.
- Resolved `luna-init` supervisor TODOs regarding HTTP readiness networking constraints.

## 2026-06-30 — Engineering Sprint (Autonomous Session)

### Phase A: Critical Bug Fixes

- **fix(compositor): mouse clamping against live display dimensions** — `lgp_mouse_init()` now requires DRM dimensions to be set first; cursor clamping uses the actual detected screen resolution instead of the hardcoded 1920×1080.
- **fix(compositor): add LGP_CAP_DIRECT_LGP capability check to FILL_RECT handler** — unauthenticated clients can no longer fill arbitrary screen regions; handshake-stage capability verification enforced.
- **feat(gui): add lgui_widget_destroy() with recursive tree cleanup** — prevents accumulated memory leaks during multi-page application navigation; all page-transition code now calls this before building the next page.
- **fix(gui): lgui_window_update() handles root widget replacement correctly** — the old root is no longer accessed after being passed to lgui_widget_destroy().
- **feat(gui): implement lgui_font_init() startup call** — font subsystem is initialized before any widget rendering begins.

### Phase B: LunaGUI Toolkit Expansion

- **feat(gui): production-quality input event routing** — `POINTER_MOTION` events decoded from LGP wire format, cursor position tracked in `lgui_application_t`. `POINTER_BUTTON` events routed through `lgui_widget_hittest()` (recursive depth-first search using absolute screen coordinates) to the deepest widget under the cursor, triggering `on_click` callbacks. Buttons are now fully interactive end-to-end.
- **feat(gui): Mahina Design System renderer** — `render.c` completely rewritten with a curated 6-color dark palette (`MAHINA_BG_SURFACE`, `MAHINA_BG_RAISED`, `MAHINA_ACCENT_PRIMARY`, etc.). VBox and HBox layout engines handle padding and flex distribution. Buttons render with fill + accent border + top highlight + centered text label.
- **feat(gui): expanded public API** — `LGUI_LAYER_*` constants, `lgui_canvas_draw_rect_outline()`, `lgui_widget_set_size()`, `lgui_application_quit()`, full documentation comments on every exported function.

### Phase C: Installer

- **feat(installer): full 10-page graphical installer** — Welcome, Language, Keyboard, Timezone, Disk Selection, Partitioning, User Creation, Summary, Installation Progress (simulated), and Complete/Reboot. Each page uses `lgui_widget_destroy()` on the previous tree before building the next. Navigation buttons wired with correct page-index callbacks.

### Phase D: Terminal

- **feat(terminal): ANSI/VT100 escape sequence parser** (`ansi.c`, `ansi.h`):
  - State machine: GROUND → ESCAPE → CSI → CSI_PARAM
  - `term_cell_t` with per-cell fg/bg color (XRGB8888), bold, dirty flags
  - 16-color palette (8 base + 8 bright) tuned to Mahina dark theme
  - Cursor movement: A/B/C/D (relative), H/f (absolute 1-indexed)
  - Erase: J 0/1/2 (display), K 0/1/2 (line)
  - SGR: colors 30–37/40–47 (8-color) + 90–97/100–107 (bright), bold (1/22), reset (0)
  - Cursor visibility: `?25h/l`
  - Control characters: `\r`, `\n`, `\b`, `\t`
  - Scrollback: 2000-line circular buffer; top row pushed on scroll
  - `term_grid_resize()` with cursor clamping for SIGWINCH / PTY resize
- **feat(terminal): PTY resize via TIOCSWINSZ** — shell column width stays in sync with displayed terminal dimensions.
- **feat(terminal): clean shell teardown** — SIGHUP to shell process on application quit.

### Phase E (Partial): Compositor Hardening

- **fix(compositor): capability guard on FILL_RECT** — only clients with `LGP_CAP_DIRECT_LGP` flag in the HELLO handshake can issue direct pixel fill operations.

### Priority 7: Core Desktop Applications (all 6 implemented)

- **luna-settings** — 6-page settings app. Pages: Display, Network, Audio, Users & Accounts, About System. Reads current user from `getenv(USER)`. Widget tree freed on every page transition.
- **luna-files** — POSIX file manager. Uses `opendir()`/`readdir()` (no external libs). Sorts entries (directories first, then alphabetically). Header bar with ↑ Up and ⌂ Home buttons. Directory navigation on click. Entry count overflow indicator.
- **luna-calc** — 4-function calculator. State machine: accumulator + pending_op + new_input flag. Decimal point guard, backspace, clear. Callbacks use static char addresses (zero allocation, no dangling pointers).
- **luna-text** — Text file viewer. `fopen()` reads up to 2000 lines from `argv[1]`. Page-based scroll (14 lines/page). Widget tree rebuilt and freed on each navigation.
- **luna-about** — About Mahina dialog. Reads `/proc/version`, `/proc/cpuinfo`, `/proc/meminfo`, `uname(2)` for live system data: hostname, architecture, CPU model, total RAM, kernel version.
- **luna-tasks** — Task manager. Reads `/proc/<pid>/status` for all running processes. Shows PID, name, state, VmRSS. Kill button sends SIGTERM. Refresh re-reads `/proc`. Uses `intptr_t` for PID in button `user_data` (zero dynamic allocation).

### Documentation

- **IMPLEMENTATION_STATUS.md** updated: dashboard reflects new completion percentages, LunaGUI section expanded with per-subsystem status, new sections added for installer, terminal, and all 6 applications.
- **CHANGELOG.md** updated (this entry).

### Git Commits

- `fix(compositor): mouse resolution clamping + FILL_RECT capability check`
- `feat(lunagui): add input routing, improve renderer and public API`
- `feat(installer): full 10-page graphical installer`
- `feat(terminal): implement ANSI/VT100 parser with scrollback and PTY resize`
- `feat(apps): implement core desktop applications (Priority 7)`
