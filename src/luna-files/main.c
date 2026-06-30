/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * luna-files — MahinaOS graphical file manager.
 *
 * Architecture:
 *   - Browse the filesystem using opendir()/readdir() (POSIX, no GTK/Qt)
 *   - Display directory contents as a list of label/button widgets
 *   - Navigation: click a directory button to enter, "← Up" to go parent
 *   - Current path shown in header bar
 *
 * Limitations (Phase 7):
 *   - Read-only navigation (no delete/rename — deferred to Phase 8)
 *   - No icon rendering (deferred until lgui_image_t is implemented)
 *   - No drag-and-drop (deferred)
 *   - Max 48 entries per directory listed (VBox child limit = 16 per container,
 *     but using a scrollable vbox once lgui_scroll_container is available;
 *     for now cap at 40 entries + navigation)
 */

#include "lunagui.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>

#define MAX_ENTRIES 40

static lgui_application_t *g_app  = NULL;
static lgui_window_t      *g_win  = NULL;
static lgui_widget_t      *g_root = NULL;
static char                g_cwd[4096];

static void navigate_to(const char *path);

/* ── Entry list ───────────────────────────────────────────────────────────── */

typedef struct {
    char name[256];
    bool is_dir;
} dir_entry_t;

static dir_entry_t g_entries[MAX_ENTRIES];
static int         g_entry_count = 0;

static int entry_cmp(const void *a, const void *b) {
    const dir_entry_t *ea = (const dir_entry_t *)a;
    const dir_entry_t *eb = (const dir_entry_t *)b;
    /* Directories first */
    if (ea->is_dir != eb->is_dir) return eb->is_dir - ea->is_dir;
    return strcmp(ea->name, eb->name);
}

static void load_directory(const char *path) {
    g_entry_count = 0;
    DIR *d = opendir(path);
    if (!d) return;

    struct dirent *ent;
    while ((ent = readdir(d)) != NULL && g_entry_count < MAX_ENTRIES) {
        if (ent->d_name[0] == '.' && ent->d_name[1] == '\0') continue; /* skip . */

        dir_entry_t *e = &g_entries[g_entry_count++];
        strncpy(e->name, ent->d_name, sizeof(e->name) - 1);
        e->name[sizeof(e->name) - 1] = '\0';

        /* Determine type */
        if (ent->d_type == DT_DIR) {
            e->is_dir = true;
        } else if (ent->d_type == DT_UNKNOWN) {
            /* Fall back to stat */
            char full[4096 + 256 + 1];
            snprintf(full, sizeof(full), "%s/%s", path, ent->d_name);
            struct stat st;
            e->is_dir = (stat(full, &st) == 0 && S_ISDIR(st.st_mode));
        } else {
            e->is_dir = false;
        }
    }
    closedir(d);
    qsort(g_entries, (size_t)g_entry_count, sizeof(dir_entry_t), entry_cmp);
}

/* ── Click handler (captures index as user_data) ─────────────────────────── */

static void entry_clicked(lgui_widget_t *btn, void *user_data) {
    (void)btn;
    int idx = (int)(intptr_t)user_data;
    if (idx < 0 || idx >= g_entry_count) return;

    if (g_entries[idx].is_dir) {
        char new_path[4096];
        if (strcmp(g_entries[idx].name, "..") == 0) {
            /* Parent directory */
            strncpy(new_path, g_cwd, sizeof(new_path) - 1);
            char *slash = strrchr(new_path, '/');
            if (slash && slash != new_path) {
                *slash = '\0';
            } else {
                strcpy(new_path, "/");
            }
        } else {
            snprintf(new_path, sizeof(new_path), "%s/%s",
                     g_cwd, g_entries[idx].name);
        }
        navigate_to(new_path);
    }
    /* For files: open with default app — deferred to future Phase */
}

static void btn_up_clicked(lgui_widget_t *btn, void *u) {
    (void)btn; (void)u;
    char parent[4096];
    strncpy(parent, g_cwd, sizeof(parent) - 1);
    char *slash = strrchr(parent, '/');
    if (slash && slash != parent) {
        *slash = '\0';
    } else {
        strcpy(parent, "/");
    }
    navigate_to(parent);
}

static void btn_home_clicked(lgui_widget_t *btn, void *u) {
    (void)btn; (void)u;
    const char *home = getenv("HOME");
    navigate_to(home ? home : "/root");
}

/* ── Renderer ─────────────────────────────────────────────────────────────── */

static void navigate_to(const char *path) {
    strncpy(g_cwd, path, sizeof(g_cwd) - 1);
    g_cwd[sizeof(g_cwd) - 1] = '\0';

    load_directory(g_cwd);

    /* Free old tree */
    if (g_root) { lgui_widget_destroy(g_root); g_root = NULL; }

    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    /* Header bar */
    lgui_widget_t *hbar = lgui_hbox_create();
    lgui_widget_set_size(hbar, 800, 40);

    lgui_widget_t *btn_up = lgui_button_create("↑ Up");
    lgui_button_set_on_click(btn_up, btn_up_clicked, NULL);
    lgui_box_add_child(hbar, btn_up);

    lgui_widget_t *btn_home = lgui_button_create("⌂ Home");
    lgui_button_set_on_click(btn_home, btn_home_clicked, NULL);
    lgui_box_add_child(hbar, btn_home);

    lgui_box_add_child(vbox, hbar);

    /* Current path */
    char path_label[4096 + 10];
    snprintf(path_label, sizeof(path_label), "  %s", g_cwd);
    lgui_widget_t *path_lbl = lgui_label_create(path_label);
    lgui_widget_set_size(path_lbl, 800, 20);
    lgui_box_add_child(vbox, path_lbl);

    /* File/directory entries */
    int shown = (g_entry_count < 14) ? g_entry_count : 14; /* visible limit */
    for (int i = 0; i < shown; i++) {
        char label[300];
        snprintf(label, sizeof(label), "%s  %s",
                 g_entries[i].is_dir ? "[DIR]" : "     ",
                 g_entries[i].name);

        lgui_widget_t *btn = lgui_button_create(label);
        lgui_widget_set_size(btn, 780, 28);
        lgui_button_set_on_click(btn, entry_clicked, (void *)(intptr_t)i);
        lgui_box_add_child(vbox, btn);
    }

    if (g_entry_count > shown) {
        char more[64];
        snprintf(more, sizeof(more), "  ... %d more entries",
                 g_entry_count - shown);
        lgui_box_add_child(vbox, lgui_label_create(more));
    }

    g_root = vbox;
    lgui_window_set_root_widget(g_win, g_root);
}

/* ── Entry point ──────────────────────────────────────────────────────────── */

int main(void) {
    g_app = lgui_application_create("luna-files");
    if (!g_app) return 1;

    g_win = lgui_window_create(g_app, 800, 600, LGUI_LAYER_APPLICATION);
    if (!g_win) { lgui_application_destroy(g_app); return 1; }

    const char *start = getenv("HOME");
    if (!start) start = "/";

    navigate_to(start);
    lgui_window_show(g_win);

    lgui_application_run(g_app);
    lgui_application_destroy(g_app);
    return 0;
}
