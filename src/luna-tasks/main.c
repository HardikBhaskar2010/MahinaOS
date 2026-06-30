/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * luna-tasks — MahinaOS Task Manager.
 *
 * Lists running processes by reading /proc/<pid>/status for each entry
 * in /proc. Displays: PID, process name, memory usage (VmRSS), state.
 *
 * Refresh: clicking "Refresh" rebuilds the list from /proc.
 *
 * Sorting: currently by PID ascending. Future: clickable column headers.
 *
 * Kill: clicking "× Kill" next to a process sends SIGTERM to that PID.
 * Only processes owned by the current user can be killed safely;
 * root-owned processes will produce EPERM (silently ignored).
 */

#include "lunagui.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <signal.h>
#include <unistd.h>

#define MAX_PROCS   64
#define VISIBLE_MAX 12

typedef struct {
    int    pid;
    char   name[64];
    char   state[8];
    long   rss_kb;
} proc_info_t;

static lgui_application_t *g_app  = NULL;
static lgui_window_t      *g_win  = NULL;
static lgui_widget_t      *g_root = NULL;

static proc_info_t g_procs[MAX_PROCS];
static int         g_proc_count = 0;

/* ── /proc reader ─────────────────────────────────────────────────────────── */

static void read_proc(void) {
    g_proc_count = 0;
    DIR *d = opendir("/proc");
    if (!d) return;

    struct dirent *ent;
    while ((ent = readdir(d)) != NULL && g_proc_count < MAX_PROCS) {
        /* Only numeric entries are PIDs */
        int pid = atoi(ent->d_name);
        if (pid <= 0) continue;

        char status_path[64];
        snprintf(status_path, sizeof(status_path), "/proc/%d/status", pid);

        FILE *f = fopen(status_path, "r");
        if (!f) continue;

        proc_info_t *p = &g_procs[g_proc_count];
        p->pid    = pid;
        p->rss_kb = 0;
        strcpy(p->name, "?");
        strcpy(p->state, "?");

        char line[256];
        while (fgets(line, sizeof(line), f)) {
            if (strncmp(line, "Name:", 5) == 0) {
                sscanf(line + 5, " %63s", p->name);
            } else if (strncmp(line, "State:", 6) == 0) {
                sscanf(line + 6, " %7s", p->state);
            } else if (strncmp(line, "VmRSS:", 6) == 0) {
                sscanf(line + 6, " %ld", &p->rss_kb);
            }
        }
        fclose(f);
        g_proc_count++;
    }
    closedir(d);
}

/* ── Kill callback ────────────────────────────────────────────────────────── */

static void render_view(void);

static void kill_proc(lgui_widget_t *b, void *user_data) {
    (void)b;
    int pid = (int)(intptr_t)user_data;
    if (pid > 1) {
        kill((pid_t)pid, SIGTERM);
    }
    /* Refresh list */
    read_proc();
    render_view();
}

static void refresh_cb(lgui_widget_t *b, void *u) {
    (void)b; (void)u;
    read_proc();
    render_view();
}

/* ── Renderer ─────────────────────────────────────────────────────────────── */

static void render_view(void) {
    if (g_root) { lgui_widget_destroy(g_root); g_root = NULL; }

    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    /* Header */
    lgui_widget_t *hbar = lgui_hbox_create();
    lgui_widget_set_size(hbar, 800, 40);
    lgui_widget_t *title = lgui_label_create("Task Manager");
    lgui_widget_set_size(title, 540, 32);
    lgui_box_add_child(hbar, title);
    lgui_widget_t *ref = lgui_button_create("↻ Refresh");
    lgui_button_set_on_click(ref, refresh_cb, NULL);
    lgui_box_add_child(hbar, ref);
    lgui_box_add_child(vbox, hbar);

    /* Column headers */
    char header[256];
    snprintf(header, sizeof(header), "  %-6s  %-20s  %-8s  %8s",
             "PID", "Name", "State", "Mem (KB)");
    lgui_box_add_child(vbox, lgui_label_create(header));

    /* Process rows */
    int shown = (g_proc_count < VISIBLE_MAX) ? g_proc_count : VISIBLE_MAX;
    for (int i = 0; i < shown; i++) {
        proc_info_t *p = &g_procs[i];
        lgui_widget_t *row = lgui_hbox_create();
        lgui_widget_set_size(row, 800, 30);

        char info[256];
        snprintf(info, sizeof(info), "  %-6d  %-20s  %-8s  %8ld",
                 p->pid, p->name, p->state, p->rss_kb);
        lgui_widget_t *lbl = lgui_label_create(info);
        lgui_widget_set_size(lbl, 640, 24);
        lgui_box_add_child(row, lbl);

        lgui_widget_t *kill_btn = lgui_button_create("× Kill");
        lgui_widget_set_size(kill_btn, 80, 24);
        lgui_button_set_on_click(kill_btn, kill_proc, (void *)(intptr_t)p->pid);
        lgui_box_add_child(row, kill_btn);

        lgui_box_add_child(vbox, row);
    }

    if (g_proc_count > VISIBLE_MAX) {
        char more[64];
        snprintf(more, sizeof(more), "  ... %d more processes",
                 g_proc_count - VISIBLE_MAX);
        lgui_box_add_child(vbox, lgui_label_create(more));
    }

    char count_str[64];
    snprintf(count_str, sizeof(count_str), "  Total visible: %d processes", g_proc_count);
    lgui_box_add_child(vbox, lgui_label_create(count_str));

    g_root = vbox;
    lgui_window_set_root_widget(g_win, g_root);
}

/* ── Entry point ──────────────────────────────────────────────────────────── */

int main(void) {
    g_app = lgui_application_create("luna-tasks");
    if (!g_app) return 1;

    g_win = lgui_window_create(g_app, 800, 600, LGUI_LAYER_APPLICATION);
    if (!g_win) { lgui_application_destroy(g_app); return 1; }

    read_proc();
    render_view();
    lgui_window_show(g_win);

    lgui_application_run(g_app);
    lgui_application_destroy(g_app);
    return 0;
}
