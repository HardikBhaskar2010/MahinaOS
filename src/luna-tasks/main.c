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
#include <dirent.h>
#include <signal.h>
#include <sys/time.h>
#include <sys/timerfd.h>

#define WIN_W 600
#define WIN_H 480
#define MAX_PROCS 512
#define VISIBLE_PROCS 20

#define COL_BG     0xFF12121Cu
#define COL_HEADER 0xFF1E1E28u
#define COL_ACCENT 0xFF00D4FFu
#define COL_TEXT   0xFFEEEEF4u
#define COL_DIM   0xFF9898AAu
#define COL_GREEN  0xFF00FF88u
#define COL_PINK   0xFFFF2D78u
#define COL_BTN   0xFF262636u
#define COL_BORDER 0xFF303044u
#define COL_ERR   0xFFFF2D78u

#define MAX_CLICK 24
static struct { int x,y,w,h; int action; } g_zones[MAX_CLICK];
static int g_zone_count = 0;

struct proc_info {
    int pid;
    char name[64];
    char state;
    unsigned long rss_kb;
    int cpu_percent;
};

static struct proc_info g_procs[MAX_PROCS];
static int g_proc_count = 0;
static int g_scroll = 0;
static char g_status[256] = "";
static unsigned long g_prev_total = 0;
static unsigned long g_prev_idle = 0;

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

static unsigned long get_total_cpu(void) {
    FILE *f = fopen("/proc/stat", "r");
    if (!f) return 0;
    char line[256];
    unsigned long total = 0;
    if (fgets(line, sizeof(line), f)) {
        unsigned long user, nice, sys, idle, iowait, irq, softirq;
        if (sscanf(line, "cpu %lu %lu %lu %lu %lu %lu %lu",
                   &user, &nice, &sys, &idle, &iowait, &irq, &softirq) >= 5) {
            total = user + nice + sys + idle + iowait + irq + softirq;
        }
    }
    fclose(f);
    return total;
}

static unsigned long get_idle_cpu(void) {
    FILE *f = fopen("/proc/stat", "r");
    if (!f) return 0;
    char line[256];
    unsigned long idle = 0;
    if (fgets(line, sizeof(line), f)) {
        unsigned long user, nice, sys;
        if (sscanf(line, "cpu %lu %lu %lu %lu", &user, &nice, &sys, &idle) < 4) {
            idle = 0;
        }
    }
    fclose(f);
    return idle;
}

static void calc_cpu_percent(void) {
    unsigned long total = get_total_cpu();
    unsigned long idle = get_idle_cpu();
    unsigned long total_diff = total - g_prev_total;

    g_prev_total = total;
    g_prev_idle = idle;

    if (total_diff == 0) return;

    for (int i = 0; i < g_proc_count; i++) {
        /* Approximate: use process utime+stime from /proc/<pid>/stat */
        char path[64];
        snprintf(path, sizeof(path), "/proc/%d/stat", g_procs[i].pid);
        FILE *f = fopen(path, "r");
        if (!f) { g_procs[i].cpu_percent = 0; continue; }

        char line[1024];
        if (fgets(line, sizeof(line), f)) {
            /* Skip past comm field (which can contain spaces and parens) */
            char *p = strrchr(line, ')');
            if (p) {
                p += 2; /* skip ") " */
                unsigned long utime, stime;
                if (sscanf(p, "%*c %*d %*d %*d %*d %*d %*u %*u %*u %*u %*u %lu %lu",
                           &utime, &stime) == 2) {
                    unsigned long proc_total = utime + stime;
                    g_procs[i].cpu_percent = (int)(proc_total * 100 / (total_diff > 0 ? total_diff : 1));
                    if (g_procs[i].cpu_percent > 999) g_procs[i].cpu_percent = 999;
                }
            }
        }
        fclose(f);
    }
}

static void read_procs(void) {
    DIR *d = opendir("/proc");
    if (!d) return;
    g_proc_count = 0;
    struct dirent *de;
    while ((de = readdir(d)) && g_proc_count < MAX_PROCS) {
        int pid = atoi(de->d_name);
        if (pid <= 0) continue;

        char path[64];
        snprintf(path, sizeof(path), "/proc/%d/status", pid);
        FILE *f = fopen(path, "r");
        if (!f) continue;

        g_procs[g_proc_count].pid = pid;
        g_procs[g_proc_count].name[0] = '\0';
        g_procs[g_proc_count].state = '?';
        g_procs[g_proc_count].rss_kb = 0;
        g_procs[g_proc_count].cpu_percent = 0;

        char line[256];
        while (fgets(line, sizeof(line), f)) {
            if (strncmp(line, "Name:", 5) == 0) {
                char *p = line + 5;
                while (*p == ' ' || *p == '\t') p++;
                size_t l = strlen(p);
                while (l > 0 && (p[l-1]=='\n'||p[l-1]=='\r')) p[--l]='\0';
                strncpy(g_procs[g_proc_count].name, p, 63);
            } else if (strncmp(line, "State:", 6) == 0) {
                g_procs[g_proc_count].state = line[7];
            } else if (strncmp(line, "VmRSS:", 6) == 0) {
                sscanf(line + 6, "%lu", &g_procs[g_proc_count].rss_kb);
            }
        }
        fclose(f);
        g_proc_count++;
    }
    closedir(d);

    /* Sort by PID */
    for (int i = 0; i < g_proc_count - 1; i++) {
        for (int j = i + 1; j < g_proc_count; j++) {
            if (g_procs[i].pid > g_procs[j].pid) {
                struct proc_info tmp = g_procs[i];
                g_procs[i] = g_procs[j];
                g_procs[j] = tmp;
            }
        }
    }

    calc_cpu_percent();
    snprintf(g_status, sizeof(g_status), "%d processes", g_proc_count);
}

enum { ACT_REFRESH=100, ACT_KILL_BASE=200, ACT_SCROLL_UP, ACT_SCROLL_DOWN };

static void do_action(int a) {
    if (a == ACT_REFRESH) {
        read_procs();
    } else if (a == ACT_SCROLL_UP) {
        if (g_scroll > 0) g_scroll--;
    } else if (a == ACT_SCROLL_DOWN) {
        if (g_scroll + VISIBLE_PROCS < g_proc_count) g_scroll++;
    } else if (a >= ACT_KILL_BASE && a < ACT_KILL_BASE + VISIBLE_PROCS) {
        int idx = a - ACT_KILL_BASE + g_scroll;
        if (idx < g_proc_count && g_procs[idx].pid > 1) {
            if (kill(g_procs[idx].pid, SIGTERM) == 0) {
                snprintf(g_status, sizeof(g_status), "Killed PID %d", g_procs[idx].pid);
                read_procs();
            } else {
                snprintf(g_status, sizeof(g_status), "Failed to kill PID %d", g_procs[idx].pid);
            }
        }
    }
}

static void tasks_render(lgui_widget_t *w, lgui_canvas_t *c, int x, int y) {
    (void)w; g_zone_count = 0;
    lgui_canvas_push_clip(c, x, y, WIN_W, WIN_H);
    lgui_canvas_fill_rect(c, 0, 0, WIN_W, WIN_H, COL_BG);

    /* Header */
    lgui_canvas_fill_rect(c, 0, 0, WIN_W, 22, COL_HEADER);
    lgui_canvas_draw_text(c, 8, 3, "Task Manager", COL_ACCENT);

    /* Buttons */
    lgui_canvas_fill_rect(c, WIN_W-160, 2, 60, 18, COL_BTN);
    lgui_canvas_draw_rect_outline(c, WIN_W-160, 2, 60, 18, COL_BORDER);
    lgui_canvas_draw_text(c, WIN_W-156, 4, "Refresh", COL_TEXT);
    add_zone(WIN_W-160, 2, 60, 18, ACT_REFRESH);

    /* Column headers */
    int y0 = 28;
    lgui_canvas_fill_rect(c, 0, y0, WIN_W, 18, COL_BG);
    lgui_canvas_draw_text(c, 8, y0+1, "PID", COL_DIM);
    lgui_canvas_draw_text(c, 72, y0+1, "Name", COL_DIM);
    lgui_canvas_draw_text(c, 300, y0+1, "State", COL_DIM);
    lgui_canvas_draw_text(c, 380, y0+1, "RSS", COL_DIM);
    lgui_canvas_draw_text(c, 480, y0+1, "CPU%", COL_DIM);
    lgui_canvas_draw_text(c, 540, y0+1, "Kill", COL_DIM);
    y0 += 20;

    /* Process list */
    int end = g_scroll + VISIBLE_PROCS;
    if (end > g_proc_count) end = g_proc_count;

    for (int i = g_scroll; i < end; i++) {
        int ry = y0 + (i - g_scroll) * 18;
        uint32_t text_col = COL_TEXT;

        /* Row background */
        lgui_canvas_fill_rect(c, 0, ry, WIN_W, 18, (i % 2 == 0) ? COL_BG : COL_HEADER);

        /* PID */
        char buf[16];
        snprintf(buf, sizeof(buf), "%d", g_procs[i].pid);
        lgui_canvas_draw_text(c, 8, ry+1, buf, COL_DIM);

        /* Name (truncated) */
        char name[36];
        strncpy(name, g_procs[i].name, 35);
        name[35] = '\0';
        lgui_canvas_draw_text(c, 72, ry+1, name, text_col);

        /* State */
        const char *state_str = "?";
        switch(g_procs[i].state) {
            case 'R': state_str = "Running"; break;
            case 'S': state_str = "Sleep"; break;
            case 'D': state_str = "Disk"; break;
            case 'Z': state_str = "Zombie"; break;
            case 'T': state_str = "Stop"; break;
        }
        lgui_canvas_draw_text(c, 300, ry+1, state_str, COL_DIM);

        /* RSS */
        if (g_procs[i].rss_kb > 1024)
            snprintf(buf, sizeof(buf), "%lu MB", g_procs[i].rss_kb / 1024);
        else
            snprintf(buf, sizeof(buf), "%lu KB", g_procs[i].rss_kb);
        lgui_canvas_draw_text(c, 380, ry+1, buf, COL_DIM);

        /* CPU% */
        snprintf(buf, sizeof(buf), "%d%%", g_procs[i].cpu_percent);
        uint32_t cpu_col = g_procs[i].cpu_percent > 50 ? COL_PINK :
                           g_procs[i].cpu_percent > 10 ? COL_ACCENT : COL_DIM;
        lgui_canvas_draw_text(c, 480, ry+1, buf, cpu_col);

        /* Kill button */
        if (g_procs[i].pid > 1) {
            lgui_canvas_fill_rect(c, 540, ry, 40, 18, COL_ERR);
            lgui_canvas_draw_text(c, 544, ry+1, "Kill", COL_TEXT);
            add_zone(540, ry, 40, 18, ACT_KILL_BASE + (i - g_scroll));
        }
    }

    /* Scroll indicators */
    if (g_scroll > 0) {
        lgui_canvas_fill_rect(c, WIN_W-20, 48, 18, 18, COL_BTN);
        lgui_canvas_draw_text(c, WIN_W-16, 50, "^", COL_TEXT);
        add_zone(WIN_W-20, 48, 18, 18, ACT_SCROLL_UP);
    }
    if (g_scroll + VISIBLE_PROCS < g_proc_count) {
        lgui_canvas_fill_rect(c, WIN_W-20, WIN_H-40, 18, 18, COL_BTN);
        lgui_canvas_draw_text(c, WIN_W-16, WIN_H-38, "v", COL_TEXT);
        add_zone(WIN_W-20, WIN_H-40, 18, 18, ACT_SCROLL_DOWN);
    }

    /* Status bar */
    lgui_canvas_fill_rect(c, 0, WIN_H-20, WIN_W, 20, COL_BG);
    lgui_canvas_fill_rect(c, 0, WIN_H-20, WIN_W, 1, COL_BORDER);
    lgui_canvas_draw_text(c, 8, WIN_H-16, g_status, COL_DIM);

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
    if (c == 'r' || c == 'R') do_action(ACT_REFRESH);
    else if (c == 'q' || c == 'Q') {
        lgui_application_t *app = (lgui_application_t *)ud;
        lgui_application_quit(app);
    }
}

static void on_timer(int fd, void *ud) {
    (void)fd; (void)ud;
    read_procs();
    /* Window will be marked dirty on next interaction */
}

int main(void) {
    read_procs();

    lgui_application_t *app = lgui_application_create("luna-tasks");
    if (!app) return 1;

    lgui_window_t *win = lgui_window_create(app, WIN_W, WIN_H, LGUI_LAYER_APPLICATION);
    if (!win) { lgui_application_destroy(app); return 1; }

    lgui_widget_t *cw = lgui_canvas_widget_create();
    lgui_widget_set_size(cw, WIN_W, WIN_H);
    lgui_canvas_widget_set_render(cw, tasks_render);
    lgui_window_set_root_widget(win, cw);
    lgui_window_show(win);

    lgui_application_set_global_pointer_cb(app, on_pointer, NULL);
    lgui_widget_set_on_key(cw, on_key, app);

    /* Auto-refresh every 2 seconds using timerfd */
    int tfd = timerfd_create(CLOCK_MONOTONIC, TFD_CLOEXEC);
    if (tfd >= 0) {
        struct itimerspec its = { {2, 0}, {2, 0} };
        timerfd_settime(tfd, 0, &its, NULL);
        lgui_application_add_fd(app, tfd, on_timer, NULL);
    }

    lgui_widget_focus(app, cw);
    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
