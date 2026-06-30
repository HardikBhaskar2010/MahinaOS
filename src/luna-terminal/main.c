/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include "luna-gui/include/lunagui.h"

int pty_spawn(pid_t *out_pid, int *out_fd);

static void pty_callback(int fd, void *user_data) {
    (void)user_data;
    char buf[1024];
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    if (n > 0) {
        buf[n] = '\0';
        printf("PTY Output: %s", buf);
        /* In a real terminal, we would parse ANSI and update a text grid */
    } else if (n == 0) {
        printf("PTY Closed\n");
        exit(0);
    }
}

int main(void) {
    lgui_application_t *app = lgui_application_create("luna-terminal");
    if (!app) return 1;
    
    int pty_fd = -1;
    if (pty_spawn(NULL, &pty_fd) == 0 && pty_fd >= 0) {
        lgui_application_add_fd(app, pty_fd, pty_callback, NULL);
    }
    
    lgui_window_t *win = lgui_window_create(app, 640, 480, 100 /* LGP_LAYER_APPLICATION */);
    if (win) {
        lgui_widget_t *vbox = lgui_vbox_create();
        
        lgui_widget_t *lbl = lgui_label_create("Terminal - /bin/sh");
        lgui_box_add_child(vbox, lbl);
        
        lgui_window_set_root_widget(win, vbox);
        lgui_window_show(win);
    }
    
    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
