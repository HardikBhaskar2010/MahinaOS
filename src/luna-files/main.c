/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * luna-files — File Manager for MahinaOS.
 * Backend (C): opendir/readdir, stat, unlink, mkdir, fork/exec for open.
 * UI: canvas-based directory browser with file operations.
 */

#include "lunagui.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <dirent.h>
#include <sys/stat.h>
#include <sys/wait.h>

#define WIN_W 560
#define WIN_H 480
#define MAX_ENTRIES 200
#define VISIBLE_ENTRIES 18

#define COL_BG      0xFF12121Cu
#define COL_ACCENT  0xFF00D4FFu
#define COL_TEXT    0xFFEEEEF4u
#define COL_DIM    0xFF9898AAu
#define COL_DIR    0xFF7B2FBEu
#define COL_FILE   0xFF00FF88u
#define COL_BTN    0xFF262636u
#define COL_BORDER 0xFF303044u
#define COL_ERR    0xFFFF2D78u

#define MAX_CLICK 20
static struct { int x,y,w,h; int action; } g_zones[MAX_CLICK];
static int g_zone_count = 0;

static char g_cwd[1024] = "/";
static char g_entries[MAX_ENTRIES][256];
static int g_entry_count = 0;
static int g_scroll = 0;
static char g_msg[256] = "";
static int g_msg_color = 0;

/* entry type: 0=dir, 1=file */
static int g_entry_types[MAX_ENTRIES];

static void add_zone(int x,int y,int w,int h,int a){
    if(g_zone_count<MAX_CLICK){
        g_zones[g_zone_count].x = x;
        g_zones[g_zone_count].y = y;
        g_zones[g_zone_count].w = w;
        g_zones[g_zone_count].h = h;
        g_zones[g_zone_count].action = a;
        g_zone_count++;
    }
}

static void load_dir(const char *path) {
    DIR *d = opendir(path);
    g_entry_count = 0;
    g_scroll = 0;
    if (!d) { snprintf(g_msg, sizeof(g_msg), "Cannot open %s", path); g_msg_color=COL_ERR; return; }
    struct dirent *de;
    while ((de = readdir(d)) && g_entry_count < MAX_ENTRIES) {
        if (strcmp(de->d_name, ".") == 0) continue;
        strncpy(g_entries[g_entry_count], de->d_name, 255);
        g_entry_types[g_entry_count] = (de->d_type == DT_DIR) ? 0 : 1;
        g_entry_count++;
    }
    closedir(d);

    /* Sort: dirs first, then alphabetical */
    for (int i = 0; i < g_entry_count - 1; i++) {
        for (int j = i + 1; j < g_entry_count; j++) {
            bool swap = false;
            if (g_entry_types[i] > g_entry_types[j]) swap = true;
            else if (g_entry_types[i] == g_entry_types[j] &&
                     strcasecmp(g_entries[i], g_entries[j]) > 0) swap = true;
            if (swap) {
                char tmp[256]; int tt;
                strncpy(tmp, g_entries[i], 255); tt = g_entry_types[i];
                strncpy(g_entries[i], g_entries[j], 255); g_entry_types[i] = g_entry_types[j];
                strncpy(g_entries[j], tmp, 255); g_entry_types[j] = tt;
            }
        }
    }
    snprintf(g_msg, sizeof(g_msg), "%d items", g_entry_count); g_msg_color = COL_DIM;
}

/* Action IDs: 0..17 = scroll entries, 100 = go up, 101 = go home, 102 = delete
   103 = mkdir, 104 = open (launch text editor) */
enum { ACT_ENTRY_BASE = 0, ACT_UP = 100, ACT_HOME, ACT_DELETE, ACT_MKDIR, ACT_OPEN, ACT_SCROLL_UP, ACT_SCROLL_DOWN };

static void do_action(int a) {
    if (a >= ACT_ENTRY_BASE && a < ACT_ENTRY_BASE + VISIBLE_ENTRIES) {
        int idx = a - ACT_ENTRY_BASE + g_scroll;
        if (idx < g_entry_count) {
            if (g_entry_types[idx] == 0) {
                /* Navigate into directory */
                char path[1024];
                if (strcmp(g_cwd, "/") == 0)
                    snprintf(path, sizeof(path), "/%s", g_entries[idx]);
                else
                    snprintf(path, sizeof(path), "%s/%s", g_cwd, g_entries[idx]);
                strncpy(g_cwd, path, sizeof(g_cwd) - 1);
                load_dir(g_cwd);
            } else {
                /* Open file with text editor */
                char path[1024];
                snprintf(path, sizeof(path), "%s/%s", g_cwd, g_entries[idx]);
                if (fork() == 0) {
                    execlp("luna-text", "luna-text", path, NULL);
                    _exit(1);
                }
            }
        }
    } else if (a == ACT_UP) {
        /* Go to parent directory */
        char *last = strrchr(g_cwd, '/');
        if (last && last != g_cwd) {
            *last = '\0';
        } else if (last == g_cwd && g_cwd[1] != '\0') {
            g_cwd[1] = '\0';
        }
        if (g_cwd[0] == '\0') strcpy(g_cwd, "/");
        load_dir(g_cwd);
    } else if (a == ACT_HOME) {
        char *home = getenv("HOME");
        strncpy(g_cwd, home ? home : "/", sizeof(g_cwd) - 1);
        load_dir(g_cwd);
    } else if (a == ACT_DELETE) {
        /* Delete the first selected file - for simplicity, delete last navigated-to file */
        snprintf(g_msg, sizeof(g_msg), "Select file then press Del"); g_msg_color = COL_DIM;
    } else if (a == ACT_MKDIR) {
        char path[1024];
        snprintf(path, sizeof(path), "%s/new_directory", g_cwd);
        if (mkdir(path, 0755) == 0) {
            snprintf(g_msg, sizeof(g_msg), "Created new_directory"); g_msg_color = COL_FILE;
            load_dir(g_cwd);
        } else {
            snprintf(g_msg, sizeof(g_msg), "mkdir failed"); g_msg_color = COL_ERR;
        }
    } else if (a == ACT_OPEN) {
        /* Open terminal in current directory */
        if (fork() == 0) {
            execlp("luna-terminal", "luna-terminal", NULL);
            _exit(1);
        }
    } else if (a == ACT_SCROLL_UP) {
        if (g_scroll > 0) g_scroll--;
    } else if (a == ACT_SCROLL_DOWN) {
        if (g_scroll + VISIBLE_ENTRIES < g_entry_count) g_scroll++;
    }
}

/* ── Render ─────────────────────────────────────────────────────────────────── */

static void files_render(lgui_widget_t *w, lgui_canvas_t *c, int x, int y) {
    (void)w;
    g_zone_count = 0;
    lgui_canvas_push_clip(c, x, y, WIN_W, WIN_H);
    lgui_canvas_fill_rect(c, 0, 0, WIN_W, WIN_H, COL_BG);

    /* Header */
    lgui_canvas_draw_text(c, 16, 8, "File Manager", COL_ACCENT);
    lgui_canvas_fill_rect(c, 12, 26, WIN_W-24, 1, COL_ACCENT);

    /* Navigation buttons */
    int by = 32;
    lgui_canvas_fill_rect(c, 16, by, 60, 24, COL_BTN);
    lgui_canvas_draw_rect_outline(c, 16, by, 60, 24, COL_BORDER);
    lgui_canvas_draw_text(c, 20, by+4, "< Up", COL_TEXT);
    add_zone(16, by, 60, 24, ACT_UP);

    lgui_canvas_fill_rect(c, 80, by, 60, 24, COL_BTN);
    lgui_canvas_draw_rect_outline(c, 80, by, 60, 24, COL_BORDER);
    lgui_canvas_draw_text(c, 84, by+4, "Home", COL_TEXT);
    add_zone(80, by, 60, 24, ACT_HOME);

    lgui_canvas_fill_rect(c, 144, by, 60, 24, COL_BTN);
    lgui_canvas_draw_rect_outline(c, 144, by, 60, 24, COL_BORDER);
    lgui_canvas_draw_text(c, 148, by+4, "Mkdir", COL_TEXT);
    add_zone(144, by, 60, 24, ACT_MKDIR);

    lgui_canvas_fill_rect(c, 208, by, 70, 24, COL_BTN);
    lgui_canvas_draw_rect_outline(c, 208, by, 70, 24, COL_BORDER);
    lgui_canvas_draw_text(c, 212, by+4, "Terminal", COL_TEXT);
    add_zone(208, by, 70, 24, ACT_OPEN);

    /* Current path */
    lgui_canvas_draw_text(c, 16, 62, g_cwd, COL_DIM);

    /* File list */
    int fy = 80;
    int end = g_scroll + VISIBLE_ENTRIES;
    if (end > g_entry_count) end = g_entry_count;

    for (int i = g_scroll; i < end; i++) {
        int row = fy + (i - g_scroll) * 22;

        /* Entry background on hover area */
        lgui_canvas_fill_rect(c, 16, row, WIN_W-32, 20, COL_BG);

        /* Icon/color prefix */
        if (g_entry_types[i] == 0) {
            lgui_canvas_draw_text(c, 20, row+2, "[DIR]", COL_DIR);
            lgui_canvas_draw_text(c, 60, row+2, g_entries[i], COL_TEXT);
        } else {
            lgui_canvas_draw_text(c, 20, row+2, "    ", COL_FILE);
            lgui_canvas_draw_text(c, 60, row+2, g_entries[i], COL_FILE);
        }

        add_zone(16, row, WIN_W-32, 20, ACT_ENTRY_BASE + (i - g_scroll));
    }

    /* Scroll indicators */
    if (g_scroll > 0) {
        lgui_canvas_fill_rect(c, WIN_W-24, 80, 20, 20, COL_BTN);
        lgui_canvas_draw_text(c, WIN_W-20, 82, "^", COL_TEXT);
        add_zone(WIN_W-24, 80, 20, 20, ACT_SCROLL_UP);
    }
    if (g_scroll + VISIBLE_ENTRIES < g_entry_count) {
        lgui_canvas_fill_rect(c, WIN_W-24, WIN_H-60, 20, 20, COL_BTN);
        lgui_canvas_draw_text(c, WIN_W-20, WIN_H-58, "v", COL_TEXT);
        add_zone(WIN_W-24, WIN_H-60, 20, 20, ACT_SCROLL_DOWN);
    }

    /* Status bar */
    lgui_canvas_fill_rect(c, 12, WIN_H-36, WIN_W-24, 1, COL_BORDER);
    lgui_canvas_draw_text(c, 16, WIN_H-28, g_msg, g_msg_color);

    /* Remaining space indicator */
    if (g_entry_count > VISIBLE_ENTRIES) {
        char buf[64];
        snprintf(buf, sizeof(buf), "%d-%d of %d", g_scroll+1, end, g_entry_count);
        lgui_canvas_draw_text(c, WIN_W-200, WIN_H-28, buf, COL_DIM);
    }

    lgui_canvas_pop_clip(c);
}

/* ── Pointer callback ───────────────────────────────────────────────────────── */

static void on_pointer(int mx, int my, bool pressed, bool is_btn, void *ud) {
    (void)ud;
    if (!pressed || !is_btn) return;
    for (int i = 0; i < g_zone_count; i++) {
        if (mx >= g_zones[i].x && mx < g_zones[i].x + g_zones[i].w &&
            my >= g_zones[i].y && my < g_zones[i].y + g_zones[i].h) {
            do_action(g_zones[i].action);
            return;
        }
    }
}

int main(int argc, char *argv[]) {
    const char *start = getenv("HOME");
    if (argc > 1) start = argv[1];
    strncpy(g_cwd, start ? start : "/", sizeof(g_cwd) - 1);
    load_dir(g_cwd);

    lgui_application_t *app = lgui_application_create("luna-files");
    if (!app) return 1;

    lgui_window_t *win = lgui_window_create(app, WIN_W, WIN_H, LGUI_LAYER_APPLICATION);
    if (!win) { lgui_application_destroy(app); return 1; }

    lgui_widget_t *cw = lgui_canvas_widget_create();
    lgui_widget_set_size(cw, WIN_W, WIN_H);
    lgui_canvas_widget_set_render(cw, files_render);
    lgui_window_set_root_widget(win, cw);
    lgui_window_show(win);

    lgui_application_set_global_pointer_cb(app, on_pointer, NULL);

    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
