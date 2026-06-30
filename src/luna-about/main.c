/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * luna-about — MahinaOS "About This System" application.
 *
 * Reads system information from:
 *   /proc/version     — kernel version
 *   /proc/meminfo     — total RAM
 *   /proc/cpuinfo     — CPU model
 *   uname(2)          — architecture
 *
 * All data read at startup; no polling.
 */

#include "lunagui.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/utsname.h>

/* ── System info collector ────────────────────────────────────────────────── */

static char g_kernel[256]   = "Unknown";
static char g_cpu[256]      = "Unknown";
static char g_ram[64]       = "Unknown";
static char g_arch[64]      = "Unknown";
static char g_hostname[256] = "mahina";

static void read_line(const char *path, char *out, size_t outlen) {
    FILE *f = fopen(path, "r");
    if (!f) return;
    if (fgets(out, (int)outlen, f)) {
        /* Strip trailing newline */
        size_t l = strlen(out);
        while (l > 0 && (out[l-1] == '\n' || out[l-1] == '\r'))
            out[--l] = '\0';
    }
    fclose(f);
}

static void collect_sysinfo(void) {
    /* Kernel */
    read_line("/proc/version", g_kernel, sizeof(g_kernel));
    /* Truncate to first 60 chars for display */
    if (strlen(g_kernel) > 60) {
        g_kernel[60] = '\0';
        strcat(g_kernel, "...");
    }

    /* CPU model */
    FILE *cpuf = fopen("/proc/cpuinfo", "r");
    if (cpuf) {
        char line[256];
        while (fgets(line, sizeof(line), cpuf)) {
            if (strncmp(line, "model name", 10) == 0) {
                char *colon = strchr(line, ':');
                if (colon) {
                    colon++;
                    while (*colon == ' ') colon++;
                    strncpy(g_cpu, colon, sizeof(g_cpu) - 1);
                    /* Strip newline */
                    size_t l = strlen(g_cpu);
                    while (l > 0 && (g_cpu[l-1] == '\n' || g_cpu[l-1] == '\r'))
                        g_cpu[--l] = '\0';
                    break;
                }
            }
        }
        fclose(cpuf);
    }

    /* RAM */
    FILE *memf = fopen("/proc/meminfo", "r");
    if (memf) {
        char line[128];
        while (fgets(line, sizeof(line), memf)) {
            if (strncmp(line, "MemTotal:", 9) == 0) {
                unsigned long kb = 0;
                sscanf(line + 9, " %lu", &kb);
                snprintf(g_ram, sizeof(g_ram), "%lu MB", kb / 1024);
                break;
            }
        }
        fclose(memf);
    }

    /* Arch + hostname via uname */
    struct utsname u;
    if (uname(&u) == 0) {
        strncpy(g_arch,     u.machine,  sizeof(g_arch)     - 1);
        strncpy(g_hostname, u.nodename, sizeof(g_hostname)  - 1);
    }
}

/* ── Entry point ──────────────────────────────────────────────────────────── */

int main(void) {
    collect_sysinfo();

    lgui_application_t *app = lgui_application_create("luna-about");
    if (!app) return 1;

    lgui_window_t *win = lgui_window_create(app, 600, 420, LGUI_LAYER_APPLICATION);
    if (!win) { lgui_application_destroy(app); return 1; }

    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 600, 420);

    lgui_widget_t *title = lgui_label_create("About MahinaOS");
    lgui_widget_set_size(title, 580, 32);
    lgui_box_add_child(vbox, title);
    lgui_box_add_child(vbox, lgui_label_create(""));

    lgui_box_add_child(vbox, lgui_label_create("  MahinaOS 0.1.0  (codename: Horizon)"));
    lgui_box_add_child(vbox, lgui_label_create("  Copyright (c) 2026 Hardik Bhaskar"));
    lgui_box_add_child(vbox, lgui_label_create("  MIT License"));
    lgui_box_add_child(vbox, lgui_label_create(""));

    char buf[320];
    snprintf(buf, sizeof(buf), "  Hostname:     %s", g_hostname);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    snprintf(buf, sizeof(buf), "  Architecture: %s", g_arch);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    snprintf(buf, sizeof(buf), "  CPU:          %s", g_cpu);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    snprintf(buf, sizeof(buf), "  Memory:       %s", g_ram);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    snprintf(buf, sizeof(buf), "  Kernel:       %s", g_kernel);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("  Compositor:   lgp-compositor 1.0 (LGP protocol)"));
    lgui_box_add_child(vbox, lgui_label_create("  GUI Toolkit:  LunaGUI 1.0 (native, no third-party)"));

    lgui_window_set_root_widget(win, vbox);
    lgui_window_show(win);

    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
