/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * luna-shell/wallpaper.c — Animated Luna Island wallpaper engine.
 *
 * Renders a time-of-day gradient background with an animated star field
 * entirely in software (XRGB8888). No GPU shaders required.
 *
 * Palette (from v0.3 spec):
 *   Night   : #0A0A0F (top) → #1B1B2F (bottom)  + cyan nebula wisps
 *   Morning : #1A1A2E (top) → #16213E (bottom)   + soft amber horizon
 *   Afternoon: #0F3460 (top) → #16213E (bottom)  + purple midtones
 */

#include "wallpaper.h"

#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/* ── LCG pseudo-random (no stdlib rand — deterministic seeds) ──────────────*/

static uint32_t lcg_next(uint32_t *state) {
    *state = *state * 1664525u + 1013904223u;
    return *state;
}

/* ── Colour blend helpers ──────────────────────────────────────────────────*/

static uint32_t blend_colour(uint32_t a, uint32_t b, uint32_t t) {
    /* t: 0 = full a, 255 = full b */
    uint32_t ar = (a >> 16) & 0xFF;
    uint32_t ag = (a >>  8) & 0xFF;
    uint32_t ab =  a        & 0xFF;
    uint32_t br = (b >> 16) & 0xFF;
    uint32_t bg = (b >>  8) & 0xFF;
    uint32_t bb =  b        & 0xFF;
    uint32_t r = (ar * (255 - t) + br * t) / 255;
    uint32_t g = (ag * (255 - t) + bg * t) / 255;
    uint32_t bv = (ab * (255 - t) + bb * t) / 255;
    return (r << 16) | (g << 8) | bv;
}

/* ── Per-star structure ────────────────────────────────────────────────────*/

#define MAX_STARS 220

typedef struct {
    uint16_t x;
    uint16_t y;
    uint8_t  brightness; /* 0-255 */
    uint8_t  twinkle;    /* phase offset for twinkle */
} star_t;

static star_t g_stars[MAX_STARS];
static bool   g_stars_inited = false;

static void stars_init(uint32_t width, uint32_t height) {
    uint32_t seed = 0xDEADBEEFu;
    for (int i = 0; i < MAX_STARS; i++) {
        g_stars[i].x          = (uint16_t)(lcg_next(&seed) % width);
        g_stars[i].y          = (uint16_t)(lcg_next(&seed) % (height * 3 / 4));
        g_stars[i].brightness = (uint8_t)(40 + lcg_next(&seed) % 216);
        g_stars[i].twinkle    = (uint8_t)(lcg_next(&seed) % 64);
    }
    g_stars_inited = true;
}

/* ── Wallpaper API ─────────────────────────────────────────────────────────*/

void wallpaper_init(wallpaper_state_t *ws, uint32_t width, uint32_t height) {
    if (!ws) return;
    ws->tick   = 0;
    ws->tod    = wallpaper_get_tod();
    ws->width  = width;
    ws->height = height;
    if (!g_stars_inited) stars_init(width, height);
}

void wallpaper_tick(wallpaper_state_t *ws) {
    if (!ws) return;
    ws->tick++;
    /* Re-evaluate time of day every ~10 seconds (~600 ticks at 60fps) */
    if ((ws->tick % 600) == 0) {
        ws->tod = wallpaper_get_tod();
    }
}

wallpaper_tod_t wallpaper_get_tod(void) {
    /* Read uptime seconds from /proc/uptime and map to time-of-day cycle.
     * Each full cycle = 3 minutes (180s) for demo purposes in QEMU.
     *   0-60s  = Night
     *   60-120s = Morning
     *   120-180s = Afternoon */
    FILE *f = fopen("/proc/uptime", "r");
    if (!f) return WALLPAPER_NIGHT;
    double uptime = 0.0;
    int scanned = fscanf(f, "%lf", &uptime);
    fclose(f);
    if (scanned != 1) return WALLPAPER_NIGHT;
    int phase = (int)uptime % 180;
    if (phase < 60)  return WALLPAPER_NIGHT;
    if (phase < 120) return WALLPAPER_MORNING;
    return WALLPAPER_AFTERNOON;
}

void wallpaper_render(const wallpaper_state_t *ws, void *pixels, uint32_t stride) {
    if (!ws || !pixels) return;

    uint32_t w = ws->width;
    uint32_t h = ws->height;
    uint32_t tick = ws->tick;

    /* ── Gradient colour pairs (top → bottom) ────────────────────────────*/
    uint32_t col_top, col_bot, col_mid;

    switch (ws->tod) {
    case WALLPAPER_MORNING:
        col_top = 0x1A1A2E; /* Deep navy */
        col_mid = 0x16213E; /* Darker blue */
        col_bot = 0x0F3460; /* Midnight blue */
        break;
    case WALLPAPER_AFTERNOON:
        col_top = 0x0F3460; /* Blue */
        col_mid = 0x533483; /* Purple */
        col_bot = 0x1B1B2F; /* Dark */
        break;
    case WALLPAPER_NIGHT:
    default:
        col_top = 0x0A0A0F; /* Void black */
        col_mid = 0x0F0E15; /* Deep space */
        col_bot = 0x1B1B2F; /* Dark indigo */
        break;
    }

    /* ── Scanline gradient ───────────────────────────────────────────────*/
    uint8_t *row_ptr = (uint8_t *)pixels;
    for (uint32_t y = 0; y < h; y++) {
        uint32_t *row = (uint32_t *)(void *)row_ptr;
        uint32_t t = (y * 255) / (h > 1 ? h - 1 : 1);
        uint32_t col;
        if (t < 128) {
            col = blend_colour(col_top, col_mid, t * 2);
        } else {
            col = blend_colour(col_mid, col_bot, (t - 128) * 2);
        }

        /* Subtle horizontal nebula wave — varies slowly with time */
        uint32_t wave = (uint32_t)((y + tick / 4) % h);
        uint32_t nebula_amt = 0;
        if (ws->tod == WALLPAPER_NIGHT || ws->tod == WALLPAPER_MORNING) {
            /* Add faint cyan tint at wave band */
            uint32_t band = h / 3;
            if (wave >= band && wave < band + 20) {
                nebula_amt = (wave - band < 10)
                    ? (wave - band) * 3
                    : (20 - (wave - band)) * 3;
            }
        }

        if (nebula_amt > 0) {
            uint32_t nr = ((col >> 16) & 0xFF);
            uint32_t ng = ((col >>  8) & 0xFF);
            uint32_t nb =  (col        & 0xFF);
            /* Shift toward cyan #00F0FF */
            nr = (nr * (255 - nebula_amt) + 0   * nebula_amt) / 255;
            ng = (ng * (255 - nebula_amt) + 120 * nebula_amt) / 255;
            nb = (nb * (255 - nebula_amt) + 180 * nebula_amt) / 255;
            col = (nr << 16) | (ng << 8) | nb;
        }

        for (uint32_t x = 0; x < w; x++) {
            row[x] = col;
        }

        row_ptr += stride;
    }

    /* ── Stars (only for Night and Morning) ──────────────────────────────*/
    if (ws->tod == WALLPAPER_NIGHT || ws->tod == WALLPAPER_MORNING) {
        for (int i = 0; i < MAX_STARS; i++) {
            uint32_t sx = g_stars[i].x % w;
            uint32_t sy = g_stars[i].y % h;
            /* Twinkle: vary brightness with a slow sine approximation */
            uint32_t phase = (tick / 2 + g_stars[i].twinkle) & 0x3F;
            uint32_t tw = (phase < 32) ? phase * 4 : (64 - phase) * 4;
            uint32_t bri = (uint32_t)g_stars[i].brightness;
            /* Scale brightness by twinkle */
            bri = (bri * (128 + tw / 2)) / 255;
            if (bri > 255) bri = 255;

            uint32_t *row = (uint32_t *)(void *)((uint8_t *)pixels + sy * stride);
            row[sx] = (bri << 16) | (bri << 8) | bri;
        }
    }

    /* ── Luna Island floating moon (night only) ──────────────────────────*/
    if (ws->tod == WALLPAPER_NIGHT) {
        /* Draw a small glowing circle for the moon */
        int mx = (int)(w * 3 / 4);
        int my = (int)(h / 6);
        int mr = 18;
        for (int dy = -mr; dy <= mr; dy++) {
            int ry = my + dy;
            if (ry < 0 || ry >= (int)h) continue;
            uint32_t *row = (uint32_t *)(void *)((uint8_t *)pixels + (uint32_t)ry * stride);
            for (int dx = -mr; dx <= mr; dx++) {
                int rx = mx + dx;
                if (rx < 0 || rx >= (int)w) continue;
                int dist2 = dx * dx + dy * dy;
                if (dist2 <= mr * mr) {
                    /* Core: near-white with slight yellow tint */
                    uint32_t core = (dist2 <= (mr - 4) * (mr - 4))
                        ? 0xFFF8DCu   /* Cornsilk */
                        : 0xC8B860u;  /* Pale gold rim */
                    row[rx] = core;
                } else if (dist2 <= (mr + 8) * (mr + 8)) {
                    /* Glow halo — blend cyan into background */
                    uint32_t glo_t = (uint32_t)((dist2 - mr * mr) * 255) /
                                     ((mr + 8) * (mr + 8) - mr * mr);
                    uint32_t bg = row[rx];
                    row[rx] = blend_colour(0x8080C0u, bg, (uint8_t)glo_t);
                }
            }
        }
    }
}
