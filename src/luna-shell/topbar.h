/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * luna-shell/topbar.h — Top status panel (clock, workspace, system tray).
 */

#ifndef LUNA_SHELL_TOPBAR_H
#define LUNA_SHELL_TOPBAR_H

#include <stdint.h>
#include "../../luna-gui/include/lunagui.h"

#define TOPBAR_HEIGHT   32

typedef struct {
    uint32_t tick;
    char     clock_str[16];   /* "HH:MM:SS" from uptime */
    uint32_t cpu_percent;     /* 0-100, from /proc/stat */
    uint32_t mem_percent;     /* 0-100, from /proc/meminfo */
    int      workspace;       /* Current workspace index */
} topbar_state_t;

void topbar_init(topbar_state_t *t);
void topbar_tick(topbar_state_t *t);
void topbar_render(const topbar_state_t *t, lgui_canvas_t *canvas, uint32_t screen_width);

#endif /* LUNA_SHELL_TOPBAR_H */
