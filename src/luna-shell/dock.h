/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * luna-shell/dock.h — Left sidebar application launcher dock.
 */

#ifndef LUNA_SHELL_DOCK_H
#define LUNA_SHELL_DOCK_H

#include <stdint.h>
#include <stdbool.h>
#include "../../luna-gui/include/lunagui.h"

#define DOCK_WIDTH   56
#define DOCK_ICON_SZ 40
#define DOCK_PADDING  8
#define DOCK_MAX_ITEMS 12

typedef struct {
    const char *label;     /* Short label (e.g., "Term") */
    const char *glyph;     /* 1-2 char icon glyph rendered large */
    const char *command;   /* execvp command */
} dock_item_def_t;

typedef struct {
    int          hovered;  /* Index of hovered item, or -1 */
    uint32_t     height;   /* Panel height (output height) */
    lgui_canvas_t *canvas; /* Drawing canvas */
} dock_state_t;

void dock_init(dock_state_t *d, uint32_t screen_height);
void dock_render(dock_state_t *d, lgui_canvas_t *canvas, uint32_t screen_height);
void dock_on_pointer(dock_state_t *d, int x, int y);
bool dock_on_click(dock_state_t *d, int x, int y);

extern const dock_item_def_t g_dock_items[];
extern const int              g_dock_item_count;

#endif /* LUNA_SHELL_DOCK_H */
