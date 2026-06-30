/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * luna-calc — MahinaOS calculator application.
 *
 * Implements a 4-function calculator with expression evaluation.
 * No external math libraries — uses integer arithmetic with double conversion
 * for display, all in pure C.
 *
 * Layout (320 × 400 window):
 *
 *   ┌─────────────────────────┐
 *   │  Display: 0             │
 *   ├────┬────┬────┬──────────┤
 *   │ 7  │ 8  │ 9  │   ÷     │
 *   │ 4  │ 5  │ 6  │   ×     │
 *   │ 1  │ 2  │ 3  │   −     │
 *   │ 0  │ .  │ =  │   +     │
 *   │ C  │ ←  │    │         │
 *   └─────────────────────────┘
 */

#include "lunagui.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

/* ── Calculator state ────────────────────────────────────────────────────── */

static lgui_application_t *g_app = NULL;
static lgui_window_t      *g_win = NULL;
static lgui_widget_t      *g_root = NULL;
static lgui_widget_t      *g_display = NULL;  /* Label showing current input */

static char   g_display_text[128] = "0";
static double g_accumulator = 0.0;
static double g_operand     = 0.0;
static char   g_pending_op  = 0;
static bool   g_new_input   = true;

/* ── Display update ───────────────────────────────────────────────────────── */

static void update_display(void) {
    if (g_display) {
        lgui_label_set_text(g_display, g_display_text);
        /* Mark window dirty for re-render */
        if (g_win) g_win->dirty = true;
    }
}

/* ── Calculator logic ─────────────────────────────────────────────────────── */

static void apply_op(char op, double a, double b, double *result) {
    switch (op) {
        case '+': *result = a + b; break;
        case '-': *result = a - b; break;
        case '*': *result = a * b; break;
        case '/': *result = (b != 0.0) ? a / b : 0.0; break;
        default:  *result = b; break;
    }
}

static void press_digit(char digit) {
    if (g_new_input) {
        g_display_text[0] = digit;
        g_display_text[1] = '\0';
        g_new_input = false;
    } else {
        size_t len = strlen(g_display_text);
        if (len < 20) {
            /* Prevent multiple decimal points */
            if (digit == '.' && strchr(g_display_text, '.')) return;
            g_display_text[len]     = digit;
            g_display_text[len + 1] = '\0';
        }
    }
    update_display();
}

static void press_op(char op) {
    double current = atof(g_display_text);
    if (g_pending_op) {
        apply_op(g_pending_op, g_accumulator, current, &g_accumulator);
        snprintf(g_display_text, sizeof(g_display_text),
                 "%g", g_accumulator);
        update_display();
    } else {
        g_accumulator = current;
    }
    g_pending_op = op;
    g_new_input  = true;
}

static void press_equals(void) {
    double current = atof(g_display_text);
    if (g_pending_op) {
        apply_op(g_pending_op, g_accumulator, current, &g_accumulator);
        snprintf(g_display_text, sizeof(g_display_text),
                 "%g", g_accumulator);
        update_display();
        g_pending_op = 0;
        g_new_input  = true;
    }
}

static void press_clear(void) {
    strcpy(g_display_text, "0");
    g_accumulator = 0.0;
    g_pending_op  = 0;
    g_new_input   = true;
    update_display();
}

static void press_backspace(void) {
    size_t len = strlen(g_display_text);
    if (len > 1) {
        g_display_text[len - 1] = '\0';
    } else {
        strcpy(g_display_text, "0");
        g_new_input = true;
    }
    update_display();
}

/* ── Button callbacks ─────────────────────────────────────────────────────── */

/* user_data is a char* pointing to a static string of length 1 */
static void digit_cb(lgui_widget_t *b, void *u) {
    (void)b;
    press_digit(((char *)u)[0]);
}

static void op_cb(lgui_widget_t *b, void *u) {
    (void)b;
    press_op(((char *)u)[0]);
}

static void eq_cb(lgui_widget_t *b, void *u)   { (void)b;(void)u; press_equals();    }
static void cl_cb(lgui_widget_t *b, void *u)   { (void)b;(void)u; press_clear();     }
static void bs_cb(lgui_widget_t *b, void *u)   { (void)b;(void)u; press_backspace(); }

/* ── Static key data (avoids dangling pointers in callbacks) ─────────────── */

static const char k_0 = '0', k_1 = '1', k_2 = '2', k_3 = '3', k_4 = '4';
static const char k_5 = '5', k_6 = '6', k_7 = '7', k_8 = '8', k_9 = '9';
static const char k_dot = '.';
static const char k_plus = '+', k_minus = '-', k_mul = '*', k_div = '/';

static lgui_widget_t *make_btn(const char *label, lgui_button_click_cb cb, const void *data) {
    lgui_widget_t *b = lgui_button_create(label);
    lgui_widget_set_size(b, 72, 40);
    lgui_button_set_on_click(b, cb, (void *)(uintptr_t)data);
    return b;
}

/* ── Build UI ─────────────────────────────────────────────────────────────── */

static lgui_widget_t *build_calc_ui(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 320, 400);

    /* Display */
    g_display = lgui_label_create(g_display_text);
    lgui_widget_set_size(g_display, 300, 32);
    lgui_box_add_child(vbox, g_display);

    /* Row: 7 8 9 ÷ */
    lgui_widget_t *r1 = lgui_hbox_create();
    lgui_widget_set_size(r1, 320, 44);
    lgui_box_add_child(r1, make_btn("7", digit_cb, &k_7));
    lgui_box_add_child(r1, make_btn("8", digit_cb, &k_8));
    lgui_box_add_child(r1, make_btn("9", digit_cb, &k_9));
    lgui_box_add_child(r1, make_btn("÷", op_cb, &k_div));
    lgui_box_add_child(vbox, r1);

    /* Row: 4 5 6 × */
    lgui_widget_t *r2 = lgui_hbox_create();
    lgui_widget_set_size(r2, 320, 44);
    lgui_box_add_child(r2, make_btn("4", digit_cb, &k_4));
    lgui_box_add_child(r2, make_btn("5", digit_cb, &k_5));
    lgui_box_add_child(r2, make_btn("6", digit_cb, &k_6));
    lgui_box_add_child(r2, make_btn("×", op_cb, &k_mul));
    lgui_box_add_child(vbox, r2);

    /* Row: 1 2 3 − */
    lgui_widget_t *r3 = lgui_hbox_create();
    lgui_widget_set_size(r3, 320, 44);
    lgui_box_add_child(r3, make_btn("1", digit_cb, &k_1));
    lgui_box_add_child(r3, make_btn("2", digit_cb, &k_2));
    lgui_box_add_child(r3, make_btn("3", digit_cb, &k_3));
    lgui_box_add_child(r3, make_btn("−", op_cb, &k_minus));
    lgui_box_add_child(vbox, r3);

    /* Row: 0 . = + */
    lgui_widget_t *r4 = lgui_hbox_create();
    lgui_widget_set_size(r4, 320, 44);
    lgui_box_add_child(r4, make_btn("0", digit_cb, &k_0));
    lgui_box_add_child(r4, make_btn(".", digit_cb, &k_dot));
    lgui_box_add_child(r4, make_btn("=", eq_cb, NULL));
    lgui_box_add_child(r4, make_btn("+", op_cb, &k_plus));
    lgui_box_add_child(vbox, r4);

    /* Row: C ← (1 and 2) */
    lgui_widget_t *r5 = lgui_hbox_create();
    lgui_widget_set_size(r5, 320, 44);
    lgui_box_add_child(r5, make_btn("C", cl_cb, NULL));
    lgui_box_add_child(r5, make_btn("←", bs_cb, NULL));
    lgui_box_add_child(vbox, r5);

    return vbox;
}

/* ── Entry point ──────────────────────────────────────────────────────────── */

int main(void) {
    g_app = lgui_application_create("luna-calc");
    if (!g_app) return 1;

    g_win = lgui_window_create(g_app, 320, 400, LGUI_LAYER_APPLICATION);
    if (!g_win) { lgui_application_destroy(g_app); return 1; }

    g_root = build_calc_ui();
    lgui_window_set_root_widget(g_win, g_root);
    lgui_window_show(g_win);

    lgui_application_run(g_app);
    lgui_application_destroy(g_app);
    return 0;
}
