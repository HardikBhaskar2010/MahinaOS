/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * luna-shell/decorations.h — Window decorations and drag state machine.
 */

#ifndef LUNA_SHELL_DECORATIONS_H
#define LUNA_SHELL_DECORATIONS_H

#include <stdint.h>
#include <stdbool.h>
#include "../../luna-gui/include/lunagui.h"

#define DECO_TITLEBAR_H 24   /* Height of the window titlebar in pixels */
#define DECO_BTN_W      16   /* Width of close/min buttons */
#define DECO_MAX_WINS   64   /* Max tracked decorated windows */

typedef struct {
    uint32_t surface_id;  /* Compositor surface id */
    int      x;           /* Current x position */
    int      y;           /* Current y position */
    uint32_t w;           /* Surface width */
    uint32_t h;           /* Surface height */
    char     title[64];   /* Window title text */
    bool     focused;
} deco_window_t;

typedef struct {
    deco_window_t wins[DECO_MAX_WINS];
    int           win_count;

    /* Drag state */
    bool    dragging;
    int     drag_win_idx;  /* Index into wins[] */
    int     drag_start_mx; /* Mouse X when drag started */
    int     drag_start_my; /* Mouse Y when drag started */
    int     drag_start_wx; /* Window X when drag started */
    int     drag_start_wy; /* Window Y when drag started */
} deco_state_t;

void deco_init(deco_state_t *d);
void deco_add_window(deco_state_t *d, uint32_t surface_id, uint32_t w, uint32_t h);
void deco_remove_window(deco_state_t *d, uint32_t surface_id);
void deco_update_position(deco_state_t *d, uint32_t surface_id, int x, int y);
void deco_set_title(deco_state_t *d, uint32_t surface_id, const char *title);
void deco_set_focused(deco_state_t *d, uint32_t surface_id);

/* Pointer events for drag */
void deco_pointer_down(deco_state_t *d, int mx, int my, lgui_application_t *app);
void deco_pointer_move(deco_state_t *d, int mx, int my, lgui_application_t *app);
void deco_pointer_up(deco_state_t *d);

/* Render all decorations into the shell overlay surface */
void deco_render(const deco_state_t *d, lgui_canvas_t *canvas);

#endif /* LUNA_SHELL_DECORATIONS_H */
