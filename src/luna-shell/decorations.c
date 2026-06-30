/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * luna-shell/decorations.c — Window decorations and drag state machine.
 *
 * Decorations are painted into the shell overlay surface (LGUI_LAYER_SHELL),
 * which composites above all application surfaces. Each decorated window gets
 * a 24px titlebar drawn at (win.x, win.y - DECO_TITLEBAR_H) in the overlay.
 *
 * Visual style (v0.3 spec):
 *   - Titlebar: dark acrylic #0D0D18 with 1px purple border #8A2BE2
 *   - Focused:  magenta left accent bar #E03E8A
 *   - Buttons:  close [×] and minimize [–]
 *   - Drag:     hold mouse on titlebar → move via lgui_wm_set_surface_position
 */

#include "decorations.h"

#include <string.h>
#include <stdio.h>

/* ── Colour palette ────────────────────────────────────────────────────────*/
#define DECO_BG         0x0D0D18u  /* Acrylic dark */
#define DECO_BORDER     0x8A2BE2u  /* Purple border */
#define DECO_FOCUSED    0xE03E8Au  /* Magenta accent */
#define DECO_TEXT       0xC8C8E0u  /* Soft white title */
#define DECO_BTN_CLR    0xFF4060u  /* Close button red */
#define DECO_BTN_MIN    0xF0A820u  /* Minimize button yellow */
#define DECO_UNFOCUSED  0x4A3A5Au  /* Unfocused border */

/* ── Public API ────────────────────────────────────────────────────────────*/

void deco_init(deco_state_t *d) {
    if (!d) return;
    memset(d, 0, sizeof(*d));
    d->drag_win_idx = -1;
}

static int deco_find(deco_state_t *d, uint32_t surface_id) {
    for (int i = 0; i < d->win_count; i++) {
        if (d->wins[i].surface_id == surface_id) return i;
    }
    return -1;
}

void deco_add_window(deco_state_t *d, uint32_t surface_id, uint32_t w, uint32_t h) {
    if (!d || d->win_count >= DECO_MAX_WINS) return;
    if (deco_find(d, surface_id) >= 0) return; /* Already tracked */
    int idx = d->win_count++;
    d->wins[idx].surface_id = surface_id;
    d->wins[idx].w = w;
    d->wins[idx].h = h;
    d->wins[idx].x = 0;
    d->wins[idx].y = 0;
    d->wins[idx].focused = false;
    snprintf(d->wins[idx].title, sizeof(d->wins[idx].title), "Window %u", surface_id);
}

void deco_remove_window(deco_state_t *d, uint32_t surface_id) {
    if (!d) return;
    int idx = deco_find(d, surface_id);
    if (idx < 0) return;
    /* Swap-erase */
    d->wins[idx] = d->wins[d->win_count - 1];
    d->win_count--;
    if (d->dragging && d->drag_win_idx == idx) {
        d->dragging = false;
    }
}

void deco_update_position(deco_state_t *d, uint32_t surface_id, int x, int y) {
    if (!d) return;
    int idx = deco_find(d, surface_id);
    if (idx < 0) return;
    d->wins[idx].x = x;
    d->wins[idx].y = y;
}

void deco_set_title(deco_state_t *d, uint32_t surface_id, const char *title) {
    if (!d || !title) return;
    int idx = deco_find(d, surface_id);
    if (idx < 0) return;
    strncpy(d->wins[idx].title, title, sizeof(d->wins[idx].title) - 1);
}

void deco_set_focused(deco_state_t *d, uint32_t surface_id) {
    if (!d) return;
    for (int i = 0; i < d->win_count; i++) {
        d->wins[i].focused = (d->wins[i].surface_id == surface_id);
    }
}

/* ── Hit testing ───────────────────────────────────────────────────────────*/

/* Returns window index if (mx, my) is within its titlebar, else -1 */
static int titlebar_hittest(const deco_state_t *d, int mx, int my) {
    for (int i = 0; i < d->win_count; i++) {
        const deco_window_t *win = &d->wins[i];
        int tx = win->x;
        int ty = win->y - DECO_TITLEBAR_H;
        int tw = (int)win->w;
        int th = DECO_TITLEBAR_H;
        if (mx >= tx && mx < tx + tw && my >= ty && my < ty + th) return i;
    }
    return -1;
}

void deco_pointer_down(deco_state_t *d, int mx, int my, lgui_application_t *app) {
    (void)app;
    if (!d) return;
    int idx = titlebar_hittest(d, mx, my);
    if (idx < 0) return;

    /* Check if click is on close button */
    const deco_window_t *win = &d->wins[idx];
    int close_x = win->x + (int)win->w - DECO_BTN_W - 4;
    int btn_y   = win->y - DECO_TITLEBAR_H + (DECO_TITLEBAR_H - DECO_BTN_W) / 2;
    if (mx >= close_x && mx < close_x + DECO_BTN_W &&
        my >= btn_y && my < btn_y + DECO_BTN_W) {
        /* Close: just remove the decoration (compositor will clean up on disconnect) */
        deco_remove_window(d, win->surface_id);
        return;
    }

    /* Begin drag */
    d->dragging      = true;
    d->drag_win_idx  = idx;
    d->drag_start_mx = mx;
    d->drag_start_my = my;
    d->drag_start_wx = d->wins[idx].x;
    d->drag_start_wy = d->wins[idx].y;
}

void deco_pointer_move(deco_state_t *d, int mx, int my, lgui_application_t *app) {
    if (!d || !d->dragging || d->drag_win_idx < 0) return;
    if (d->drag_win_idx >= d->win_count) { d->dragging = false; return; }

    deco_window_t *win = &d->wins[d->drag_win_idx];
    int new_x = d->drag_start_wx + (mx - d->drag_start_mx);
    int new_y = d->drag_start_wy + (my - d->drag_start_my);

    /* Clamp: don't allow dragging fully off-screen */
    if (new_x < -20) new_x = -20;
    if (new_y < DECO_TITLEBAR_H) new_y = DECO_TITLEBAR_H;

    win->x = new_x;
    win->y = new_y;

    lgui_wm_set_surface_position(app, win->surface_id, new_x, new_y);
}

void deco_pointer_up(deco_state_t *d) {
    if (!d) return;
    d->dragging     = false;
    d->drag_win_idx = -1;
}

/* ── Rendering ─────────────────────────────────────────────────────────────*/

void deco_render(const deco_state_t *d, lgui_canvas_t *canvas) {
    if (!d || !canvas) return;

    for (int i = 0; i < d->win_count; i++) {
        const deco_window_t *win = &d->wins[i];
        int tx = win->x;
        int ty = win->y - DECO_TITLEBAR_H;
        int tw = (int)win->w;

        if (ty < 0) ty = 0; /* Clamp to top of screen */

        /* Titlebar background */
        lgui_canvas_fill_rect(canvas, tx, ty, tw, DECO_TITLEBAR_H, DECO_BG);

        /* Focused accent bar (left 3px strip in magenta) */
        if (win->focused) {
            lgui_canvas_fill_rect(canvas, tx, ty, 3, DECO_TITLEBAR_H, DECO_FOCUSED);
        }

        /* Title bar border (bottom 1px) */
        lgui_canvas_fill_rect(canvas, tx, ty + DECO_TITLEBAR_H - 1, tw, 1,
                              win->focused ? DECO_FOCUSED : DECO_BORDER);

        /* Outer border of the titlebar */
        lgui_canvas_draw_rect_outline(canvas, tx, ty, tw, DECO_TITLEBAR_H,
                                      win->focused ? DECO_FOCUSED : DECO_UNFOCUSED);

        /* Window title text (left-aligned, after the accent bar) */
        lgui_canvas_draw_text(canvas, tx + 8, ty + (DECO_TITLEBAR_H - 8) / 2,
                              win->title, DECO_TEXT);

        /* Close button [×] */
        int btn_y  = ty + (DECO_TITLEBAR_H - DECO_BTN_W) / 2;
        int close_x = tx + tw - DECO_BTN_W - 4;
        lgui_canvas_fill_rect(canvas, close_x, btn_y, DECO_BTN_W, DECO_BTN_W, DECO_BTN_CLR);
        lgui_canvas_draw_text(canvas, close_x + 4, btn_y + (DECO_BTN_W - 8) / 2, "x",
                              0xFFFFFFu);

        /* Minimize button [–] */
        int min_x = close_x - DECO_BTN_W - 4;
        lgui_canvas_fill_rect(canvas, min_x, btn_y, DECO_BTN_W, DECO_BTN_W, DECO_BTN_MIN);
        lgui_canvas_draw_text(canvas, min_x + 4, btn_y + (DECO_BTN_W - 8) / 2, "-",
                              0xFFFFFFu);
    }
}
