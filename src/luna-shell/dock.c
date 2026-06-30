/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * luna-shell/dock.c — Left sidebar application launcher dock.
 *
 * Renders a vertical strip of icon tiles on the left edge of the screen.
 * Each tile shows a PSF glyph, a label below, and highlights in neon magenta
 * on hover. Clicking launches the associated binary via fork/execvp.
 *
 * Aesthetics (v0.3 spec):
 *   - Dark acrylic tile background: #0A0A0F @ 80% opacity over wallpaper
 *   - Hover: magenta border #E03E8A
 *   - Active: filled magenta with white glyph
 *   - 1px purple separator between tiles
 */

#include "dock.h"

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

/* ── Application catalogue ─────────────────────────────────────────────────*/

const dock_item_def_t g_dock_items[] = {
    { "Term",    ">_",    "luna-terminal" },
    { "Files",   "[F]",   "luna-files"    },
    { "Text",    "[T]",   "luna-text"     },
    { "Calc",    "[+]",   "luna-calc"     },
    { "Tasks",   "[*]",   "luna-tasks"    },
    { "Set",     "[S]",   "luna-settings" },
    { "About",   "[?]",   "luna-about"    },
};
const int g_dock_item_count = (int)(sizeof(g_dock_items) / sizeof(g_dock_items[0]));

/* ── Colour palette ────────────────────────────────────────────────────────*/

#define DOCK_BG_TILE      0x0D0D14u  /* Near-black tile */
#define DOCK_BG_STRIP     0x08080Eu  /* Strip background */
#define DOCK_ACCENT       0xE03E8Au  /* Neon magenta */
#define DOCK_PURPLE       0x8A2BE2u  /* Purple border */
#define DOCK_TEXT         0xC0C0D0u  /* Soft white */
#define DOCK_GLYPH_HOV    0xFFFFFFu  /* White glyph when hovered */

/* ── Helpers ───────────────────────────────────────────────────────────────*/

static void fill_rect(lgui_canvas_t *cv, int x, int y, int w, int h, uint32_t c) {
    lgui_canvas_fill_rect(cv, x, y, w, h, c);
}

static void draw_outline(lgui_canvas_t *cv, int x, int y, int w, int h, uint32_t c) {
    lgui_canvas_draw_rect_outline(cv, x, y, w, h, c);
}

/* ── Public API ────────────────────────────────────────────────────────────*/

void dock_init(dock_state_t *d, uint32_t screen_height) {
    if (!d) return;
    d->hovered = -1;
    d->height  = screen_height;
    d->canvas  = NULL;
}

void dock_render(dock_state_t *d, lgui_canvas_t *canvas, uint32_t screen_height) {
    if (!d || !canvas) return;

    /* Draw the strip background */
    fill_rect(canvas, 0, 0, DOCK_WIDTH, (int)screen_height, DOCK_BG_STRIP);

    /* Right border line */
    fill_rect(canvas, DOCK_WIDTH - 1, 0, 1, (int)screen_height, DOCK_PURPLE);

    /* Render each icon tile */
    int item_y = 72; /* Start below the top panel + small gap */
    for (int i = 0; i < g_dock_item_count; i++) {
        int tx = DOCK_PADDING;
        int ty = item_y;
        int tw = DOCK_WIDTH - DOCK_PADDING * 2;
        int th = DOCK_ICON_SZ;

        bool hovered = (d->hovered == i);

        /* Tile background */
        uint32_t tile_bg = hovered ? DOCK_ACCENT : DOCK_BG_TILE;
        fill_rect(canvas, tx, ty, tw, th, tile_bg);

        /* Tile border */
        uint32_t border_c = hovered ? DOCK_GLYPH_HOV : DOCK_PURPLE;
        draw_outline(canvas, tx, ty, tw, th, border_c);

        /* Glyph text (centered) */
        uint32_t glyph_c = hovered ? DOCK_GLYPH_HOV : DOCK_ACCENT;
        const char *glyph = g_dock_items[i].glyph;
        int gx = tx + 4;
        int gy = ty + (th - 16) / 2; /* Vertically center 16px font */
        lgui_canvas_draw_text(canvas, gx, gy, glyph, glyph_c);

        /* Label below glyph */
        uint32_t label_c = hovered ? DOCK_GLYPH_HOV : DOCK_TEXT;
        int lx = tx + 2;
        int ly = ty + th - 10;
        lgui_canvas_draw_text(canvas, lx, ly, g_dock_items[i].label, label_c);

        item_y += th + 4; /* 4px gap between tiles */
    }

    /* Luna logo at bottom of dock */
    int logo_y = (int)screen_height - 48;
    fill_rect(canvas, DOCK_PADDING, logo_y, DOCK_WIDTH - DOCK_PADDING * 2, 36, 0x1A1A3Au);
    draw_outline(canvas, DOCK_PADDING, logo_y, DOCK_WIDTH - DOCK_PADDING * 2, 36, DOCK_ACCENT);
    lgui_canvas_draw_text(canvas, DOCK_PADDING + 4, logo_y + 4,  "MA",  0xE03E8Au);
    lgui_canvas_draw_text(canvas, DOCK_PADDING + 4, logo_y + 20, "HIN", 0x8A2BE2u);
}

void dock_on_pointer(dock_state_t *d, int x, int y) {
    if (!d) return;

    /* Check if pointer is within dock strip */
    if (x < 0 || x >= DOCK_WIDTH) {
        d->hovered = -1;
        return;
    }

    /* Identify which tile */
    int item_y = 72;
    for (int i = 0; i < g_dock_item_count; i++) {
        int ty = item_y;
        int th = DOCK_ICON_SZ;
        if (y >= ty && y < ty + th) {
            d->hovered = i;
            return;
        }
        item_y += th + 4;
    }

    d->hovered = -1;
}

bool dock_on_click(dock_state_t *d, int x, int y) {
    if (!d) return false;
    if (x < 0 || x >= DOCK_WIDTH) return false;

    int item_y = 72;
    for (int i = 0; i < g_dock_item_count; i++) {
        int ty = item_y;
        int th = DOCK_ICON_SZ;
        if (y >= ty && y < ty + th) {
            /* Launch the application */
            if (fork() == 0) {
                char *argv[] = { (char *)g_dock_items[i].command, NULL };
                execvp(g_dock_items[i].command, argv);
                _exit(1);
            }
            return true;
        }
        item_y += th + 4;
    }

    return false;
}
