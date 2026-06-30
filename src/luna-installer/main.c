/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include <stdio.h>
#include <stdlib.h>
#include "luna-gui/include/lunagui.h"

static lgui_window_t *g_win = NULL;
static int g_step = 0;

static void render_step(void);

static void btn_next_clicked(lgui_widget_t *btn, void *user_data) {
    (void)btn; (void)user_data;
    g_step++;
    render_step();
}

static void btn_finish_clicked(lgui_widget_t *btn, void *user_data) {
    (void)btn; (void)user_data;
    printf("Installation finished!\n");
    exit(0);
}

static void render_step(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    
    if (g_step == 0) {
        lgui_box_add_child(vbox, lgui_label_create("Welcome to MahinaOS Setup"));
        lgui_box_add_child(vbox, lgui_label_create("Click Next to begin."));
        lgui_widget_t *btn = lgui_button_create("Next");
        lgui_button_set_on_click(btn, btn_next_clicked, NULL);
        lgui_box_add_child(vbox, btn);
    } else if (g_step == 1) {
        lgui_box_add_child(vbox, lgui_label_create("Partitioning Disk..."));
        lgui_box_add_child(vbox, lgui_label_create("Simulating filesystem creation."));
        lgui_widget_t *btn = lgui_button_create("Next");
        lgui_button_set_on_click(btn, btn_next_clicked, NULL);
        lgui_box_add_child(vbox, btn);
    } else {
        lgui_box_add_child(vbox, lgui_label_create("Installation Complete!"));
        lgui_widget_t *btn = lgui_button_create("Reboot");
        lgui_button_set_on_click(btn, btn_finish_clicked, NULL);
        lgui_box_add_child(vbox, btn);
    }
    
    lgui_window_set_root_widget(g_win, vbox);
}

int main(void) {
    lgui_application_t *app = lgui_application_create("luna-installer");
    if (!app) return 1;
    
    g_win = lgui_window_create(app, 800, 600, 100 /* LGP_LAYER_APPLICATION */);
    if (g_win) {
        render_step();
        lgui_window_show(g_win);
    }
    
    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
