/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * lunagui.h — Public API for the LunaGUI toolkit.
 *
 * LunaGUI is the native GUI toolkit for MahinaOS. It communicates exclusively
 * with the lgp-compositor over the LGP protocol (DL-053). No third-party
 * graphical frameworks are used.
 *
 * Usage:
 *   lgui_application_t *app = lgui_application_create("my-app");
 *   lgui_window_t *win = lgui_window_create(app, 800, 600, LGP_LAYER_APPLICATION);
 *   lgui_window_set_root_widget(win, some_widget);
 *   lgui_window_show(win);
 *   lgui_application_run(app);
 *   lgui_application_destroy(app);
 */

#ifndef MAHINA_LUNAGUI_H
#define MAHINA_LUNAGUI_H

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

/* ── Layer constants (must match lgp-compositor LGP_LAYER_* values) ───────── */
#define LGUI_LAYER_WALLPAPER    0u
#define LGUI_LAYER_APPLICATION  100u
#define LGUI_LAYER_SHELL        200u
#define LGUI_LAYER_OVERLAY      300u
#define LGUI_LAYER_NOTIFICATION 400u
#define LGUI_LAYER_LUNA_ISLAND  500u
#define LGUI_LAYER_SYSTEM_MODAL 600u
#define LGUI_LAYER_CURSOR       700u

/* ── Opaque type forward declarations ─────────────────────────────────────── */
typedef struct lgui_application_t lgui_application_t;
typedef struct lgui_window_t      lgui_window_t;
typedef struct lgui_widget_t      lgui_widget_t;
typedef struct lgui_canvas_t      lgui_canvas_t;

/* ── Application ──────────────────────────────────────────────────────────── */

typedef void (*lgui_fd_callback_t)(int fd, void *user_data);

/*
 * lgui_application_create() — Connect to lgp-compositor, perform the LGP
 * HELLO handshake, and initialise the application event loop.
 * Returns NULL if the compositor is unreachable or rejects the handshake.
 */
lgui_application_t *lgui_application_create(const char *name);

/*
 * lgui_application_destroy() — Disconnect from the compositor and free all
 * resources. Does not free windows or widgets (caller's responsibility).
 */
void lgui_application_destroy(lgui_application_t *app);

/*
 * lgui_application_run() — Enter the event loop. Blocks until the compositor
 * disconnects or the application calls lgui_application_quit().
 * Returns 0 on clean exit, -1 on error.
 */
int lgui_application_run(lgui_application_t *app);

/*
 * lgui_application_quit() — Request the event loop to stop cleanly.
 */
void lgui_application_quit(lgui_application_t *app);

/*
 * lgui_application_add_fd() — Register a file descriptor to be polled in the
 * event loop. cb is called whenever the fd has data available. Up to 8 fds.
 */
void lgui_application_add_fd(lgui_application_t *app, int fd,
                              lgui_fd_callback_t cb, void *user_data);

/* ── Canvas ───────────────────────────────────────────────────────────────── */

/*
 * lgui_canvas_create() — Wrap a raw pixel buffer.
 * pixels  — pointer to XRGB8888 pixel data
 * stride  — bytes per row (usually width * 4)
 */
lgui_canvas_t *lgui_canvas_create(void *pixels, uint32_t width,
                                   uint32_t height, uint32_t stride);
void lgui_canvas_destroy(lgui_canvas_t *canvas);

/*
 * lgui_canvas_fill_rect() — Fill a rectangle with a solid ARGB color.
 * Out-of-bounds coordinates are clamped automatically.
 */
void lgui_canvas_fill_rect(lgui_canvas_t *canvas,
                            int x, int y, int w, int h, uint32_t color);

/*
 * lgui_canvas_draw_rect_outline() — Draw a 1-pixel-wide rectangle outline.
 */
void lgui_canvas_draw_rect_outline(lgui_canvas_t *canvas,
                                    int x, int y, int w, int h, uint32_t color);

/*
 * lgui_canvas_draw_text() — Render text using the embedded PSF bitmap font.
 * Requires lgui_font_init() to have been called during application startup.
 */
void lgui_canvas_draw_text(lgui_canvas_t *canvas, int x, int y,
                            const char *text, uint32_t color);

/* ── Window ───────────────────────────────────────────────────────────────── */

/*
 * lgui_window_create() — Allocate a shared-memory surface for a window.
 * Does NOT send anything to the compositor until lgui_window_show() is called.
 * layer — one of the LGUI_LAYER_* constants.
 */
lgui_window_t *lgui_window_create(lgui_application_t *app,
                                   uint32_t width, uint32_t height,
                                   uint32_t layer);

/*
 * lgui_window_set_root_widget() — Attach a widget as the root of this window's
 * widget tree. Marks the window dirty so it will be repainted on the next
 * event loop cycle.
 */
void lgui_window_set_root_widget(lgui_window_t *win, lgui_widget_t *widget);

/*
 * lgui_window_show() — Send CREATE_SURFACE to the compositor, paint the
 * initial widget tree, and send COMMIT_BUFFER to display the window.
 * Synchronous — blocks until the compositor sends a reply.
 */
void lgui_window_show(lgui_window_t *win);

/*
 * lgui_window_update() — Re-render the widget tree and send a new
 * COMMIT_BUFFER to the compositor. Safe to call from callbacks.
 */
void lgui_window_update(lgui_window_t *win);

/* ── Widgets ──────────────────────────────────────────────────────────────── */

typedef void (*lgui_button_click_cb)(lgui_widget_t *button, void *user_data);

/* Label */
lgui_widget_t *lgui_label_create(const char *text);
void           lgui_label_set_text(lgui_widget_t *label, const char *text);

/* Button */
lgui_widget_t *lgui_button_create(const char *text);
void           lgui_button_set_on_click(lgui_widget_t *button,
                                         lgui_button_click_cb cb, void *user_data);

/* Layout containers */
lgui_widget_t *lgui_vbox_create(void);
lgui_widget_t *lgui_hbox_create(void);
void           lgui_box_add_child(lgui_widget_t *box, lgui_widget_t *child);

/* Widget sizing */
void lgui_widget_set_size(lgui_widget_t *widget, int width, int height);

/* Memory management */
void lgui_widget_destroy(lgui_widget_t *widget);

#endif
