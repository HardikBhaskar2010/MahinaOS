/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "lunagui.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#define WIN_W 320
#define WIN_H 420

#define COL_BG     0xFF12121Cu
#define COL_DISP   0xFF1E1E28u
#define COL_ACCENT 0xFF00D4FFu
#define COL_TEXT   0xFFEEEEF4u
#define COL_DIM   0xFF9898AAu
#define COL_BTN   0xFF262636u
#define COL_BTNOP 0xFF7B2FBEu
#define COL_BTNEQ 0xFF00FF88u
#define COL_BTNC  0xFFFF2D78u
#define COL_BORDER 0xFF303044u

#define MAX_CLICK 20
static struct { int x,y,w,h; int action; } g_zones[MAX_CLICK];
static int g_zone_count = 0;

static char g_display[64] = "0";
static double g_accum = 0;
static int g_pending_op = 0; /* 0=none, 1=+, 2=-, 3=*, 4=/ */
static bool g_new_input = true;
static bool g_error = false;

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

enum { ACT_0,ACT_1,ACT_2,ACT_3,ACT_4,ACT_5,ACT_6,ACT_7,ACT_8,ACT_9,
       ACT_DOT,ACT_ADD,ACT_SUB,ACT_MUL,ACT_DIV,ACT_EQ,ACT_CLR,ACT_BS,ACT_NEG,ACT_PCT };

static void input_digit(int d) {
    if (g_error) { strcpy(g_display, "0"); g_error = false; g_new_input = true; }
    if (g_new_input) { strcpy(g_display, "0"); g_new_input = false; }
    if (strcmp(g_display, "0") == 0) {
        snprintf(g_display, sizeof(g_display), "%d", d);
    } else {
        char tmp[64];
        snprintf(tmp, sizeof(tmp), "%s%d", g_display, d);
        strncpy(g_display, tmp, sizeof(g_display) - 1);
    }
}

static void input_dot(void) {
    if (g_new_input) { strcpy(g_display, "0."); g_new_input = false; return; }
    if (!strchr(g_display, '.')) {
        strcat(g_display, ".");
    }
}

static void do_op(int op) {
    if (g_error) return;
    double val = atof(g_display);
    if (g_pending_op && !g_new_input) {
        switch (g_pending_op) {
            case 1: g_accum += val; break;
            case 2: g_accum -= val; break;
            case 3: g_accum *= val; break;
            case 4:
                if (val == 0.0) {
                    snprintf(g_display, sizeof(g_display), "Error");
                    g_error = true; g_pending_op = 0; g_new_input = true;
                    return;
                }
                g_accum /= val; break;
        }
        snprintf(g_display, sizeof(g_display), "%g", g_accum);
    } else {
        g_accum = val;
    }
    g_pending_op = op;
    g_new_input = true;
}

static void do_equals(void) {
    if (g_error || !g_pending_op) return;
    double val = atof(g_display);
    switch (g_pending_op) {
        case 1: g_accum += val; break;
        case 2: g_accum -= val; break;
        case 3: g_accum *= val; break;
        case 4:
            if (val == 0.0) {
                snprintf(g_display, sizeof(g_display), "Error");
                g_error = true; g_pending_op = 0; g_new_input = true;
                return;
            }
            g_accum /= val; break;
    }
    snprintf(g_display, sizeof(g_display), "%g", g_accum);
    g_pending_op = 0;
    g_new_input = true;
}

static void do_action(int a) {
    switch (a) {
        case ACT_0: case ACT_1: case ACT_2: case ACT_3: case ACT_4:
        case ACT_5: case ACT_6: case ACT_7: case ACT_8: case ACT_9:
            input_digit(a - ACT_0); break;
        case ACT_DOT: input_dot(); break;
        case ACT_ADD: do_op(1); break;
        case ACT_SUB: do_op(2); break;
        case ACT_MUL: do_op(3); break;
        case ACT_DIV: do_op(4); break;
        case ACT_EQ: do_equals(); break;
        case ACT_CLR:
            strcpy(g_display, "0"); g_accum = 0; g_pending_op = 0;
            g_new_input = true; g_error = false; break;
        case ACT_BS:
            if (!g_new_input && strlen(g_display) > 1) {
                g_display[strlen(g_display)-1] = '\0';
            } else {
                strcpy(g_display, "0"); g_new_input = true;
            }
            break;
        case ACT_NEG: {
            double v = atof(g_display);
            snprintf(g_display, sizeof(g_display), "%g", -v);
            break;
        }
        case ACT_PCT: {
            double v = atof(g_display);
            snprintf(g_display, sizeof(g_display), "%g", v / 100.0);
            break;
        }
    }
}

static void calc_render(lgui_widget_t *w, lgui_canvas_t *c, int x, int y) {
    (void)w; g_zone_count = 0;
    lgui_canvas_push_clip(c, x, y, WIN_W, WIN_H);
    lgui_canvas_fill_rect(c, 0, 0, WIN_W, WIN_H, COL_BG);

    /* Display */
    lgui_canvas_fill_rect(c, 12, 12, WIN_W-24, 48, COL_DISP);
    lgui_canvas_draw_rect_outline(c, 12, 12, WIN_W-24, 48, COL_BORDER);
    /* Right-align display text */
    int tw = (int)strlen(g_display) * 8;
    int dx = WIN_W - 24 - tw - 8;
    if (dx < 16) dx = 16;
    lgui_canvas_draw_text(c, dx, 28, g_display, COL_TEXT);

    /* Pending operation indicator */
    if (g_pending_op && !g_new_input) {
        const char *ops[] = {"", "+", "-", "x", "/"};
        lgui_canvas_fill_rect(c, WIN_W-36, 14, 20, 14, COL_BTNOP);
        lgui_canvas_draw_text(c, WIN_W-32, 16, ops[g_pending_op], COL_TEXT);
    }

    /* Button grid: 5 rows x 4 cols */
    int bw = 66, bh = 52, gap = 6;
    int sx = 16, sy = 68;

    /* Row 1: C, +/-, %, / */
    struct { const char *t; int act; int col; } btns[] = {
        {"C",  ACT_CLR,  COL_BTNC}, {"+/-", ACT_NEG, COL_BTN}, {"%", ACT_PCT, COL_BTN}, {"/", ACT_DIV, COL_BTNOP},
        {"7",  ACT_7,    COL_BTN},  {"8",  ACT_8,   COL_BTN},  {"9", ACT_9, COL_BTN},   {"x", ACT_MUL, COL_BTNOP},
        {"4",  ACT_4,    COL_BTN},  {"5",  ACT_5,   COL_BTN},  {"6", ACT_6, COL_BTN},   {"-", ACT_SUB, COL_BTNOP},
        {"1",  ACT_1,    COL_BTN},  {"2",  ACT_2,   COL_BTN},  {"3", ACT_3, COL_BTN},   {"+", ACT_ADD, COL_BTNOP},
        {"0",  ACT_0,    COL_BTN},  {".",  ACT_DOT, COL_BTN},  {" ", -1,     COL_BTN},   {"=", ACT_EQ, COL_BTNEQ},
    };

    for (int i = 0; i < 20; i++) {
        int row = i / 4, col = i % 4;
        int bx = sx + col * (bw + gap);
        int by = sy + row * (bh + gap);
        uint32_t bg = btns[i].col;

        lgui_canvas_fill_rect(c, bx, by, bw, bh, bg);
        lgui_canvas_draw_rect_outline(c, bx, by, bw, bh, COL_BORDER);

        if (btns[i].act >= 0) {
            lgui_canvas_draw_text(c, bx + (bw - (int)strlen(btns[i].t)*8)/2,
                                   by + (bh - 16)/2, btns[i].t, COL_TEXT);
            add_zone(bx, by, bw, bh, btns[i].act);
        }
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
}

static void on_key(lgui_widget_t *w, uint32_t key, uint32_t mods, void *ud) {
    (void)w; (void)ud; (void)mods;
    char c = lgui_keymap_translate(key, mods);
    if (c >= '0' && c <= '9') input_digit(c - '0');
    else if (c == '.') input_dot();
    else if (c == '+') do_op(1);
    else if (c == '-') do_op(2);
    else if (c == '*') do_op(3);
    else if (c == '/') do_op(4);
    else if (c == '=' || c == '\n') do_equals();
    else if (c == '\x1b') { /* Esc = Clear */
        strcpy(g_display, "0"); g_accum = 0; g_pending_op = 0;
        g_new_input = true; g_error = false;
    }
    else if (c == '\x08') do_action(ACT_BS); /* Backspace */
}

int main(void) {
    lgui_application_t *app = lgui_application_create("luna-calc");
    if (!app) return 1;

    lgui_window_t *win = lgui_window_create(app, WIN_W, WIN_H, LGUI_LAYER_APPLICATION);
    if (!win) { lgui_application_destroy(app); return 1; }

    lgui_widget_t *cw = lgui_canvas_widget_create();
    lgui_widget_set_size(cw, WIN_W, WIN_H);
    lgui_canvas_widget_set_render(cw, calc_render);
    lgui_window_set_root_widget(win, cw);
    lgui_window_show(win);

    lgui_application_set_global_pointer_cb(app, on_pointer, NULL);
    lgui_widget_set_on_key(cw, on_key, NULL);

    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
