/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * frames.c — PNG Frame Sequence Loader
 *
 * Design goals:
 *   • No external dependencies beyond stb_image.h (pure C, zero libs)
 *   • 1:1 video displayed CENTRED on the framebuffer at native resolution —
 *     not scaled up.  Black regions fill the letterbox areas.
 *   • Colors preserved exactly: stb_image loads PNG RGB → we write to
 *     framebuffer as XRGB8888 with the correct R/G/B byte order.
 *   • Graceful skip: if an individual frame PNG is missing or corrupt,
 *     the frame is skipped and the animation continues.
 *   • Automatic looping: frame index wraps around at end of sequence.
 */

#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include "frames.h"
#include "stb_image.h"

#include <dirent.h>
#include <errno.h>
#include <limits.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ── Internal state ─────────────────────────────────────────────────────── */

#define MAX_FRAMES 512

static char   s_frame_paths[MAX_FRAMES][PATH_MAX];
static int    s_frame_count  = 0;
static int    s_frame_index  = 0;     /* next frame to display */
static int    s_native_w     = 0;
static int    s_native_h     = 0;
static int    s_fps          = 30;
static bool   s_bg_cleared   = false; /* black letterbox painted once */

/* ── Helpers ─────────────────────────────────────────────────────────────── */

/* Comparison function for qsort — sort frame paths lexicographically so that
 * frame0001.png < frame0002.png < ... regardless of readdir order. */
static int path_cmp(const void *a, const void *b) {
    return strcmp((const char *)a, (const char *)b);
}

/* Read optional meta.txt from the frame directory.
 * Format (one key=value per line):
 *   frames=144
 *   width=1080
 *   height=1080
 *   fps=30
 */
static void read_meta(const char *dir) {
    char meta_path[PATH_MAX];
    snprintf(meta_path, sizeof(meta_path), "%s/meta.txt", dir);

    FILE *f = fopen(meta_path, "r");
    if (!f) return;

    char line[128];
    while (fgets(line, sizeof(line), f)) {
        int  val = 0;
        char key[64] = {0};
        if (sscanf(line, "%63[^=]=%d", key, &val) == 2) {
            if (strcmp(key, "width")  == 0) s_native_w = val;
            if (strcmp(key, "height") == 0) s_native_h = val;
            if (strcmp(key, "fps")    == 0) s_fps       = val;
        }
    }
    fclose(f);
}

/* ── Public API ──────────────────────────────────────────────────────────── */

int frames_init(const char *dir) {
    s_frame_count = 0;
    s_frame_index = 0;
    s_bg_cleared  = false;

    if (!dir) return 0;

    /* Read optional metadata first */
    read_meta(dir);

    /* Scan directory for frame*.png files */
    DIR *d = opendir(dir);
    if (!d) return 0;

    struct dirent *entry;
    while ((entry = readdir(d)) != NULL && s_frame_count < MAX_FRAMES) {
        const char *name = entry->d_name;
        /* Match frame followed by digits and .png */
        if (strncmp(name, "frame", 5) != 0) continue;
        const char *ext = strrchr(name, '.');
        if (!ext || strcmp(ext, ".png") != 0) continue;

        snprintf(s_frame_paths[s_frame_count], PATH_MAX,
                 "%s/%s", dir, name);
        s_frame_count++;
    }
    closedir(d);

    if (s_frame_count == 0) return 0;

    /* Sort so frames play in correct order */
    qsort(s_frame_paths, (size_t)s_frame_count,
          sizeof(s_frame_paths[0]), path_cmp);

    /* If meta.txt didn't give us dimensions, probe the first frame */
    if (s_native_w == 0 || s_native_h == 0) {
        int w, h, ch;
        unsigned char *tmp = stbi_load(s_frame_paths[0], &w, &h, &ch, 3);
        if (tmp) {
            s_native_w = w;
            s_native_h = h;
            stbi_image_free(tmp);
        }
    }

    return s_frame_count;
}

int frames_count(void) {
    return s_frame_count;
}

void frames_get_native_size(int *w, int *h) {
    if (w) *w = s_native_w;
    if (h) *h = s_native_h;
}

int frames_get_fps(void) {
    return s_fps > 0 ? s_fps : 30;
}

bool frames_load_next(uint32_t *fb_mem,
                      int fb_w, int fb_h, int fb_stride) {
    if (s_frame_count == 0 || !fb_mem) return false;

    /* ── 1. Decode PNG ───────────────────────────────────────────────────── */
    const char *path = s_frame_paths[s_frame_index];

    int img_w, img_h, channels;
    /* Request 3 channels (RGB) — we manually build the XRGB8888 uint32_t so
     * there is no colour-space alteration at all.  stb_image returns bytes in
     * R, G, B order for a 3-channel load of a standard PNG. */
    unsigned char *pixels = stbi_load(path, &img_w, &img_h, &channels, 3);

    /* Advance index and wrap around for looping */
    s_frame_index = (s_frame_index + 1) % s_frame_count;

    if (!pixels) {
        /* Corrupt / missing frame — skip silently */
        return false;
    }

    /* ── 2. Black letterbox (painted once; video frame does not cover it) ── */
    if (!s_bg_cleared) {
        memset(fb_mem, 0,
               (size_t)fb_h * (size_t)fb_stride * sizeof(uint32_t));
        s_bg_cleared = true;
    }

    /* ── 3. Calculate centred placement ─────────────────────────────────── */
    /*
     * The video is a 1:1 square.  We do NOT scale it up — we display it at
     * its native pixel size, centered on the framebuffer.  If the framebuffer
     * is smaller than the video in either dimension we clamp to the fb size
     * so we never write out of bounds.
     */
    int draw_w = img_w < fb_w ? img_w : fb_w;
    int draw_h = img_h < fb_h ? img_h : fb_h;

    int x_off = (fb_w - draw_w) / 2;   /* left black bar */
    int y_off = (fb_h - draw_h) / 2;   /* top  black bar */

    /* ── 4. Blit: RGB bytes → XRGB8888 uint32_t ─────────────────────────── */
    /*
     * Framebuffer pixel format (from render.c): 0x00RRGGBB stored as a
     * uint32_t.  In little-endian memory this is [B][G][R][0] in byte order,
     * which is what the kernel framebuffer driver (XRGB8888) expects.
     *
     * stb_image returns bytes in [R][G][B] order for a 3-channel load.
     * So: fb_pixel = (R << 16) | (G << 8) | B
     *
     * No colour alteration — this is a 1-to-1 mapping of the original values.
     */
    for (int row = 0; row < draw_h; row++) {
        const unsigned char *src_row =
            pixels + (size_t)row * (size_t)img_w * 3;
        uint32_t *dst_row =
            fb_mem + (size_t)(y_off + row) * (size_t)fb_stride + x_off;

        for (int col = 0; col < draw_w; col++) {
            unsigned char r = src_row[col * 3 + 0];
            unsigned char g = src_row[col * 3 + 1];
            unsigned char b = src_row[col * 3 + 2];
            dst_row[col] = ((uint32_t)r << 16)
                         | ((uint32_t)g <<  8)
                         |  (uint32_t)b;
        }
    }

    stbi_image_free(pixels);
    return true;
}

void frames_cleanup(void) {
    s_frame_count = 0;
    s_frame_index = 0;
    s_native_w    = 0;
    s_native_h    = 0;
    s_bg_cleared  = false;
}
