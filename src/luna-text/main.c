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
#include <fcntl.h>
#include <sys/stat.h>

#define WIN_W 640
#define WIN_H 480
#define MAX_LINES 4000
#define MAX_LINE_LEN 256
#define VISIBLE_LINES 24

#define COL_BG     0xFF12121Cu
#define COL_LINE   0xFF1E1E28u
#define COL_ACCENT 0xFF00D4FFu
#define COL_TEXT   0xFFEEEEF4u
#define COL_DIM   0xFF9898AAu
#define COL_LINENO 0xFF555566u
#define COL_CURSOR 0xFF00D4FFu
#define COL_SEL   0xFF7B2FBEu
#define COL_BTN   0xFF262636u
#define COL_BORDER 0xFF303044u
#define COL_ERR   0xFFFF2D78u
#define COL_MODIFIED 0xFFFF2D78u

#define MAX_CLICK 16
static struct { int x,y,w,h; int action; } g_zones[MAX_CLICK];
static int g_zone_count = 0;

static char g_lines[MAX_LINES][MAX_LINE_LEN];
static int g_line_count = 0;
static int g_cursor_line = 0;
static int g_cursor_col = 0;
static int g_scroll_top = 0;
static bool g_modified = false;
static char g_filename[256] = "Untitled";
static char g_status[256] = "";
static bool g_insert_mode = true;

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

static void load_file(const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) { snprintf(g_status, sizeof(g_status), "Cannot open: %s", path); return; }
    g_line_count = 0;
    while (g_line_count < MAX_LINES && fgets(g_lines[g_line_count], MAX_LINE_LEN, f)) {
        size_t l = strlen(g_lines[g_line_count]);
        while (l > 0 && (g_lines[g_line_count][l-1]=='\n'||g_lines[g_line_count][l-1]=='\r'))
            g_lines[g_line_count][--l] = '\0';
        g_line_count++;
    }
    fclose(f);
    strncpy(g_filename, path, sizeof(g_filename) - 1);
    snprintf(g_status, sizeof(g_status), "Loaded %d lines", g_line_count);
}

static void save_file(void) {
    if (strcmp(g_filename, "Untitled") == 0) {
        strncpy(g_status, "No filename - use Save As", sizeof(g_status));
        return;
    }
    FILE *f = fopen(g_filename, "w");
    if (!f) { snprintf(g_status, sizeof(g_status), "Cannot write: %s", g_filename); return; }
    for (int i = 0; i < g_line_count; i++) {
        fprintf(f, "%s\n", g_lines[i]);
    }
    fclose(f);
    g_modified = false;
    snprintf(g_status, sizeof(g_status), "Saved %s", g_filename);
}

enum { ACT_SAVE=100, ACT_NEW, ACT_OPEN_FILE, ACT_CUT_LINE, ACT_PASTE_LINE };

static void do_action(int a) {
    switch(a) {
        case ACT_SAVE: save_file(); break;
        case ACT_NEW:
            g_line_count = 1; strcpy(g_lines[0], "");
            g_cursor_line = 0; g_cursor_col = 0;
            strcpy(g_filename, "Untitled"); g_modified = false;
            strcpy(g_status, "New file");
            break;
        case ACT_CUT_LINE:
            if (g_line_count > 0) {
                /* Move remaining lines up */
                for (int i = g_cursor_line; i < g_line_count - 1; i++)
                    strcpy(g_lines[i], g_lines[i+1]);
                g_line_count--;
                if (g_cursor_line >= g_line_count && g_line_count > 0)
                    g_cursor_line = g_line_count - 1;
                g_modified = true;
                strcpy(g_status, "Line cut");
            }
            break;
        case ACT_PASTE_LINE:
            /* Insert empty line at cursor */
            if (g_line_count < MAX_LINES) {
                for (int i = g_line_count; i > g_cursor_line; i--)
                    strcpy(g_lines[i], g_lines[i-1]);
                strcpy(g_lines[g_cursor_line], "");
                g_line_count++;
                g_modified = true;
                strcpy(g_status, "Line inserted");
            }
            break;
    }
}

static void editor_render(lgui_widget_t *w, lgui_canvas_t *c, int x, int y) {
    (void)w; g_zone_count = 0;
    lgui_canvas_push_clip(c, x, y, WIN_W, WIN_H);
    lgui_canvas_fill_rect(c, 0, 0, WIN_W, WIN_H, COL_BG);

    /* Title bar */
    char title[300];
    snprintf(title, sizeof(title), "%s%s - Text Editor",
             g_modified ? "* " : "", g_filename);
    lgui_canvas_fill_rect(c, 0, 0, WIN_W, 22, COL_LINE);
    lgui_canvas_draw_text(c, 8, 3, title, COL_ACCENT);

    /* Action buttons */
    lgui_canvas_fill_rect(c, WIN_W-180, 2, 50, 18, COL_BTN);
    lgui_canvas_draw_rect_outline(c, WIN_W-180, 2, 50, 18, COL_BORDER);
    lgui_canvas_draw_text(c, WIN_W-176, 4, "Save", COL_TEXT);
    add_zone(WIN_W-180, 2, 50, 18, ACT_SAVE);

    lgui_canvas_fill_rect(c, WIN_W-124, 2, 40, 18, COL_BTN);
    lgui_canvas_draw_rect_outline(c, WIN_W-124, 2, 40, 18, COL_BORDER);
    lgui_canvas_draw_text(c, WIN_W-120, 4, "Cut", COL_TEXT);
    add_zone(WIN_W-124, 2, 40, 18, ACT_CUT_LINE);

    lgui_canvas_fill_rect(c, WIN_W-80, 2, 50, 18, COL_BTN);
    lgui_canvas_draw_rect_outline(c, WIN_W-80, 2, 50, 18, COL_BORDER);
    lgui_canvas_draw_text(c, WIN_W-76, 4, "New", COL_TEXT);
    add_zone(WIN_W-80, 2, 50, 18, ACT_NEW);

    /* Editor area */
    int ey = 28;
    int lh = 18;
    lgui_canvas_fill_rect(c, 0, ey, WIN_W, WIN_H - ey - 20, COL_LINE);

    /* Ensure cursor is in view */
    if (g_cursor_line < g_scroll_top) g_scroll_top = g_cursor_line;
    if (g_cursor_line >= g_scroll_top + VISIBLE_LINES)
        g_scroll_top = g_cursor_line - VISIBLE_LINES + 1;

    for (int i = 0; i < VISIBLE_LINES && (g_scroll_top + i) < g_line_count; i++) {
        int ln = g_scroll_top + i;
        int ly = ey + i * lh;

        /* Line number */
        char num[16];
        snprintf(num, sizeof(num), "%4d", ln + 1);
        lgui_canvas_fill_rect(c, 0, ly, 40, lh, COL_BG);
        lgui_canvas_draw_text(c, 4, ly+1, num, COL_LINENO);

        /* Line content */
        lgui_canvas_draw_text(c, 44, ly+1, g_lines[ln], COL_TEXT);

        /* Cursor highlight */
        if (ln == g_cursor_line) {
            lgui_canvas_fill_rect(c, 0, ly, WIN_W, lh, 0xFF1A1A2Eu);
            /* Cursor line */
            int cx = 44 + g_cursor_col * 8;
            if (cx < WIN_W - 8) {
                lgui_canvas_fill_rect(c, cx, ly + 2, 2, lh - 4, COL_CURSOR);
            }
        }
    }

    /* Status bar */
    lgui_canvas_fill_rect(c, 0, WIN_H-20, WIN_W, 20, COL_BG);
    lgui_canvas_fill_rect(c, 0, WIN_H-20, WIN_W, 1, COL_BORDER);
    char status[300];
    snprintf(status, sizeof(status), "Ln %d Col %d | %s | %s",
             g_cursor_line+1, g_cursor_col+1, g_filename,
             g_insert_mode ? "INSERT" : "NORMAL");
    lgui_canvas_draw_text(c, 8, WIN_H-16, status, COL_DIM);

    if (g_status[0]) {
        lgui_canvas_draw_text(c, WIN_W-200, WIN_H-16, g_status, COL_DIM);
    }

    lgui_canvas_pop_clip(c);
}

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
    /* Click in editor area - move cursor */
    if (my >= 28 && my < WIN_H - 20) {
        int line_idx = g_scroll_top + (my - 28) / 18;
        if (line_idx >= 0 && line_idx < g_line_count) {
            g_cursor_line = line_idx;
            int col = (mx - 44) / 8;
            if (col < 0) col = 0;
            int maxlen = (int)strlen(g_lines[g_cursor_line]);
            if (col > maxlen) col = maxlen;
            g_cursor_col = col;
        }
    }
}

static void on_key(lgui_widget_t *w, uint32_t key, uint32_t mods, void *ud) {
    (void)w; (void)ud; (void)mods;
    char c = lgui_keymap_translate(key, mods);

    if (g_insert_mode) {
        if (c >= 32 && c < 127) {
            /* Insert character at cursor */
            char *line = g_lines[g_cursor_line];
            int len = (int)strlen(line);
            if (len < MAX_LINE_LEN - 1) {
                memmove(&line[g_cursor_col + 1], &line[g_cursor_col],
                        (size_t)(len - g_cursor_col + 1));
                line[g_cursor_col] = c;
                g_cursor_col++;
                g_modified = true;
            }
        } else if (key == 0x00E7) {
            /* Backspace */
            if (g_cursor_col > 0) {
                char *line = g_lines[g_cursor_line];
                int len = (int)strlen(line);
                memmove(&line[g_cursor_col - 1], &line[g_cursor_col],
                        (size_t)(len - g_cursor_col + 1));
                g_cursor_col--;
                g_modified = true;
            } else if (g_cursor_line > 0) {
                /* Join with previous line */
                int prev_len = (int)strlen(g_lines[g_cursor_line - 1]);
                int cur_len = (int)strlen(g_lines[g_cursor_line]);
                if (prev_len + cur_len < MAX_LINE_LEN - 1) {
                    strcat(g_lines[g_cursor_line - 1], g_lines[g_cursor_line]);
                    for (int i = g_cursor_line; i < g_line_count - 1; i++)
                        strcpy(g_lines[i], g_lines[i+1]);
                    g_line_count--;
                    g_cursor_line--;
                    g_cursor_col = prev_len;
                    g_modified = true;
                }
            }
        } else if (key == 0x009C) {
            /* Delete */
            char *line = g_lines[g_cursor_line];
            int len = (int)strlen(line);
            if (g_cursor_col < len) {
                memmove(&line[g_cursor_col], &line[g_cursor_col + 1],
                        (size_t)(len - g_cursor_col));
                g_modified = true;
            } else if (g_cursor_line < g_line_count - 1) {
                /* Join with next line */
                int curlen = (int)strlen(line);
                int nextlen = (int)strlen(g_lines[g_cursor_line + 1]);
                if (curlen + nextlen < MAX_LINE_LEN - 1) {
                    strcat(line, g_lines[g_cursor_line + 1]);
                    for (int i = g_cursor_line + 1; i < g_line_count - 1; i++)
                        strcpy(g_lines[i], g_lines[i+1]);
                    g_line_count--;
                    g_modified = true;
                }
            }
        } else if (key == 0x0098) {
            /* Left */
            if (g_cursor_col > 0) g_cursor_col--;
            else if (g_cursor_line > 0) {
                g_cursor_line--;
                g_cursor_col = (int)strlen(g_lines[g_cursor_line]);
            }
        } else if (key == 0x0097) {
            /* Right */
            if (g_cursor_col < (int)strlen(g_lines[g_cursor_line])) g_cursor_col++;
            else if (g_cursor_line < g_line_count - 1) {
                g_cursor_line++;
                g_cursor_col = 0;
            }
        } else if (key == 0x0096) {
            /* Up */
            if (g_cursor_line > 0) {
                g_cursor_line--;
                int ml = (int)strlen(g_lines[g_cursor_line]);
                if (g_cursor_col > ml) g_cursor_col = ml;
            }
        } else if (key == 0x0095) {
            /* Down */
            if (g_cursor_line < g_line_count - 1) {
                g_cursor_line++;
                int ml = (int)strlen(g_lines[g_cursor_line]);
                if (g_cursor_col > ml) g_cursor_col = ml;
            }
        } else if (key == 0x0096) {
            /* Home */
            g_cursor_col = 0;
        } else if (key == 0x009B) {
            /* End */
            g_cursor_col = (int)strlen(g_lines[g_cursor_line]);
        } else if (key == 0x009D) {
            /* Enter - insert new line */
            if (g_line_count < MAX_LINES) {
                char *line = g_lines[g_cursor_line];
                int cur_col = g_cursor_col;
                /* Move remaining text to new line */
                for (int i = g_line_count; i > g_cursor_line + 1; i--)
                    strcpy(g_lines[i], g_lines[i-1]);
                strncpy(g_lines[g_cursor_line + 1], line + cur_col, MAX_LINE_LEN - 1);
                line[cur_col] = '\0';
                g_line_count++;
                g_cursor_line++;
                g_cursor_col = 0;
                g_modified = true;
            }
        } else if (c == '\x1b') {
            /* Esc - toggle insert/normal mode */
            g_insert_mode = !g_insert_mode;
        }
    } else {
        /* Normal mode - vi-like commands */
        if (c == 'i') g_insert_mode = true;
        else if (c == 'h' && g_cursor_col > 0) g_cursor_col--;
        else if (c == 'l' && g_cursor_col < (int)strlen(g_lines[g_cursor_line])) g_cursor_col++;
        else if (c == 'j' && g_cursor_line < g_line_count - 1) {
            g_cursor_line++;
            int ml = (int)strlen(g_lines[g_cursor_line]);
            if (g_cursor_col > ml) g_cursor_col = ml;
        }
        else if (c == 'k' && g_cursor_line > 0) {
            g_cursor_line--;
            int ml = (int)strlen(g_lines[g_cursor_line]);
            if (g_cursor_col > ml) g_cursor_col = ml;
        }
        else if (c == 'x') {
            /* Delete char under cursor */
            char *line = g_lines[g_cursor_line];
            int len = (int)strlen(line);
            if (g_cursor_col < len) {
                memmove(&line[g_cursor_col], &line[g_cursor_col + 1],
                        (size_t)(len - g_cursor_col));
                g_modified = true;
            }
        }
        else if (c == 'o') {
            /* Insert line below */
            if (g_line_count < MAX_LINES) {
                for (int i = g_line_count; i > g_cursor_line + 1; i--)
                    strcpy(g_lines[i], g_lines[i-1]);
                strcpy(g_lines[g_cursor_line + 1], "");
                g_line_count++;
                g_cursor_line++;
                g_cursor_col = 0;
                g_insert_mode = true;
                g_modified = true;
            }
        }
        else if (c == ':') {
            /* Command mode - simplified */
            /* TODO: implement command line */
        }
        else if (c == 'q') {
            /* Quit */
            /* Could prompt for save, for now just quit */
        }
        else if (c == 'w') {
            save_file();
        }
    }

    g_status[0] = '\0'; /* Clear status on keypress */
}

int main(int argc, char *argv[]) {
    g_line_count = 1;
    strcpy(g_lines[0], "");

    if (argc > 1) {
        load_file(argv[1]);
    }

    lgui_application_t *app = lgui_application_create("luna-text");
    if (!app) return 1;

    lgui_window_t *win = lgui_window_create(app, WIN_W, WIN_H, LGUI_LAYER_APPLICATION);
    if (!win) { lgui_application_destroy(app); return 1; }

    lgui_widget_t *cw = lgui_canvas_widget_create();
    lgui_widget_set_size(cw, WIN_W, WIN_H);
    lgui_canvas_widget_set_render(cw, editor_render);
    lgui_window_set_root_widget(win, cw);
    lgui_window_show(win);

    lgui_application_set_global_pointer_cb(app, on_pointer, NULL);
    lgui_widget_set_on_key(cw, on_key, NULL);

    /* Focus the canvas so it receives keyboard events */
    lgui_widget_focus(app, cw);

    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
