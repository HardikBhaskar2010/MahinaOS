/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <sys/timerfd.h>
#include <unistd.h>
#include "luna-gui/include/lunagui.h"

static lgui_window_t *g_dock_win = NULL;
static lgui_widget_t *g_lbl_clock = NULL;

static void update_clock(void) {
    time_t now = time(NULL);
    struct tm *t = localtime(&now);
    char buf[64];
    strftime(buf, sizeof(buf), "%H:%M", t);
    lgui_label_set_text(g_lbl_clock, buf);
    if (g_dock_win) {
        lgui_window_update(g_dock_win);
    }
}

static void timer_callback(int fd, void *user_data) {
    (void)user_data;
    uint64_t expirations;
    ssize_t s = read(fd, &expirations, sizeof(expirations));
    (void)s;
    update_clock();
}

int main(int argc, char **argv) {
    (void)argc; (void)argv;
    
    lgui_application_t *app = lgui_application_create("luna-desktop");
    if (!app) return 1;
    
    int tfd = timerfd_create(CLOCK_MONOTONIC, TFD_CLOEXEC);
    if (tfd >= 0) {
        struct itimerspec ts = {
            .it_interval = { 60, 0 }, /* every 60 seconds */
            .it_value = { 1, 0 }      /* first tick in 1 second */
        };
        timerfd_settime(tfd, 0, &ts, NULL);
        lgui_application_add_fd(app, tfd, timer_callback, NULL);
    }
    
    /* 1. Desktop Background (Wallpaper) */
    lgui_window_t *bg_win = lgui_window_create(app, 1920, 1080, 0 /* LGP_LAYER_WALLPAPER */);
    if (bg_win) {
        lgui_widget_t *bg_label = lgui_label_create("MahinaOS");
        lgui_window_set_root_widget(bg_win, bg_label);
        lgui_window_show(bg_win);
    }
    
    /* 2. Dock/Panel */
    g_dock_win = lgui_window_create(app, 1920, 48, 200 /* LGP_LAYER_SHELL */);
    if (g_dock_win) {
        lgui_widget_t *dock_box = lgui_hbox_create();
        
        lgui_box_add_child(dock_box, lgui_button_create("Menu"));
        lgui_box_add_child(dock_box, lgui_button_create("Files"));
        lgui_box_add_child(dock_box, lgui_button_create("Terminal"));
        
        g_lbl_clock = lgui_label_create("00:00");
        lgui_box_add_child(dock_box, g_lbl_clock);
        update_clock(); /* Initial time */
        
        lgui_window_set_root_widget(g_dock_win, dock_box);
        lgui_window_show(g_dock_win);
    }
    
    lgui_application_run(app);
    lgui_application_destroy(app);
    if (tfd >= 0) close(tfd);
    return 0;
}
