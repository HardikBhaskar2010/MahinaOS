/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include "luna-gui/include/lunagui.h"

static lgui_window_t *g_win = NULL;
static char g_cwd[1024] = "/";

static void render_directory(void);

static void btn_file_clicked(lgui_widget_t *btn, void *user_data) {
    (void)btn;
    char *name = (char *)user_data;
    if (strcmp(name, "..") == 0) {
        char *last_slash = strrchr(g_cwd, '/');
        if (last_slash && last_slash != g_cwd) {
            *last_slash = '\0';
        } else {
            strcpy(g_cwd, "/");
        }
    } else {
        if (strcmp(g_cwd, "/") != 0) {
            strcat(g_cwd, "/");
        }
        strcat(g_cwd, name);
    }
    render_directory();
}

static void render_directory(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    
    char title[1024];
    snprintf(title, sizeof(title), "File Manager - %s", g_cwd);
    lgui_box_add_child(vbox, lgui_label_create(title));
    
    DIR *dir = opendir(g_cwd);
    if (dir) {
        struct dirent *ent;
        int count = 0;
        while ((ent = readdir(dir)) != NULL && count < 14) {
            if (strcmp(ent->d_name, ".") == 0) continue;
            
            char *name_copy = strdup(ent->d_name);
            lgui_widget_t *btn = lgui_button_create(ent->d_name);
            lgui_button_set_on_click(btn, btn_file_clicked, name_copy);
            lgui_box_add_child(vbox, btn);
            count++;
        }
        closedir(dir);
    } else {
        lgui_box_add_child(vbox, lgui_label_create("<Permission Denied or Not a Directory>"));
        lgui_widget_t *btn = lgui_button_create("..");
        lgui_button_set_on_click(btn, btn_file_clicked, strdup(".."));
        lgui_box_add_child(vbox, btn);
    }
    
    lgui_window_set_root_widget(g_win, vbox);
}

int main(void) {
    lgui_application_t *app = lgui_application_create("luna-files");
    if (!app) return 1;
    
    g_win = lgui_window_create(app, 800, 600, 100 /* LGP_LAYER_APPLICATION */);
    if (g_win) {
        render_directory();
        lgui_window_show(g_win);
    }
    
    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
