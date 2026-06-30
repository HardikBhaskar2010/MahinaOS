/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * luna-text — MahinaOS text viewer/editor.
 *
 * Phase 7 implementation:
 *   - Opens a file specified by argv[1], or shows a file chooser (TBD)
 *   - Displays file content line by line using label widgets
 *   - Scroll with ↑/↓ buttons (up to 60 lines visible, 14 at a time)
 *
 * Full edit capability (keystroke input) requires keyboard event routing
 * from lgp-compositor, which is being implemented in Phase E. This file
 * provides the read path and view UI.
 *
 * Decision: the viewport is page-based (not pixel scroll) because LunaGUI
 * does not yet have a scroll container widget. This will be refactored
 * when lgui_scroll_container_t lands (deferred, DL-TBD).
 */

#include "lunagui.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_LINES     2000
#define LINES_PER_PAGE  14
#define MAX_LINE_LEN   512

static lgui_application_t *g_app  = NULL;
static lgui_window_t      *g_win  = NULL;
static lgui_widget_t      *g_root = NULL;

static char g_filename[512] = "[No file]";
static char g_lines[MAX_LINES][MAX_LINE_LEN];
static int  g_line_count  = 0;
static int  g_view_offset = 0;   /* First visible line */

static void render_view(void);

/* ── Navigation ───────────────────────────────────────────────────────────── */

static void btn_page_up(lgui_widget_t *b, void *u) {
    (void)b; (void)u;
    g_view_offset -= LINES_PER_PAGE;
    if (g_view_offset < 0) g_view_offset = 0;
    render_view();
}

static void btn_page_down(lgui_widget_t *b, void *u) {
    (void)b; (void)u;
    if (g_line_count > LINES_PER_PAGE) {
        g_view_offset += LINES_PER_PAGE;
        if (g_view_offset > g_line_count - LINES_PER_PAGE)
            g_view_offset = g_line_count - LINES_PER_PAGE;
    }
    render_view();
}

/* ── Renderer ─────────────────────────────────────────────────────────────── */

static void render_view(void) {
    if (g_root) { lgui_widget_destroy(g_root); g_root = NULL; }

    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    /* Title bar */
    char title[600];
    snprintf(title, sizeof(title), "Text Editor — %s", g_filename);
    lgui_widget_t *title_lbl = lgui_label_create(title);
    lgui_widget_set_size(title_lbl, 800, 24);
    lgui_box_add_child(vbox, title_lbl);

    /* Line info */
    char info[128];
    snprintf(info, sizeof(info), "Lines %d–%d of %d",
             g_view_offset + 1,
             g_view_offset + LINES_PER_PAGE < g_line_count
                 ? g_view_offset + LINES_PER_PAGE : g_line_count,
             g_line_count);
    lgui_box_add_child(vbox, lgui_label_create(info));

    /* Content area */
    int end = g_view_offset + LINES_PER_PAGE;
    if (end > g_line_count) end = g_line_count;

    for (int i = g_view_offset; i < end; i++) {
        lgui_widget_t *lbl = lgui_label_create(g_lines[i]);
        lgui_widget_set_size(lbl, 790, 20);
        lgui_box_add_child(vbox, lbl);
    }

    /* Navigation row */
    lgui_widget_t *nav = lgui_hbox_create();
    lgui_widget_set_size(nav, 800, 40);

    if (g_view_offset > 0) {
        lgui_widget_t *up = lgui_button_create("↑ Page Up");
        lgui_button_set_on_click(up, btn_page_up, NULL);
        lgui_box_add_child(nav, up);
    }
    if (g_view_offset + LINES_PER_PAGE < g_line_count) {
        lgui_widget_t *dn = lgui_button_create("↓ Page Down");
        lgui_button_set_on_click(dn, btn_page_down, NULL);
        lgui_box_add_child(nav, dn);
    }
    lgui_box_add_child(vbox, nav);

    g_root = vbox;
    lgui_window_set_root_widget(g_win, g_root);
}

/* ── File loader ──────────────────────────────────────────────────────────── */

static int load_file(const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) return -1;

    g_line_count = 0;
    while (g_line_count < MAX_LINES) {
        char *ok = fgets(g_lines[g_line_count], MAX_LINE_LEN, f);
        if (!ok) break;
        /* Strip newline */
        size_t len = strlen(g_lines[g_line_count]);
        if (len > 0 && g_lines[g_line_count][len - 1] == '\n')
            g_lines[g_line_count][len - 1] = '\0';
        g_line_count++;
    }
    fclose(f);
    return 0;
}

/* ── Entry point ──────────────────────────────────────────────────────────── */

int main(int argc, char *argv[]) {
    g_app = lgui_application_create("luna-text");
    if (!g_app) return 1;

    g_win = lgui_window_create(g_app, 800, 600, LGUI_LAYER_APPLICATION);
    if (!g_win) { lgui_application_destroy(g_app); return 1; }

    if (argc >= 2) {
        strncpy(g_filename, argv[1], sizeof(g_filename) - 1);
        if (load_file(argv[1]) != 0) {
            snprintf(g_lines[0], MAX_LINE_LEN,
                     "Error: cannot open '%s'", argv[1]);
            g_line_count = 1;
        }
    } else {
        /* No file specified: show a placeholder */
        strncpy(g_filename, "[New File]", sizeof(g_filename) - 1);
        strncpy(g_lines[0], "Open a file by launching: luna-text <path>", MAX_LINE_LEN - 1);
        strncpy(g_lines[1], "", MAX_LINE_LEN - 1);
        strncpy(g_lines[2], "Keyboard input editing requires Phase E (keyboard events).", MAX_LINE_LEN - 1);
        g_line_count = 3;
    }

    render_view();
    lgui_window_show(g_win);

    lgui_application_run(g_app);
    lgui_application_destroy(g_app);
    return 0;
}
