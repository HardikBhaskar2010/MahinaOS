/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef LUNAGUI_PRIVATE_H
#define LUNAGUI_PRIVATE_H

#include "lunagui.h"
#include "window_private.h"
#include <stdbool.h>

struct lgui_application_t {
    char name[64];
    int  lgp_fd;
    bool running;

    /* Capabilities granted by compositor during HELLO handshake */
    uint32_t caps_granted;

    /* Current pointer position (updated by POINTER_MOTION events) */
    int cursor_x;
    int cursor_y;

    struct lgui_window_t *windows[16];
    int window_count;

    struct {
        int              fd;
        lgui_fd_callback_t cb;
        void            *user_data;
    } custom_fds[8];
    int custom_fd_count;
};

/* Render a widget tree into a canvas at (offset_x, offset_y). */
void lgui_widget_render(lgui_canvas_t *canvas, lgui_widget_t *w, int offset_x, int offset_y);

/* font.c — must be called once before any text rendering. */
void lgui_font_init(const char *psf_path);

#endif
