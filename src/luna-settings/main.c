/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "luna-gui/include/lunagui.h"

static lgui_window_t *g_win = NULL;
static int g_current_page = 0;

static void render_settings(void);

static void btn_page_clicked(lgui_widget_t *btn, void *user_data) {
    (void)btn;
    g_current_page = (int)(intptr_t)user_data;
    render_settings();
}

static void render_settings(void) {
    lgui_widget_t *hbox = lgui_hbox_create();
    
    /* Sidebar */
    lgui_widget_t *sidebar = lgui_vbox_create();
    lgui_widget_t *btn1 = lgui_button_create("Display");
    lgui_widget_t *btn2 = lgui_button_create("Network");
    lgui_widget_t *btn3 = lgui_button_create("About");
    lgui_button_set_on_click(btn1, btn_page_clicked, (void *)(intptr_t)0);
    lgui_button_set_on_click(btn2, btn_page_clicked, (void *)(intptr_t)1);
    lgui_button_set_on_click(btn3, btn_page_clicked, (void *)(intptr_t)2);
    lgui_box_add_child(sidebar, btn1);
    lgui_box_add_child(sidebar, btn2);
    lgui_box_add_child(sidebar, btn3);
    lgui_box_add_child(hbox, sidebar);
    
    /* Content Area */
    lgui_widget_t *content = lgui_vbox_create();
    if (g_current_page == 0) {
        lgui_box_add_child(content, lgui_label_create("Display Settings"));
        lgui_box_add_child(content, lgui_label_create("Resolution: 1920x1080 (Managed by KMS)"));
    } else if (g_current_page == 1) {
        lgui_box_add_child(content, lgui_label_create("Network Settings"));
        lgui_box_add_child(content, lgui_label_create("IP: DHCP (eth0)"));
    } else {
        lgui_box_add_child(content, lgui_label_create("About MahinaOS"));
        lgui_box_add_child(content, lgui_label_create("Version: 0.1 Stage 0"));
        lgui_box_add_child(content, lgui_label_create("Core: LGP Compositor & LunaGUI"));
    }
    lgui_box_add_child(hbox, content);
    
    lgui_window_set_root_widget(g_win, hbox);
}

int main(void) {
    lgui_application_t *app = lgui_application_create("luna-settings");
    if (!app) return 1;
    
    g_win = lgui_window_create(app, 600, 500, 100 /* LGP_LAYER_APPLICATION */);
    if (g_win) {
        render_settings();
        lgui_window_show(g_win);
    }
    
    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
