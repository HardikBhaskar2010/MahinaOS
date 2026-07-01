/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "lunagui.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/utsname.h>
#include <sys/statvfs.h>

#define WIN_W 500
#define WIN_H 440

#define COL_BG     0xFF12121Cu
#define COL_ACCENT 0xFF00D4FFu
#define COL_TEXT   0xFFEEEEF4u
#define COL_DIM   0xFF9898AAu
#define COL_GREEN  0xFF00FF88u
#define COL_PURPLE 0xFF7B2FBEu
#define COL_BORDER 0xFF303044u

static void about_render(lgui_widget_t *w, lgui_canvas_t *c, int x, int y) {
    (void)w;
    lgui_canvas_push_clip(c, x, y, WIN_W, WIN_H);
    lgui_canvas_fill_rect(c, 0, 0, WIN_W, WIN_H, COL_BG);

    /* Mahina logo area */
    lgui_canvas_fill_rect(c, 0, 0, WIN_W, 60, 0xFF0D0D14u);
    lgui_canvas_draw_text(c, 16, 8, "Mahina", COL_ACCENT);
    lgui_canvas_draw_text(c, 16, 28, "OS v0.1.0 (Waxing)", COL_DIM);
    lgui_canvas_draw_text(c, 16, 44, "Luna Island Desktop", COL_PURPLE);

    int y0 = 72;
    char info[256];

    /* System Info */
    lgui_canvas_draw_text(c, 16, y0, "System Information", COL_ACCENT);
    lgui_canvas_fill_rect(c, 16, y0+14, WIN_W-32, 1, COL_BORDER);
    y0 += 22;

    struct utsname u;
    uname(&u);

    lgui_canvas_draw_text(c, 16, y0, "OS:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, "Mahina OS 0.1.0", COL_TEXT);
    y0 += 18;

    lgui_canvas_draw_text(c, 16, y0, "Kernel:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, u.release, COL_TEXT);
    y0 += 18;

    lgui_canvas_draw_text(c, 16, y0, "Architecture:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, u.machine, COL_TEXT);
    y0 += 18;

    lgui_canvas_draw_text(c, 16, y0, "Hostname:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, u.nodename, COL_TEXT);
    y0 += 26;

    /* Hardware */
    lgui_canvas_draw_text(c, 16, y0, "Hardware", COL_ACCENT);
    lgui_canvas_fill_rect(c, 16, y0+14, WIN_W-32, 1, COL_BORDER);
    y0 += 22;

    /* CPU */
    FILE *f = fopen("/proc/cpuinfo", "r");
    if (f) {
        char line[256];
        while (fgets(line, sizeof(line), f)) {
            if (strncmp(line, "model name", 10) == 0) {
                char *p = strchr(line, ':');
                if (p) { p += 2; size_t l = strlen(p);
                while (l > 0 && (p[l-1]=='\n'||p[l-1]=='\r')) p[--l] = '\0';
                lgui_canvas_draw_text(c, 16, y0, "CPU:", COL_DIM);
                lgui_canvas_draw_text(c, 160, y0, p, COL_TEXT); }
                break;
            }
        }
        fclose(f);
    }
    y0 += 18;

    /* Count CPUs */
    int ncpu = 0;
    f = fopen("/proc/cpuinfo", "r");
    if (f) {
        char line[256];
        while (fgets(line, sizeof(line), f)) {
            if (strncmp(line, "processor", 9) == 0) ncpu++;
        }
        fclose(f);
    }
    snprintf(info, sizeof(info), "%d", ncpu);
    lgui_canvas_draw_text(c, 16, y0, "CPU Cores:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, info, COL_TEXT);
    y0 += 18;

    /* RAM */
    f = fopen("/proc/meminfo", "r");
    if (f) {
        char line[256];
        while (fgets(line, sizeof(line), f)) {
            unsigned long kb;
            if (sscanf(line, "MemTotal: %lu kB", &kb) == 1) {
                snprintf(info, sizeof(info), "%lu MB", kb / 1024);
                lgui_canvas_draw_text(c, 16, y0, "Memory:", COL_DIM);
                lgui_canvas_draw_text(c, 160, y0, info, COL_TEXT);
                break;
            }
        }
        fclose(f);
    }
    y0 += 26;

    /* Disk */
    lgui_canvas_draw_text(c, 16, y0, "Storage", COL_ACCENT);
    lgui_canvas_fill_rect(c, 16, y0+14, WIN_W-32, 1, COL_BORDER);
    y0 += 22;

    struct statvfs st;
    if (statvfs("/", &st) == 0) {
        unsigned long total = (unsigned long)(st.f_blocks * st.f_frsize);
        unsigned long avail = (unsigned long)(st.f_bavail * st.f_frsize);
        unsigned long used = total - avail;
        snprintf(info, sizeof(info), "%lu GB / %lu GB",
                 used / (1024*1024*1024), total / (1024*1024*1024));
        lgui_canvas_draw_text(c, 16, y0, "Disk:", COL_DIM);
        lgui_canvas_draw_text(c, 160, y0, info, COL_TEXT);
    }
    y0 += 18;

    /* Filesystem type */
    if (statvfs("/", &st) == 0) {
        lgui_canvas_draw_text(c, 16, y0, "Filesystem:", COL_DIM);
        lgui_canvas_draw_text(c, 160, y0, "Btrfs (Mahina Root)", COL_TEXT);
    }
    y0 += 26;

    /* Software */
    lgui_canvas_draw_text(c, 16, y0, "Software", COL_ACCENT);
    lgui_canvas_fill_rect(c, 16, y0+14, WIN_W-32, 1, COL_BORDER);
    y0 += 22;

    lgui_canvas_draw_text(c, 16, y0, "Compositor:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, "lgp-compositor v1.0", COL_TEXT);
    y0 += 18;

    lgui_canvas_draw_text(c, 16, y0, "GUI Toolkit:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, "LunaGUI v1.0", COL_TEXT);
    y0 += 18;

    lgui_canvas_draw_text(c, 16, y0, "Init System:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, "luna-init (PID 1)", COL_TEXT);
    y0 += 18;

    lgui_canvas_draw_text(c, 16, y0, "Bootloader:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, "Limine (UEFI)", COL_TEXT);
    y0 += 18;

    lgui_canvas_draw_text(c, 16, y0, "License:", COL_DIM);
    lgui_canvas_draw_text(c, 160, y0, "MIT License", COL_GREEN);

    lgui_canvas_pop_clip(c);
}

int main(void) {
    lgui_application_t *app = lgui_application_create("luna-about");
    if (!app) return 1;

    lgui_window_t *win = lgui_window_create(app, WIN_W, WIN_H, LGUI_LAYER_APPLICATION);
    if (!win) { lgui_application_destroy(app); return 1; }

    lgui_widget_t *cw = lgui_canvas_widget_create();
    lgui_widget_set_size(cw, WIN_W, WIN_H);
    lgui_canvas_widget_set_render(cw, about_render);
    lgui_window_set_root_widget(win, cw);
    lgui_window_show(win);

    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
