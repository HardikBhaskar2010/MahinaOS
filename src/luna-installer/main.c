/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * luna-installer/main.c  —  MahinaOS Premium Graphical Installer
 *
 * NOT like other Linux installers.
 *
 * Architecture:
 *   - ONE full-screen lgui_canvas_widget_t covers the entire display.
 *   - ALL interaction goes through lgui_application_set_global_pointer_cb
 *     (hover + click) and lgui_application_set_global_key_cb (text input).
 *   - A timerfd drives cursor-blink and progress animations at 500 ms.
 *   - The install backend forks install_worker_run() and reads progress
 *     from a pipe registered with lgui_application_add_fd().
 *
 * Visual layout:
 *
 *   ┌─────────────────────────────────────────────────────────────────────┐
 *   │ SIDEBAR (240 px)  │                CONTENT AREA                    │
 *   │                   │                                                 │
 *   │  [M] MAHINA       │  ┌─ Page heading ──────────────────────────┐  │
 *   │  Installation     │  │  Subtitle                               │  │
 *   │  ─────────────    │  └──────────────────────────────────────────┘  │
 *   │  ● 1 Welcome  ←  │                                                 │
 *   │  │ 2 Language     │  ┌──────────────┐  ┌──────────────────────┐   │
 *   │  │ 3 Keyboard     │  │  Card A      │  │  Card B              │   │
 *   │  │ 4 Timezone     │  └──────────────┘  └──────────────────────┘   │
 *   │  │ 5 Disk         │                                                 │
 *   │  │ 6 Partitions   │  ┌──────────────┐  ┌──────────────────────┐   │
 *   │  │ 7 Account      │  │  Card C      │  │  Card D              │   │
 *   │  │ 8 Confirm      │  └──────────────┘  └──────────────────────┘   │
 *   │  │ 9 Installing   │                                                 │
 *   │  │10 Done         │        [<- Back]              [Continue ->]    │
 *   └─────────────────────────────────────────────────────────────────────┘
 */

#define _GNU_SOURCE
#include "lunagui.h"
#include "install_worker.h"

#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/timerfd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

/* ── Design constants ────────────────────────────────────────────────────── */

#define SIDEBAR_W     240           /* Left sidebar width                     */
#define HEADER_H       72           /* Per-page header height                 */
#define FOOTER_H       64           /* Bottom navigation bar height           */
#define GLYPH_W         8           /* PSF font glyph width (pixels)          */
#define GLYPH_H        16           /* PSF font glyph height (pixels)         */
#define CARD_H         68           /* Selection card height                  */
#define CARD_GAP       10           /* Gap between selection cards            */
#define CARD_MARGIN    20           /* Left/right margin inside content area  */
#define INPUT_H        36           /* Text input box height                  */
#define DISK_CARD_H    90           /* Disk selection card height             */

/* ── Mahina Design System — color palette (0xFFRRGGBB) ───────────────────── */

#define C_BG           0xFF0D0D1A   /* Deepest background                     */
#define C_SIDEBAR_BG   0xFF09090F   /* Sidebar background                     */
#define C_SIDEBAR2     0xFF101020   /* Sidebar header area                    */
#define C_PANEL        0xFF141426   /* Panel/content background               */
#define C_CARD         0xFF18182A   /* Card background (unselected)           */
#define C_CARD_HOVER   0xFF20203C   /* Card background (hovered)              */
#define C_CARD_SEL     0xFF1C0E36   /* Card background (selected)             */
#define C_ACCENT       0xFFE03E8A   /* Hot pink accent                        */
#define C_ACCENT_DIM   0xFF7A2248   /* Dimmed pink (subtle)                   */
#define C_PURPLE       0xFF7C5CBF   /* Violet/purple accent                   */
#define C_TEAL         0xFF3ABFAA   /* Teal accent (success/done)             */
#define C_TEXT         0xFFEEEEF4   /* Primary text                           */
#define C_TEXT_DIM     0xFF9898B8   /* Secondary text                         */
#define C_TEXT_FAINT   0xFF454560   /* Tertiary/placeholder text              */
#define C_DIVIDER      0xFF20203A   /* Thin divider lines                     */
#define C_INPUT_BG     0xFF0A0A14   /* Text input background                  */
#define C_DANGER       0xFFFF3D5A   /* Destructive action (erase disk!)       */
#define C_SUCCESS      0xFF3AB795   /* Success state                          */
#define C_STEP_DONE    0xFF5A4B8A   /* Completed step circle                  */

/* ── Step definitions ────────────────────────────────────────────────────── */

#define STEP_COUNT  10
static const char *const STEP_LABEL[STEP_COUNT] = {
    "Welcome", "Language", "Keyboard", "Timezone",
    "Disk",    "Partitions","Account",  "Confirm",
    "Installing","Done"
};

/* ── Hit-test registry ───────────────────────────────────────────────────── */

#define MAX_HITS 24
typedef struct { int x, y, w, h; char action[80]; } hit_t;
static hit_t g_hits[MAX_HITS];
static int   g_nhits = 0;

static void   hits_clear(void) { g_nhits = 0; }
static void   hits_add(int x, int y, int w, int h, const char *a) {
    if (g_nhits >= MAX_HITS) return;
    g_hits[g_nhits] = (hit_t){ x, y, w, h, "" };
    snprintf(g_hits[g_nhits].action, sizeof(g_hits[0].action), "%s", a);
    g_nhits++;
}
static const char *hits_test(int px, int py) {
    for (int i = g_nhits - 1; i >= 0; i--) /* last registered = top */
        if (px >= g_hits[i].x && px < g_hits[i].x + g_hits[i].w &&
            py >= g_hits[i].y && py < g_hits[i].y + g_hits[i].h)
            return g_hits[i].action;
    return NULL;
}

/* ── Custom text-input state (for Account page) ───────────────────────────── */

typedef struct {
    char buf[96];
    int  len;
    int  max_len;
    bool password;
    /* Rendering position (canvas-local, set during draw) */
    int  x, y, w, h;
} tinput_t;

#define N_INPUTS 3
static tinput_t g_inp[N_INPUTS];   /* 0=username  1=hostname  2=password   */
static int      g_focus = 0;       /* Which input has focus                 */

/* ── Install backend state ───────────────────────────────────────────────── */

static int    g_pipe_rd      = -1;
static pid_t  g_worker_pid   = -1;
static int    g_pct          = 0;
static char   g_pmsg[256]    = "Preparing...";
static bool   g_install_done = false;
static bool   g_install_err  = false;

/* ── Disk list ───────────────────────────────────────────────────────────── */

#define MAX_DISKS 12
static char   g_disks[MAX_DISKS][64];
static size_t g_disk_gb[MAX_DISKS];
static int    g_ndisks = 0;

/* ── Application globals ─────────────────────────────────────────────────── */

static lgui_application_t *g_app    = NULL;
static lgui_window_t      *g_win    = NULL;
static lgui_widget_t      *g_canvas = NULL;
static uint32_t            g_sw     = 1024;
static uint32_t            g_sh     = 768;

static int  g_step = 0;
static int  g_hx   = -1;    /* last hover X */
static int  g_hy   = -1;    /* last hover Y */
static int  g_tick = 0;     /* animation tick (500 ms per increment) */

static install_config_t g_cfg = {
    .disk          = "/dev/vda",
    .username      = "mahina",
    .hostname      = "mahina-pc",
    .timezone      = "UTC",
    .language      = "en_US.UTF-8",
    .keyboard      = "us",
    .password_hash = "",
};
static char g_pw_plain[64] = "mahina";   /* cleared before fork */

/* ── Forward declarations ────────────────────────────────────────────────── */

static void on_pipe_data(int fd, void *ud);
static void dispatch(const char *action);

/* ═══════════════════════════════════════════════════════════════════════════
 *  DRAWING PRIMITIVES
 * ═══════════════════════════════════════════════════════════════════════════ */

/* Draw text centred horizontally within a box */
static void text_centre(lgui_canvas_t *c, int bx, int by, int bw, int bh,
                         const char *t, uint32_t col)
{
    int tw = (int)strlen(t) * GLYPH_W;
    int tx = bx + (bw - tw) / 2;
    int ty = by + (bh - GLYPH_H) / 2;
    lgui_canvas_draw_text(c, tx, ty, t, col);
}

/* Vertical gradient (two colour stops, horizontal bands, cheap but effective) */
static void grad_v(lgui_canvas_t *c,
                    int x, int y, int w, int h,
                    uint32_t top_col, uint32_t bot_col)
{
    uint8_t tr = (top_col >> 16) & 0xFF, tg = (top_col >> 8) & 0xFF, tb = top_col & 0xFF;
    uint8_t br = (bot_col >> 16) & 0xFF, bg = (bot_col >> 8) & 0xFF, bb = bot_col & 0xFF;
    for (int i = 0; i < h; i++) {
        int r = tr + (int)(br - tr) * i / (h > 1 ? h - 1 : 1);
        int g = tg + (int)(bg - tg) * i / (h > 1 ? h - 1 : 1);
        int b = tb + (int)(bb - tb) * i / (h > 1 ? h - 1 : 1);
        uint32_t col = 0xFF000000u | ((uint32_t)r << 16) | ((uint32_t)g << 8) | (uint32_t)b;
        lgui_canvas_fill_rect(c, x, y + i, w, 1, col);
    }
}

/* Horizontal gradient */
static void grad_h(lgui_canvas_t *c,
                    int x, int y, int w, int h,
                    uint32_t l_col, uint32_t r_col)
{
    uint8_t lr = (l_col >> 16) & 0xFF, lg = (l_col >> 8) & 0xFF, lb = l_col & 0xFF;
    uint8_t rr = (r_col >> 16) & 0xFF, rg = (r_col >> 8) & 0xFF, rb = r_col & 0xFF;
    for (int i = 0; i < w; i++) {
        int r = lr + (int)(rr - lr) * i / (w > 1 ? w - 1 : 1);
        int g = lg + (int)(rg - lg) * i / (w > 1 ? w - 1 : 1);
        int b = lb + (int)(rb - lb) * i / (w > 1 ? w - 1 : 1);
        uint32_t col = 0xFF000000u | ((uint32_t)r << 16) | ((uint32_t)g << 8) | (uint32_t)b;
        lgui_canvas_fill_rect(c, x + i, y, 1, h, col);
    }
}

/* Draw a "pill" button — filled rect + label + optionally hover-brightened */
static void pill_btn(lgui_canvas_t *c,
                      int x, int y, int w, int h,
                      const char *label, uint32_t bg, uint32_t fg,
                      const char *action)
{
    bool hov = (g_hx >= x && g_hx < x + w && g_hy >= y && g_hy < y + h);
    uint32_t actual_bg = hov ? (bg | 0x282828u) : bg;   /* lighten slightly */
    lgui_canvas_fill_rect(c, x, y, w, h, actual_bg);
    /* Top highlight line */
    lgui_canvas_fill_rect(c, x, y, w, 1, 0x22FFFFFF);
    text_centre(c, x, y, w, h, label, fg);
    hits_add(x, y, w, h, action);
}

/* Draw a selection card with left accent bar when selected */
static void sel_card(lgui_canvas_t *c,
                      int x, int y, int w, int h,
                      const char *title, const char *sub,
                      bool sel, bool hov,
                      const char *action)
{
    uint32_t bg = sel ? C_CARD_SEL : (hov ? C_CARD_HOVER : C_CARD);
    uint32_t bd = sel ? C_ACCENT   : (hov ? C_PURPLE     : C_DIVIDER);

    lgui_canvas_fill_rect(c, x, y, w, h, bg);
    lgui_canvas_draw_rect_outline(c, x, y, w, h, bd);

    if (sel) lgui_canvas_fill_rect(c, x, y, 3, h, C_ACCENT);

    int text_x = x + (sel ? 16 : 14);
    int title_y = sub ? (y + h/2 - GLYPH_H - 2) : (y + (h - GLYPH_H) / 2);
    lgui_canvas_draw_text(c, text_x, title_y, title,
                           sel ? C_TEXT : (hov ? C_TEXT : C_TEXT_DIM));
    if (sub)
        lgui_canvas_draw_text(c, text_x, y + h/2 + 4, sub, C_TEXT_FAINT);

    if (sel) {
        /* Checkmark badge, top-right */
        int bx = x + w - 28, by = y + (h - 16) / 2;
        lgui_canvas_fill_rect(c, bx, by, 20, 16, C_ACCENT);
        text_centre(c, bx, by, 20, 16, "OK", 0xFFFFFFFF);
    }

    hits_add(x, y, w, h, action);
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  SIDEBAR
 * ═══════════════════════════════════════════════════════════════════════════ */

static void draw_sidebar(lgui_canvas_t *c, int H)
{
    /* Background with subtle gradient */
    grad_v(c, 0, 0, SIDEBAR_W, H, C_SIDEBAR2, C_SIDEBAR_BG);

    /* Right-edge glow */
    lgui_canvas_fill_rect(c, SIDEBAR_W - 2, 0, 2, H, 0xFF1A1A30);

    /* ── Brand header ────────────────────────────────────── */
    grad_v(c, 0, 0, SIDEBAR_W, 58, 0xFF0C0C1E, C_SIDEBAR2);

    /* Logo mark: three stacked bars with offset (M shape idea) */
    int lx = 18, ly = 10;
    lgui_canvas_fill_rect(c, lx,      ly,      30, 6, C_ACCENT);
    lgui_canvas_fill_rect(c, lx + 8,  ly + 10, 20, 6, C_PURPLE);
    lgui_canvas_fill_rect(c, lx + 14, ly + 20, 10, 6, C_ACCENT);

    lgui_canvas_draw_text(c, lx + 38, ly + 2,  "MAHINA", C_TEXT);
    lgui_canvas_draw_text(c, lx + 38, ly + 20, "OS Setup", C_TEXT_DIM);

    /* Divider below header */
    lgui_canvas_fill_rect(c, 0, 58, SIDEBAR_W, 1, C_DIVIDER);

    /* ── Step list ───────────────────────────────────────── */
    int avail  = H - 58 - 30;
    int step_h = avail / STEP_COUNT;
    if (step_h < 40) step_h = 40;

    int circle_x = 26;

    for (int i = 0; i < STEP_COUNT; i++) {
        int sy     = 64 + i * step_h;
        int cy     = sy + step_h / 2;
        int radius = 9;

        /* Connector line from previous circle */
        if (i > 0) {
            int prev_cy = 64 + (i - 1) * step_h + step_h / 2;
            uint32_t lc = (i <= g_step) ? C_ACCENT : C_DIVIDER;
            lgui_canvas_fill_rect(c, circle_x - 1,
                                   prev_cy + radius,
                                   2,
                                   cy - radius - (prev_cy + radius),
                                   lc);
        }

        /* Circle fill */
        bool done    = (i < g_step);
        bool current = (i == g_step);
        uint32_t cfill = current ? C_ACCENT : (done ? C_STEP_DONE : 0xFF1A1A30);
        uint32_t cbord = current ? C_ACCENT : (done ? C_PURPLE    : C_DIVIDER);
        lgui_canvas_fill_rect(c, circle_x - radius, cy - radius,
                               radius * 2, radius * 2, cfill);
        lgui_canvas_draw_rect_outline(c, circle_x - radius, cy - radius,
                                       radius * 2, radius * 2, cbord);

        /* Glyph inside circle */
        char num[4];
        if (done)    snprintf(num, sizeof(num), "v");   /* checkmark */
        else         snprintf(num, sizeof(num), "%d", i + 1);
        lgui_canvas_draw_text(c,
                               circle_x - (i >= 9 ? GLYPH_W : GLYPH_W / 2),
                               cy - GLYPH_H / 2,
                               num, 0xFFFFFFFF);

        /* Step name */
        uint32_t nc = current ? C_TEXT : (done ? C_TEXT_DIM : C_TEXT_FAINT);
        lgui_canvas_draw_text(c, circle_x + radius + 8,
                               cy - GLYPH_H / 2,
                               STEP_LABEL[i], nc);
    }

    /* Version footer */
    lgui_canvas_draw_text(c, 14, H - 20, "Mahina v0.2 Waxing", C_TEXT_FAINT);
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  PER-PAGE CONTENT AREA HELPERS
 * ═══════════════════════════════════════════════════════════════════════════ */

#define CX  (SIDEBAR_W + 2)   /* content-area left edge, canvas-local */

/* Page heading — gradient band with left accent bar */
static void draw_header(lgui_canvas_t *c, int W,
                          const char *title, const char *sub)
{
    grad_h(c, CX, 0, W - CX, HEADER_H, 0xFF13132A, C_BG);
    lgui_canvas_fill_rect(c, CX, 0, 4, HEADER_H, C_ACCENT);
    lgui_canvas_draw_text(c, CX + 16, 18, title, C_TEXT);
    if (sub)
        lgui_canvas_draw_text(c, CX + 16, 44, sub, C_TEXT_DIM);
    lgui_canvas_fill_rect(c, CX, HEADER_H - 1, W - CX, 1, C_DIVIDER);
}

/* Bottom navigation row */
static void draw_nav(lgui_canvas_t *c, int W, int H,
                      bool back, bool next, bool install_btn)
{
    /* Divider */
    lgui_canvas_fill_rect(c, CX, H - FOOTER_H, W - CX, 1, C_DIVIDER);

    int nav_y  = H - FOOTER_H + (FOOTER_H - 36) / 2;

    if (back && g_step > 0) {
        int bx = CX + 16;
        bool hov = (g_hx >= bx && g_hx < bx + 110 &&
                    g_hy >= nav_y && g_hy < nav_y + 36);
        lgui_canvas_fill_rect(c, bx, nav_y, 110, 36,
                               hov ? C_CARD_HOVER : C_CARD);
        lgui_canvas_draw_rect_outline(c, bx, nav_y, 110, 36, C_DIVIDER);
        text_centre(c, bx, nav_y, 110, 36, "<- Back", C_TEXT_DIM);
        hits_add(bx, nav_y, 110, 36, "back");
    }

    if (next) {
        int bw = 190, bx = W - bw - 16;
        pill_btn(c, bx, nav_y, bw, 36, "Continue  ->", C_ACCENT,
                  0xFFFFFFFF, "next");
    }

    if (install_btn) {
        int bw = 230, bx = W - bw - 16;
        bool hov = (g_hx >= bx && g_hx < bx + bw &&
                    g_hy >= nav_y && g_hy < nav_y + 36);
        lgui_canvas_fill_rect(c, bx, nav_y, bw, 36,
                               hov ? 0xFFBB2040 : C_DANGER);
        lgui_canvas_fill_rect(c, bx, nav_y, bw, 1, 0x22FFFFFF);
        text_centre(c, bx, nav_y, bw, 36, "Erase Disk + Install", 0xFFFFFFFF);
        hits_add(bx, nav_y, bw, 36, "install");
    }
}

/* Selection card grid: 2 columns */
static void card_grid(lgui_canvas_t *c,
                        int W, int H,
                        const char **labels, const char **vals,
                        int count,
                        const char *current, const char *cfg_key)
{
    int cols  = 2;
    int cw    = (W - CX - CARD_MARGIN * 2 - CARD_GAP * (cols - 1)) / cols;
    int gy    = HEADER_H + 16;
    int gx    = CX + CARD_MARGIN;

    for (int i = 0; i < count; i++) {
        int col  = i % cols;
        int row  = i / cols;
        int cx_  = gx + col * (cw + CARD_GAP);
        int cy_  = gy + row * (CARD_H + CARD_GAP);
        bool sel = (strcmp(current, vals[i]) == 0);
        bool hov = (g_hx >= cx_ && g_hx < cx_ + cw &&
                    g_hy >= cy_ && g_hy < cy_ + CARD_H);
        char action[80];
        snprintf(action, sizeof(action), "%s=%s", cfg_key, vals[i]);
        sel_card(c, cx_, cy_, cw, CARD_H, labels[i], NULL, sel, hov, action);
    }
    (void)H;
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  PAGE 0 — WELCOME
 * ═══════════════════════════════════════════════════════════════════════════ */

static void draw_welcome(lgui_canvas_t *c, int W, int H)
{
    /* Ambient star-field backdrop — short horizontal dashes */
    for (int r = 0; r < 40; r++) {
        int sx = CX + ((r * 137 + 11) % (W - CX - 20));
        int sy = (r * 193 + 7) % (H - 20);
        lgui_canvas_fill_rect(c, sx, sy, 2, 1,
                               (r % 3 == 0) ? 0xFF2A2A4A : 0xFF1E1E38);
    }

    /* Central splash */
    int mid_x = CX + (W - CX) / 2;
    int mid_y = H / 2;

    /* Logo mark (larger, 3× the sidebar version) */
    int lx = mid_x - 56, ly = mid_y - 120;
    lgui_canvas_fill_rect(c, lx,      ly,      112, 14, C_ACCENT);
    lgui_canvas_fill_rect(c, lx + 20, ly + 22, 72,  14, C_PURPLE);
    lgui_canvas_fill_rect(c, lx + 40, ly + 44, 32,  14, C_ACCENT);
    /* Glow line */
    lgui_canvas_fill_rect(c, lx - 16, ly + 58, 144, 2, C_ACCENT_DIM);

    /* OS name */
    const char *t1 = "MAHINA OS";
    lgui_canvas_draw_text(c, mid_x - (int)strlen(t1) * GLYPH_W / 2,
                           ly + 72, t1, C_TEXT);

    /* Tagline */
    const char *t2 = "A new kind of personal operating system.";
    lgui_canvas_draw_text(c, mid_x - (int)strlen(t2) * GLYPH_W / 2,
                           ly + 96, t2, C_TEXT_DIM);

    /* Feature strip */
    const char *t3 = "Luna AI  *  Built from scratch  *  Your OS, your rules";
    lgui_canvas_draw_text(c, mid_x - (int)strlen(t3) * GLYPH_W / 2,
                           ly + 116, t3, C_TEXT_FAINT);

    /* CTA button */
    int bw = 240, bh = 44;
    int bx = mid_x - bw / 2, by = mid_y + 50;
    pill_btn(c, bx, by, bw, bh, "Get Started  ->", C_ACCENT,
              0xFFFFFFFF, "next");

    /* Tiny "press Enter" hint */
    const char *hint = "(or press Enter)";
    lgui_canvas_draw_text(c, mid_x - (int)strlen(hint) * GLYPH_W / 2,
                           by + bh + 14, hint, C_TEXT_FAINT);
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  PAGES 1-3 — LANGUAGE / KEYBOARD / TIMEZONE
 * ═══════════════════════════════════════════════════════════════════════════ */

static void draw_language(lgui_canvas_t *c, int W, int H)
{
    draw_header(c, W, "Language", "Choose the system language.");
    static const char *labels[] = {
        "English (US)","Espanol","Francais",
        "Deutsch","Japanese","Chinese (Simplified)"
    };
    static const char *vals[] = {
        "en_US.UTF-8","es_ES.UTF-8","fr_FR.UTF-8",
        "de_DE.UTF-8","ja_JP.UTF-8","zh_CN.UTF-8"
    };
    card_grid(c, W, H, labels, vals, 6, g_cfg.language, "language");
    draw_nav(c, W, H, true, true, false);
}

static void draw_keyboard(lgui_canvas_t *c, int W, int H)
{
    draw_header(c, W, "Keyboard Layout", "Choose your keyboard layout.");
    static const char *labels[] = {
        "US (QWERTY)", "UK",        "German (QWERTZ)",
        "French (AZERTY)", "Spanish","Japanese"
    };
    static const char *vals[] = {
        "us", "gb", "de", "fr", "es", "jp"
    };
    card_grid(c, W, H, labels, vals, 6, g_cfg.keyboard, "keyboard");
    draw_nav(c, W, H, true, true, false);
}

static void draw_timezone(lgui_canvas_t *c, int W, int H)
{
    draw_header(c, W, "Timezone", "Select your region.");
    static const char *labels[] = {
        "UTC",           "US / Eastern",  "US / Pacific",
        "Europe/London", "Asia/Kolkata",   "Asia/Tokyo",
        "Australia",     "Brazil"
    };
    static const char *vals[] = {
        "UTC",              "America/New_York",   "America/Los_Angeles",
        "Europe/London",    "Asia/Kolkata",        "Asia/Tokyo",
        "Australia/Sydney", "America/Sao_Paulo"
    };
    card_grid(c, W, H, labels, vals, 8, g_cfg.timezone, "timezone");
    draw_nav(c, W, H, true, true, false);
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  PAGE 4 — DISK SELECTION
 * ═══════════════════════════════════════════════════════════════════════════ */

static void enumerate_disks(void)
{
    if (g_ndisks > 0) return;  /* Only run once */
    FILE *f = fopen("/proc/partitions", "r");
    if (!f) {
        snprintf(g_disks[0], sizeof(g_disks[0]), "/dev/vda");
        g_disk_gb[0] = 64;
        g_ndisks = 1;
        return;
    }
    char line[128];
    while (fgets(line, sizeof(line), f) && g_ndisks < MAX_DISKS) {
        unsigned int maj, min;
        unsigned long long blks;
        char name[64];
        if (sscanf(line, " %u %u %llu %63s", &maj, &min, &blks, name) != 4)
            continue;
        if (strncmp(name, "loop", 4) == 0) continue;
        if (strncmp(name, "ram",  3) == 0) continue;
        if (strncmp(name, "dm-",  3) == 0) continue;
        size_t nl = strlen(name);
        bool nvme = (strncmp(name, "nvme", 4) == 0 ||
                     strncmp(name, "mmcblk", 6) == 0);
        if (nvme) {
            const char *pp = strrchr(name, 'p');
            if (pp && pp > name + 4 && pp[1] >= '0' && pp[1] <= '9') continue;
        } else {
            if (nl > 0 && name[nl - 1] >= '0' && name[nl - 1] <= '9') continue;
        }
        if (blks < 2097152ULL) continue;  /* < 1 GiB */

        snprintf(g_disks[g_ndisks], sizeof(g_disks[0]), "/dev/%s", name);

        char sysfs[128];
        snprintf(sysfs, sizeof(sysfs), "/sys/block/%s/size", name);
        unsigned long long sectors = 0;
        FILE *sf = fopen(sysfs, "r");
        if (sf) { (void)fscanf(sf, "%llu", &sectors); fclose(sf); }
        g_disk_gb[g_ndisks] = (size_t)(sectors * 512ULL / (1024ULL * 1024ULL * 1024ULL));
        g_ndisks++;
    }
    fclose(f);
    if (g_ndisks == 0) {
        snprintf(g_disks[0], sizeof(g_disks[0]), "/dev/vda");
        g_disk_gb[0] = 64;
        g_ndisks = 1;
    }
}

static void draw_disk(lgui_canvas_t *c, int W, int H)
{
    draw_header(c, W, "Select Installation Disk",
                 "All data on the selected disk will be permanently erased.");

    int cx = CX + CARD_MARGIN;
    int cw = W - CX - CARD_MARGIN * 2;
    int gy = HEADER_H + 16;

    for (int i = 0; i < g_ndisks; i++) {
        int cy = gy + i * (DISK_CARD_H + CARD_GAP);
        bool sel = (strcmp(g_cfg.disk, g_disks[i]) == 0);
        bool hov = (g_hx >= cx && g_hx < cx + cw &&
                    g_hy >= cy && g_hy < cy + DISK_CARD_H);

        uint32_t bg = sel ? C_CARD_SEL : (hov ? C_CARD_HOVER : C_CARD);
        uint32_t bd = sel ? C_ACCENT   : (hov ? C_PURPLE     : C_DIVIDER);

        lgui_canvas_fill_rect(c, cx, cy, cw, DISK_CARD_H, bg);
        lgui_canvas_draw_rect_outline(c, cx, cy, cw, DISK_CARD_H, bd);
        if (sel) lgui_canvas_fill_rect(c, cx, cy, 3, DISK_CARD_H, C_ACCENT);

        /* Disk name */
        lgui_canvas_draw_text(c, cx + 16, cy + 10,
                               g_disks[i], sel ? C_TEXT : C_TEXT_DIM);

        /* Size label (top-right) */
        char sz[32];
        snprintf(sz, sizeof(sz), "%zu GiB", g_disk_gb[i]);
        int sz_w = (int)strlen(sz) * GLYPH_W;
        lgui_canvas_draw_text(c, cx + cw - sz_w - 12, cy + 10,
                               sz, C_TEXT_DIM);

        /* Visual partition diagram bar */
        int bx = cx + 16, bar_y = cy + 36;
        int bar_w = cw - 32, bar_h = 16;

        /* Total MB → proportion for ESP */
        unsigned long long total_mb = (unsigned long long)g_disk_gb[i] * 1024ULL;
        int esp_px = (total_mb > 0)
                     ? (int)(512ULL * (unsigned long long)bar_w / total_mb)
                     : 10;
        if (esp_px < 10)  esp_px = 10;
        if (esp_px > 40)  esp_px = 40;

        lgui_canvas_fill_rect(c, bx, bar_y, bar_w, bar_h, 0xFF1A1A2E);
        lgui_canvas_fill_rect(c, bx, bar_y, esp_px, bar_h, 0xFF4A9EF5);/* ESP */
        lgui_canvas_fill_rect(c, bx + esp_px, bar_y,
                               bar_w - esp_px, bar_h, C_ACCENT);          /* Root */
        lgui_canvas_draw_rect_outline(c, bx, bar_y, bar_w, bar_h, C_DIVIDER);

        /* Labels under bar */
        lgui_canvas_draw_text(c, bx, bar_y + bar_h + 6,
                               "EFI 512M", C_TEXT_FAINT);
        lgui_canvas_draw_text(c, bx + esp_px + 8, bar_y + bar_h + 6,
                               "Btrfs root", sel ? C_TEXT_DIM : C_TEXT_FAINT);

        /* Hit area */
        char action[80];
        snprintf(action, sizeof(action), "disk=%s", g_disks[i]);
        hits_add(cx, cy, cw, DISK_CARD_H, action);
    }

    draw_nav(c, W, H, true, true, false);
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  PAGE 5 — PARTITION LAYOUT OVERVIEW
 * ═══════════════════════════════════════════════════════════════════════════ */

static void draw_partitions(lgui_canvas_t *c, int W, int H)
{
    draw_header(c, W, "Partition Layout",
                 "Mahina will automatically create this layout.");

    int px = CX + CARD_MARGIN;
    int pw = W - CX - CARD_MARGIN * 2;
    int py = HEADER_H + 20;

    /* Visual disk bar */
    int bar_h = 44;
    int esp_px = pw / 14;   /* ~7 % for ESP */
    grad_h(c, px, py, esp_px, bar_h,  0xFF2A6ABF, 0xFF4A9EF5); /* ESP */
    grad_h(c, px + esp_px, py, pw - esp_px, bar_h, C_ACCENT, C_PURPLE); /* Root */
    lgui_canvas_draw_rect_outline(c, px, py, pw, bar_h, C_DIVIDER);

    /* Labels inside the bar */
    text_centre(c, px, py, esp_px, bar_h, "EFI", 0xFFFFFFFF);
    text_centre(c, px + esp_px, py, pw - esp_px, bar_h,
                "Btrfs root  ( @root | @home | @snapshots )", 0xFFFFFFFF);

    py += bar_h + 24;

    /* Detail table */
    typedef struct { const char *col1; const char *col2; } row_t;
    static const row_t rows[] = {
        { "Partition 1  (EFI System)",  "512 MiB  |  FAT32  |  /boot/efi" },
        { "Partition 2  (Root)",         "Remaining  |  Btrfs  |  /" },
        { "", "" },
        { "Btrfs subvolumes:", "" },
        { "  @root",      "mounted at  /" },
        { "  @home",      "mounted at  /home" },
        { "  @snapshots", "mounted at  /.snapshots" },
        { "", "" },
        { "Swap:", "none  (zram virtual swap used automatically)" },
        { "Bootloader:", "Limine UEFI  ->  /boot/efi/EFI/BOOT/BOOTX64.EFI" },
    };
    for (int i = 0; i < (int)(sizeof(rows) / sizeof(rows[0])); i++) {
        int ry = py + i * (GLYPH_H + 5);
        if (rows[i].col1[0]) {
            lgui_canvas_draw_text(c, px, ry, rows[i].col1,
                                   rows[i].col1[0] == ' ' ? C_TEXT_DIM : C_TEXT);
            if (rows[i].col2[0])
                lgui_canvas_draw_text(c, px + 200, ry,
                                       rows[i].col2, C_TEXT_DIM);
        }
    }

    draw_nav(c, W, H, true, true, false);
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  PAGE 6 — ACCOUNT SETUP
 * ═══════════════════════════════════════════════════════════════════════════ */

static void draw_input_field(lgui_canvas_t *c,
                               int x, int y, int w, int h,
                               const char *label,
                               int inp_idx)
{
    const tinput_t *inp = &g_inp[inp_idx];
    bool focused = (g_focus == inp_idx);
    bool cursor_visible = focused && ((g_tick % 2) == 0);

    /* Label */
    lgui_canvas_draw_text(c, x, y - 20, label, C_TEXT_DIM);

    /* Box */
    lgui_canvas_fill_rect(c, x, y, w, h, C_INPUT_BG);
    uint32_t border = focused ? C_ACCENT : C_DIVIDER;
    lgui_canvas_draw_rect_outline(c, x, y, w, h, border);
    if (focused) lgui_canvas_fill_rect(c, x, y, 2, h, C_ACCENT);

    /* Display text */
    char disp[96] = "";
    if (inp->password) {
        for (int i = 0; i < inp->len && i < (int)sizeof(disp) - 1; i++)
            disp[i] = '*';
    } else {
        snprintf(disp, sizeof(disp), "%s", inp->buf);
    }
    if (inp->len == 0 && !focused)
        lgui_canvas_draw_text(c, x + 10, y + (h - GLYPH_H) / 2,
                               "(empty)", C_TEXT_FAINT);
    else
        lgui_canvas_draw_text(c, x + 10, y + (h - GLYPH_H) / 2, disp, C_TEXT);

    /* Cursor */
    if (cursor_visible) {
        int tw = inp->len * GLYPH_W;
        lgui_canvas_fill_rect(c, x + 10 + tw, y + 6, 2, h - 12, C_ACCENT);
    }

    /* Hit area for focus */
    char action[20];
    snprintf(action, sizeof(action), "focus=%d", inp_idx);
    hits_add(x, y, w, h, action);
}

static void draw_account(lgui_canvas_t *c, int W, int H)
{
    draw_header(c, W, "Create Your Account",
                 "Set up the administrator account for this system.");

    int fx  = CX + CARD_MARGIN;
    int fw  = (W - CX - CARD_MARGIN * 2);
    if (fw > 520) fw = 520;
    int py  = HEADER_H + 36;

    draw_input_field(c, fx, py, fw, INPUT_H, "Username", 0);
    py += INPUT_H + 40;
    draw_input_field(c, fx, py, fw, INPUT_H, "Computer Name (hostname)", 1);
    py += INPUT_H + 40;
    draw_input_field(c, fx, py, fw, INPUT_H, "Password", 2);
    py += INPUT_H + 18;

    lgui_canvas_draw_text(c, fx, py,
                           "Press Tab to move between fields. Enter to continue.",
                           C_TEXT_FAINT);

    draw_nav(c, W, H, true, true, false);
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  PAGE 7 — SUMMARY / CONFIRM
 * ═══════════════════════════════════════════════════════════════════════════ */

static void draw_confirm(lgui_canvas_t *c, int W, int H)
{
    draw_header(c, W, "Ready to Install",
                 "Review your choices. This cannot be undone.");

    int px = CX + CARD_MARGIN;
    int pw = W - CX - CARD_MARGIN * 2;
    int py = HEADER_H + 20;

    /* Summary card */
    int card_h = 140;
    lgui_canvas_fill_rect(c, px, py, pw, card_h, C_CARD);
    lgui_canvas_draw_rect_outline(c, px, py, pw, card_h, C_DIVIDER);
    lgui_canvas_fill_rect(c, px, py, 3, card_h, C_PURPLE);

    typedef struct { const char *label; const char *val; } kv_t;
    kv_t rows[] = {
        { "Language:",  g_cfg.language },
        { "Keyboard:",  g_cfg.keyboard },
        { "Timezone:",  g_cfg.timezone },
        { "Disk:",      g_cfg.disk     },
        { "Username:",  g_cfg.username },
        { "Hostname:",  g_cfg.hostname },
    };
    for (int i = 0; i < (int)(sizeof(rows) / sizeof(rows[0])); i++) {
        int ry = py + 10 + i * (GLYPH_H + 6);
        lgui_canvas_draw_text(c, px + 14, ry, rows[i].label, C_TEXT_DIM);
        lgui_canvas_draw_text(c, px + 120, ry, rows[i].val, C_TEXT);
    }

    /* Danger warning banner */
    py += card_h + 20;
    lgui_canvas_fill_rect(c, px, py, pw, 50, 0xFF190808);
    lgui_canvas_draw_rect_outline(c, px, py, pw, 50, C_DANGER);
    lgui_canvas_fill_rect(c, px, py, 3, 50, C_DANGER);
    lgui_canvas_draw_text(c, px + 12, py + 8,
                           "WARNING: All data on the selected disk will be",
                           C_DANGER);
    lgui_canvas_draw_text(c, px + 12, py + 28,
                           "permanently erased. This cannot be undone.",
                           C_TEXT_DIM);

    draw_nav(c, W, H, true, false, true);
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  PAGE 8 — INSTALLING
 * ═══════════════════════════════════════════════════════════════════════════ */

static void draw_installing(lgui_canvas_t *c, int W, int H)
{
    draw_header(c, W, "Installing MahinaOS",
                 "Please wait. Do not power off your computer.");

    int px = CX + CARD_MARGIN;
    int pw = W - CX - CARD_MARGIN * 2;
    int py = HEADER_H + 50;

    if (g_install_err) {
        lgui_canvas_fill_rect(c, px, py, pw, 52, 0xFF180808);
        lgui_canvas_draw_rect_outline(c, px, py, pw, 52, C_DANGER);
        lgui_canvas_draw_text(c, px + 12, py + 10,
                               "Installation failed:", C_DANGER);
        lgui_canvas_draw_text(c, px + 12, py + 30, g_pmsg, C_TEXT_DIM);
        return;
    }

    /* Progress bar track */
    int bar_h = 40;
    lgui_canvas_fill_rect(c, px, py, pw, bar_h, C_CARD);
    lgui_canvas_draw_rect_outline(c, px, py, pw, bar_h, C_DIVIDER);

    /* Filled portion (gradient pink → purple) */
    int fill_w = (int)((long long)pw * g_pct / 100);
    if (fill_w > 0)
        grad_h(c, px, py, fill_w, bar_h, C_PURPLE, C_ACCENT);

    /* Percentage overlay */
    char pct_s[8];
    snprintf(pct_s, sizeof(pct_s), "%d%%", g_pct);
    text_centre(c, px, py, pw, bar_h, pct_s, 0xFFFFFFFF);

    /* Status message */
    py += bar_h + 18;
    int mw = (int)strlen(g_pmsg) * GLYPH_W;
    lgui_canvas_draw_text(c, px + (pw - mw) / 2, py, g_pmsg, C_TEXT_DIM);

    /* Animated dot indicator */
    py += GLYPH_H + 20;
    static const char *frames[] = { "Mahina  .  ", "Mahina  .. ", "Mahina  ..." };
    int frame = (g_tick / 1) % 3;   /* 500ms per frame */
    int fw2 = (int)strlen(frames[frame]) * GLYPH_W;
    lgui_canvas_draw_text(c, px + (pw - fw2) / 2, py,
                           frames[frame], C_ACCENT);

    (void)H;
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  PAGE 9 — DONE
 * ═══════════════════════════════════════════════════════════════════════════ */

static void draw_done(lgui_canvas_t *c, int W, int H)
{
    int mid_x = CX + (W - CX) / 2;
    int mid_y = H / 2;

    /* Large success box */
    int sz = 64;
    int bx = mid_x - sz / 2, by = mid_y - sz / 2 - 60;
    lgui_canvas_fill_rect(c, bx, by, sz, sz, C_SUCCESS);
    lgui_canvas_draw_rect_outline(c, bx, by, sz, sz, C_TEAL);
    text_centre(c, bx, by, sz, sz, "DONE", 0xFF0A1A14);

    /* Messages */
    const char *m1 = "Installation Complete!";
    const char *m2 = "Remove any installation media, then reboot.";
    const char *m3 = "Welcome to Mahina.";
    lgui_canvas_draw_text(c, mid_x - (int)strlen(m1) * GLYPH_W / 2,
                           by + sz + 16, m1, C_TEXT);
    lgui_canvas_draw_text(c, mid_x - (int)strlen(m2) * GLYPH_W / 2,
                           by + sz + 40, m2, C_TEXT_DIM);
    lgui_canvas_draw_text(c, mid_x - (int)strlen(m3) * GLYPH_W / 2,
                           by + sz + 72, m3, C_ACCENT);

    /* Reboot button */
    int bw = 200, bh = 40;
    int rbx = mid_x - bw / 2, rby = mid_y + 80;
    pill_btn(c, rbx, rby, bw, bh, "Reboot Now", C_SUCCESS, 0xFF0A1A14, "reboot");
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  MASTER CANVAS RENDER CALLBACK
 * ═══════════════════════════════════════════════════════════════════════════ */

static void render_all(lgui_widget_t *w __attribute__((unused)),
                        lgui_canvas_t *c,
                        int ox __attribute__((unused)),
                        int oy __attribute__((unused)))
{
    int W = (int)g_sw, H = (int)g_sh;

    /* Clear frame */
    lgui_canvas_fill_rect(c, 0, 0, W, H, C_BG);

    /* Content area background */
    lgui_canvas_fill_rect(c, CX, 0, W - CX, H, C_PANEL);

    /* Reset hit registry for this frame */
    hits_clear();

    /* Sidebar */
    draw_sidebar(c, H);

    /* Vertical divider */
    lgui_canvas_fill_rect(c, SIDEBAR_W, 0, 2, H, C_DIVIDER);

    /* Per-page content */
    switch (g_step) {
        case 0: draw_welcome(c, W, H);      break;
        case 1: draw_language(c, W, H);     break;
        case 2: draw_keyboard(c, W, H);     break;
        case 3: draw_timezone(c, W, H);     break;
        case 4: draw_disk(c, W, H);         break;
        case 5: draw_partitions(c, W, H);   break;
        case 6: draw_account(c, W, H);      break;
        case 7: draw_confirm(c, W, H);      break;
        case 8: draw_installing(c, W, H);   break;
        case 9: draw_done(c, W, H);         break;
        default: break;
    }

    /* Custom cursor (2×2 hot-pink dot) */
    if (g_hx >= 0 && g_hy >= 0) {
        lgui_canvas_fill_rect(c, g_hx - 2, g_hy - 2, 5, 5, C_ACCENT);
        lgui_canvas_fill_rect(c, g_hx,     g_hy,     1, 1, 0xFFFFFFFF);
    }
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  ACTION DISPATCHER
 * ═══════════════════════════════════════════════════════════════════════════ */

static void launch_install(void)
{
    /* Copy text-input data into cfg (user already pressed Continue on acct) */
    snprintf(g_cfg.username, sizeof(g_cfg.username), "%s", g_inp[0].buf);
    snprintf(g_cfg.hostname, sizeof(g_cfg.hostname), "%s", g_inp[1].buf);

    /* Hash password */
    if (g_inp[2].len > 0) {
        if (install_hash_password(g_inp[2].buf, g_cfg.password_hash,
                                   sizeof(g_cfg.password_hash)) != 0)
            g_cfg.password_hash[0] = '\0';
        explicit_bzero(g_inp[2].buf, sizeof(g_inp[2].buf));
        g_inp[2].len = 0;
    }
    explicit_bzero(g_pw_plain, sizeof(g_pw_plain));

    /* Progress pipe */
    int pfd[2];
    if (pipe2(pfd, O_CLOEXEC | O_NONBLOCK) != 0) {
        g_install_err = true;
        snprintf(g_pmsg, sizeof(g_pmsg), "ERROR: pipe2(): %s", strerror(errno));
        g_step = 8;
        lgui_window_update(g_win);
        return;
    }
    g_pipe_rd     = pfd[0];
    g_install_done = g_install_err = false;
    g_pct = 0;
    snprintf(g_pmsg, sizeof(g_pmsg), "Starting...");

    g_worker_pid = fork();
    if (g_worker_pid == 0) {
        close(pfd[0]);
        install_worker_run(pfd[1], &g_cfg);
        /* never returns */
    } else if (g_worker_pid < 0) {
        close(pfd[0]); close(pfd[1]);
        g_install_err = true;
        snprintf(g_pmsg, sizeof(g_pmsg), "ERROR: fork(): %s", strerror(errno));
        g_step = 8;
        lgui_window_update(g_win);
        return;
    }
    close(pfd[1]);
    lgui_application_add_fd(g_app, g_pipe_rd, on_pipe_data, NULL);
    g_step = 8;
    lgui_window_update(g_win);
}

static void dispatch(const char *action)
{
    if (!action || !action[0]) return;

    if (strcmp(action, "next") == 0) {
        if (g_step < 9) g_step++;
        lgui_window_update(g_win);
        return;
    }
    if (strcmp(action, "back") == 0) {
        if (g_step > 0) g_step--;
        lgui_window_update(g_win);
        return;
    }
    if (strcmp(action, "install") == 0) {
        launch_install();
        return;
    }
    if (strcmp(action, "reboot") == 0) {
        lgui_application_quit(g_app);
        return;
    }

    /* focus=N  — text input focus */
    if (strncmp(action, "focus=", 6) == 0) {
        int idx = atoi(action + 6);
        if (idx >= 0 && idx < N_INPUTS) g_focus = idx;
        lgui_window_update(g_win);
        return;
    }

    /* key=value  — selection cards */
    const char *eq = strchr(action, '=');
    if (eq) {
        char key[32], val[96];
        size_t kl = (size_t)(eq - action);
        if (kl >= sizeof(key)) kl = sizeof(key) - 1;
        memcpy(key, action, kl); key[kl] = '\0';
        snprintf(val, sizeof(val), "%s", eq + 1);
        if      (strcmp(key, "language") == 0)
            snprintf(g_cfg.language, sizeof(g_cfg.language), "%s", val);
        else if (strcmp(key, "keyboard") == 0)
            snprintf(g_cfg.keyboard, sizeof(g_cfg.keyboard), "%s", val);
        else if (strcmp(key, "timezone") == 0)
            snprintf(g_cfg.timezone, sizeof(g_cfg.timezone), "%s", val);
        else if (strcmp(key, "disk") == 0)
            snprintf(g_cfg.disk, sizeof(g_cfg.disk), "%s", val);
        lgui_window_update(g_win);
    }
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  EVENT CALLBACKS
 * ═══════════════════════════════════════════════════════════════════════════ */

static void on_pointer(int px, int py, bool pressed, bool btn_event, void *ud)
{
    (void)ud;
    bool moved = (px != g_hx || py != g_hy);
    g_hx = px; g_hy = py;

    if (btn_event && pressed) {
        const char *a = hits_test(px, py);
        if (a) dispatch(a);
    }
    if (moved || btn_event)
        lgui_window_update(g_win);
}

static void on_key(uint32_t key, uint32_t modifiers, void *ud)
{
    (void)ud;

    /* Global nav shortcuts when NOT on account page */
    if (g_step != 6) {
        if (key == KEY_ESC  && g_step > 0)  { g_step--; lgui_window_update(g_win); return; }
        if (key == KEY_ENTER && g_step < 8)  { dispatch("next"); return; }
        return;
    }

    /* Text input (account page) */
    tinput_t *inp = &g_inp[g_focus];

    if (key == KEY_TAB) {
        g_focus = (g_focus + 1) % N_INPUTS;
        lgui_window_update(g_win);
        return;
    }
    if (key == KEY_BACKSPACE) {
        if (inp->len > 0) inp->buf[--inp->len] = '\0';
        lgui_window_update(g_win);
        return;
    }
    if (key == KEY_ENTER) {
        if (g_focus < N_INPUTS - 1) g_focus++;
        else dispatch("next");
        lgui_window_update(g_win);
        return;
    }

    char ch = lgui_keymap_translate(key, modifiers);
    if (ch >= 32 && ch < 127 && inp->len < inp->max_len) {
        inp->buf[inp->len++] = ch;
        inp->buf[inp->len]   = '\0';
        lgui_window_update(g_win);
    }
}

static void on_pipe_data(int fd, void *ud)
{
    (void)ud;
    char buf[512];
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    if (n <= 0) return;
    buf[n] = '\0';

    char *line = buf, *nl;
    while ((nl = strchr(line, '\n')) != NULL) {
        *nl = '\0';
        int pct; char msg[256];
        if (sscanf(line, "%d|%255[^\n]", &pct, msg) == 2) {
            if (pct == 100) {
                g_install_done = true;
                g_pct  = 100;
                snprintf(g_pmsg, sizeof(g_pmsg), "Complete!");
                g_step = 9;
            } else if (pct < 0 || strncmp(msg, "ERROR:", 6) == 0) {
                g_install_err = true;
                snprintf(g_pmsg, sizeof(g_pmsg), "%s", msg);
            } else if (pct >= 0) {
                g_pct = pct;
                snprintf(g_pmsg, sizeof(g_pmsg), "%s", msg);
            }
        }
        line = nl + 1;
    }
    lgui_window_update(g_win);
}

static void on_timer(int fd, void *ud)
{
    (void)ud;
    uint64_t exp;
    if (read(fd, &exp, sizeof(exp)) > 0) {
        g_tick++;
        /* Animate cursor blink (page 6) and progress dots (page 8) */
        if (g_step == 6 || g_step == 8)
            lgui_window_update(g_win);
    }
}

/* ═══════════════════════════════════════════════════════════════════════════
 *  ENTRY POINT
 * ═══════════════════════════════════════════════════════════════════════════ */

int main(void)
{
    g_app = lgui_application_create("luna-installer");
    if (!g_app) {
        fprintf(stderr, "luna-installer: cannot connect to lgp-compositor\n");
        return 1;
    }

    /* Detect screen size */
    g_sw = lgui_output_width(g_app);
    g_sh = lgui_output_height(g_app);
    if (g_sw < 640) g_sw = 1024;
    if (g_sh < 480) g_sh = 768;

    /* Create full-screen window */
    g_win = lgui_window_create(g_app, g_sw, g_sh, LGUI_LAYER_APPLICATION);
    if (!g_win) {
        fprintf(stderr, "luna-installer: cannot create window\n");
        lgui_application_destroy(g_app);
        return 1;
    }

    /* Full-screen canvas widget — single root, draws everything */
    g_canvas = lgui_canvas_widget_create();
    lgui_widget_set_size(g_canvas, (int)g_sw, (int)g_sh);
    lgui_canvas_widget_set_render(g_canvas, render_all);
    lgui_window_set_root_widget(g_win, g_canvas);

    /* Enumerate block devices before first render */
    enumerate_disks();

    /* Initialise text-input state */
    g_inp[0] = (tinput_t){ .max_len = 31, .password = false };
    g_inp[1] = (tinput_t){ .max_len = 63, .password = false };
    g_inp[2] = (tinput_t){ .max_len = 63, .password = true  };
    snprintf(g_inp[0].buf, sizeof(g_inp[0].buf), "%s", g_cfg.username);
    g_inp[0].len = (int)strlen(g_inp[0].buf);
    snprintf(g_inp[1].buf, sizeof(g_inp[1].buf), "%s", g_cfg.hostname);
    g_inp[1].len = (int)strlen(g_inp[1].buf);
    snprintf(g_inp[2].buf, sizeof(g_inp[2].buf), "%s", g_pw_plain);
    g_inp[2].len = (int)strlen(g_inp[2].buf);

    /* Show initial frame */
    lgui_window_show(g_win);

    /* Register global callbacks */
    lgui_application_set_global_pointer_cb(g_app, on_pointer, NULL);
    lgui_application_set_global_key_cb(g_app,     on_key,     NULL);

    /* Animation timer — 500 ms tick for cursor blink + install dots */
    int tfd = timerfd_create(CLOCK_MONOTONIC, TFD_NONBLOCK | TFD_CLOEXEC);
    if (tfd >= 0) {
        struct itimerspec its = {
            .it_interval = { .tv_sec = 0, .tv_nsec = 500000000L },
            .it_value    = { .tv_sec = 0, .tv_nsec = 500000000L },
        };
        timerfd_settime(tfd, 0, &its, NULL);
        lgui_application_add_fd(g_app, tfd, on_timer, NULL);
    }

    lgui_application_run(g_app);
    lgui_application_destroy(g_app);
    return 0;
}
