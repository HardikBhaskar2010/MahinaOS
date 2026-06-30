/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * luna-installer — MahinaOS graphical installer.
 *
 * Implements the full 10-page installation workflow:
 *   0. Welcome
 *   1. Language
 *   2. Keyboard
 *   3. Timezone
 *   4. Disk selection
 *   5. Partitioning summary
 *   6. User creation
 *   7. Installation summary
 *   8. Installing...
 *   9. Complete
 *
 * Each page is built from LunaGUI widgets. Navigation is driven by
 * Next / Back / Install buttons. The widget tree from the previous page
 * is freed with lgui_widget_destroy() on each transition to prevent
 * memory leaks.
 *
 * Authority: Mission spec — Phase 5 Installer.
 */

#include "lunagui.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* ── Installer state ─────────────────────────────────────────────────────── */

static lgui_application_t *g_app  = NULL;
static lgui_window_t      *g_win  = NULL;
static lgui_widget_t      *g_root = NULL; /* Current root widget (freed on nav) */
static int                 g_step = 0;

/* User configuration collected during installation */
static char g_username[64] = "mahina";
static char g_hostname[64] = "mahina-pc";
static char g_timezone[64] = "UTC";
static char g_language[32] = "en_US.UTF-8";
static char g_keyboard[32] = "us";
static char g_disk[128]    = "/dev/sda";

/* ── Forward declarations ─────────────────────────────────────────────────── */

static void render_page(int page);

/* ── Navigation callbacks ─────────────────────────────────────────────────── */

static void btn_next_clicked(lgui_widget_t *btn, void *user_data) {
    (void)btn; (void)user_data;
    if (g_step < 9) {
        g_step++;
        render_page(g_step);
    }
}

static void btn_back_clicked(lgui_widget_t *btn, void *user_data) {
    (void)btn; (void)user_data;
    if (g_step > 0) {
        g_step--;
        render_page(g_step);
    }
}

static void btn_install_clicked(lgui_widget_t *btn, void *user_data) {
    (void)btn; (void)user_data;
    /* Advance to progress page */
    g_step = 8;
    render_page(g_step);
    /* In a real installer this would fork() an installation process and
     * update progress via a pipe registered with lgui_application_add_fd() */
}

static void btn_reboot_clicked(lgui_widget_t *btn, void *user_data) {
    (void)btn; (void)user_data;
    lgui_application_quit(g_app);
    /* luna-init will handle the actual reboot via its control socket */
}

/* ── Helper: build a navigation button row ───────────────────────────────── */

static lgui_widget_t *make_nav_row(bool show_back, bool show_next,
                                    bool show_install, bool show_reboot) {
    lgui_widget_t *hbox = lgui_hbox_create();
    lgui_widget_set_size(hbox, 800, 48);

    if (show_back) {
        lgui_widget_t *back = lgui_button_create("← Back");
        lgui_button_set_on_click(back, btn_back_clicked, NULL);
        lgui_box_add_child(hbox, back);
    }

    if (show_install) {
        lgui_widget_t *install = lgui_button_create("Install Now →");
        lgui_button_set_on_click(install, btn_install_clicked, NULL);
        lgui_box_add_child(hbox, install);
    }

    if (show_next) {
        lgui_widget_t *next = lgui_button_create("Next →");
        lgui_button_set_on_click(next, btn_next_clicked, NULL);
        lgui_box_add_child(hbox, next);
    }

    if (show_reboot) {
        lgui_widget_t *reboot = lgui_button_create("Reboot System");
        lgui_button_set_on_click(reboot, btn_reboot_clicked, NULL);
        lgui_box_add_child(hbox, reboot);
    }

    return hbox;
}

/* ── Helper: section heading ─────────────────────────────────────────────── */

static lgui_widget_t *make_heading(const char *title, const char *subtitle) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 80);

    lgui_widget_t *lbl_title = lgui_label_create(title);
    lgui_widget_set_size(lbl_title, 760, 28);
    lgui_box_add_child(vbox, lbl_title);

    if (subtitle) {
        lgui_widget_t *lbl_sub = lgui_label_create(subtitle);
        lgui_widget_set_size(lbl_sub, 760, 20);
        lgui_box_add_child(vbox, lbl_sub);
    }

    return vbox;
}

/* ── Page builders ────────────────────────────────────────────────────────── */

/* Page 0: Welcome */
static lgui_widget_t *page_welcome(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Welcome to MahinaOS",
                                           "Let's get your system set up."));
    lgui_box_add_child(vbox, lgui_label_create("This installer will guide you through:"));
    lgui_box_add_child(vbox, lgui_label_create("  • Choosing your language and keyboard layout"));
    lgui_box_add_child(vbox, lgui_label_create("  • Selecting a timezone and disk"));
    lgui_box_add_child(vbox, lgui_label_create("  • Creating your user account"));
    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("The installation takes about 15 minutes."));
    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, make_nav_row(false, true, false, false));
    return vbox;
}

/* Page 1: Language */
static lgui_widget_t *page_language(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Language",
                                           "Choose the system language."));

    /* Language options as buttons (simple selection) */
    const char *langs[] = { "English (US)", "Español", "Français",
                             "Deutsch", "日本語", "中文 (简体)" };
    const char *lang_codes[] = { "en_US.UTF-8", "es_ES.UTF-8", "fr_FR.UTF-8",
                                   "de_DE.UTF-8", "ja_JP.UTF-8", "zh_CN.UTF-8" };
    char selection_label[64];
    snprintf(selection_label, sizeof(selection_label), "Selected: %s", g_language);
    lgui_box_add_child(vbox, lgui_label_create(selection_label));

    for (int i = 0; i < 6; i++) {
        lgui_widget_t *btn = lgui_button_create(langs[i]);
        (void)lang_codes[i]; /* TODO: store selection in g_language on click */
        lgui_box_add_child(vbox, btn);
    }

    lgui_box_add_child(vbox, make_nav_row(true, true, false, false));
    return vbox;
}

/* Page 2: Keyboard */
static lgui_widget_t *page_keyboard(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Keyboard Layout",
                                           "Choose your keyboard layout."));

    char sel_label[64];
    snprintf(sel_label, sizeof(sel_label), "Current layout: %s", g_keyboard);
    lgui_box_add_child(vbox, lgui_label_create(sel_label));

    const char *layouts[] = { "US (QWERTY)", "UK", "German (QWERTZ)",
                               "French (AZERTY)", "Spanish", "Japanese" };
    for (int i = 0; i < 6; i++) {
        lgui_widget_t *btn = lgui_button_create(layouts[i]);
        lgui_box_add_child(vbox, btn);
    }

    lgui_box_add_child(vbox, make_nav_row(true, true, false, false));
    return vbox;
}

/* Page 3: Timezone */
static lgui_widget_t *page_timezone(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Timezone",
                                           "Choose your timezone."));

    char sel_label[64];
    snprintf(sel_label, sizeof(sel_label), "Current timezone: %s", g_timezone);
    lgui_box_add_child(vbox, lgui_label_create(sel_label));

    const char *zones[] = { "UTC", "America/New_York", "America/Los_Angeles",
                             "Europe/London", "Asia/Tokyo", "Australia/Sydney" };
    for (int i = 0; i < 6; i++) {
        lgui_widget_t *btn = lgui_button_create(zones[i]);
        lgui_box_add_child(vbox, btn);
    }

    lgui_box_add_child(vbox, make_nav_row(true, true, false, false));
    return vbox;
}

/* Page 4: Disk selection */
static lgui_widget_t *page_disk(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Disk Selection",
                                           "Choose the target disk for installation."));
    lgui_box_add_child(vbox, lgui_label_create(
        "WARNING: All data on the selected disk will be erased."));
    lgui_box_add_child(vbox, lgui_label_create(""));

    char sel_label[128];
    snprintf(sel_label, sizeof(sel_label), "Selected: %s", g_disk);
    lgui_box_add_child(vbox, lgui_label_create(sel_label));

    /* Real implementation would enumerate /proc/partitions */
    lgui_widget_t *btn_sda = lgui_button_create("/dev/sda — Virtual Disk (64 GB)");
    lgui_box_add_child(vbox, btn_sda);
    lgui_widget_t *btn_vda = lgui_button_create("/dev/vda — VirtIO Disk (64 GB)");
    lgui_box_add_child(vbox, btn_vda);

    lgui_box_add_child(vbox, make_nav_row(true, true, false, false));
    return vbox;
}

/* Page 5: Partitioning summary */
static lgui_widget_t *page_partitioning(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Partitioning",
                                           "MahinaOS will use the following partition layout:"));

    lgui_box_add_child(vbox, lgui_label_create("  /dev/sda1 — EFI System Partition (512 MB, FAT32)"));
    lgui_box_add_child(vbox, lgui_label_create("  /dev/sda2 — Root filesystem (all remaining, Btrfs)"));
    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("Btrfs subvolumes:"));
    lgui_box_add_child(vbox, lgui_label_create("  @root    → /"));
    lgui_box_add_child(vbox, lgui_label_create("  @home    → /home"));
    lgui_box_add_child(vbox, lgui_label_create("  @snapshots → /.snapshots"));
    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("Swap: none (zram used for virtual swap)"));

    lgui_box_add_child(vbox, make_nav_row(true, true, false, false));
    return vbox;
}

/* Page 6: User creation */
static lgui_widget_t *page_user(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Create Your Account",
                                           "Set up the administrator account."));

    lgui_box_add_child(vbox, lgui_label_create("Full Name:"));
    lgui_box_add_child(vbox, lgui_label_create("[Text input not yet implemented]"));
    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("Username:"));

    char usr_label[128];
    snprintf(usr_label, sizeof(usr_label), "  %s (default)", g_username);
    lgui_box_add_child(vbox, lgui_label_create(usr_label));

    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("Computer Name:"));

    char host_label[128];
    snprintf(host_label, sizeof(host_label), "  %s (default)", g_hostname);
    lgui_box_add_child(vbox, lgui_label_create(host_label));

    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("Password: ••••••••  (default: mahina)"));

    lgui_box_add_child(vbox, make_nav_row(true, true, false, false));
    return vbox;
}

/* Page 7: Installation summary */
static lgui_widget_t *page_summary(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Ready to Install",
                                           "Please review your choices before we begin."));

    char buf[128];
    snprintf(buf, sizeof(buf), "  Language:   %s", g_language);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    snprintf(buf, sizeof(buf), "  Keyboard:   %s", g_keyboard);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    snprintf(buf, sizeof(buf), "  Timezone:   %s", g_timezone);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    snprintf(buf, sizeof(buf), "  Target disk: %s", g_disk);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    snprintf(buf, sizeof(buf), "  Username:   %s", g_username);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    snprintf(buf, sizeof(buf), "  Hostname:   %s", g_hostname);
    lgui_box_add_child(vbox, lgui_label_create(buf));

    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create(
        "Proceeding will ERASE the selected disk and install MahinaOS."));

    lgui_box_add_child(vbox, make_nav_row(true, false, true, false));
    return vbox;
}

/* Page 8: Installing */
static lgui_widget_t *page_installing(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Installing MahinaOS",
                                           "Please wait while the system is installed."));
    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("[████████████████████████░░░░░░░░] 75%"));
    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("Current step: Generating initramfs..."));
    lgui_box_add_child(vbox, lgui_label_create("Do not power off your computer."));

    /* Real implementation would update this from a pipe read from the install script */
    /* Simulated: advance to complete page */
    lgui_widget_t *skip = lgui_button_create("[Simulate: Installation Complete]");
    lgui_button_set_on_click(skip, btn_next_clicked, NULL);
    lgui_box_add_child(vbox, skip);

    return vbox;
}

/* Page 9: Complete */
static lgui_widget_t *page_complete(void) {
    lgui_widget_t *vbox = lgui_vbox_create();
    lgui_widget_set_size(vbox, 800, 600);

    lgui_box_add_child(vbox, make_heading("Installation Complete",
                                           "MahinaOS has been installed successfully."));
    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("Remove the installation media and reboot"));
    lgui_box_add_child(vbox, lgui_label_create("to start using your new system."));
    lgui_box_add_child(vbox, lgui_label_create(""));
    lgui_box_add_child(vbox, lgui_label_create("Welcome to Mahina."));
    lgui_box_add_child(vbox, lgui_label_create(""));

    lgui_box_add_child(vbox, make_nav_row(false, false, false, true));
    return vbox;
}

/* ── Page router ──────────────────────────────────────────────────────────── */

typedef lgui_widget_t *(*page_fn_t)(void);

static const page_fn_t page_builders[] = {
    page_welcome,
    page_language,
    page_keyboard,
    page_timezone,
    page_disk,
    page_partitioning,
    page_user,
    page_summary,
    page_installing,
    page_complete,
};

static void render_page(int page) {
    /* Free the previous widget tree to prevent memory leaks */
    if (g_root) {
        lgui_widget_destroy(g_root);
        g_root = NULL;
    }

    if (page < 0 || page >= (int)(sizeof(page_builders) / sizeof(page_builders[0]))) {
        page = 0;
    }

    g_root = page_builders[page]();
    lgui_window_set_root_widget(g_win, g_root);
}

/* ── Entry point ──────────────────────────────────────────────────────────── */

int main(void) {
    g_app = lgui_application_create("luna-installer");
    if (!g_app) {
        fprintf(stderr, "luna-installer: cannot connect to lgp-compositor\n");
        return 1;
    }

    g_win = lgui_window_create(g_app, 800, 600, LGUI_LAYER_APPLICATION);
    if (!g_win) {
        fprintf(stderr, "luna-installer: cannot create window\n");
        lgui_application_destroy(g_app);
        return 1;
    }

    render_page(0);
    lgui_window_show(g_win);

    lgui_application_run(g_app);
    lgui_application_destroy(g_app);
    return 0;
}
