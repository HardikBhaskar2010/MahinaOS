/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "lunagui.h"
#include <string.h>

struct lgui_canvas_t {
    uint32_t *pixels;
    uint32_t width;
    uint32_t height;
    uint32_t stride;
};

void lgui_canvas_fill_rect(lgui_canvas_t *canvas, int x, int y, int w, int h, uint32_t color) {
    if (!canvas || !canvas->pixels) return;
    
    if (x < 0) { w += x; x = 0; }
    if (y < 0) { h += y; y = 0; }
    if (x + w > (int)canvas->width) w = canvas->width - x;
    if (y + h > (int)canvas->height) h = canvas->height - y;
    
    if (w <= 0 || h <= 0) return;
    
    for (int cy = y; cy < y + h; cy++) {
        uint32_t *row = (uint32_t *)((uint8_t *)canvas->pixels + cy * canvas->stride);
        for (int cx = x; cx < x + w; cx++) {
            row[cx] = color;
        }
    }
}

lgui_canvas_t *lgui_canvas_create(void *pixels, uint32_t width, uint32_t height, uint32_t stride) {
    lgui_canvas_t *c = malloc(sizeof(lgui_canvas_t));
    if (c) {
        c->pixels = pixels;
        c->width = width;
        c->height = height;
        c->stride = stride;
    }
    return c;
}

void lgui_canvas_destroy(lgui_canvas_t *canvas) {
    free(canvas);
}

void lgui_canvas_draw_rect_outline(lgui_canvas_t *canvas,
                                    int x, int y, int w, int h, uint32_t color) {
    if (!canvas || w <= 0 || h <= 0) return;
    /* Top edge */
    lgui_canvas_fill_rect(canvas, x, y, w, 1, color);
    /* Bottom edge */
    lgui_canvas_fill_rect(canvas, x, y + h - 1, w, 1, color);
    /* Left edge */
    lgui_canvas_fill_rect(canvas, x, y, 1, h, color);
    /* Right edge */
    lgui_canvas_fill_rect(canvas, x + w - 1, y, 1, h, color);
}

extern void lgui_font_draw_text(void *pixels, int x, int y, uint32_t stride, const char *text, uint32_t color);

void lgui_canvas_draw_text(lgui_canvas_t *canvas, int x, int y, const char *text, uint32_t color) {
    if (!canvas || !canvas->pixels) return;
    lgui_font_draw_text(canvas->pixels, x, y, canvas->stride, text, color);
}
