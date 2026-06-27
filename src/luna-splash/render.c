/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "render.h"
#include "font8x16.h"

#include <fcntl.h>
#include <linux/fb.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <unistd.h>

static int fb_fd = -1;
static uint32_t *fb_mem = NULL;
static struct fb_var_screeninfo vinfo;
static struct fb_fix_screeninfo finfo;

int screen_w = 0;
int screen_h = 0;
static int line_length = 0;

int render_init(void) {
    fb_fd = open("/dev/fb0", O_RDWR | O_CLOEXEC);
    if (fb_fd < 0) return -1;

    if (ioctl(fb_fd, FBIOGET_FSCREENINFO, &finfo) < 0) {
        close(fb_fd);
        return -1;
    }

    if (ioctl(fb_fd, FBIOGET_VSCREENINFO, &vinfo) < 0) {
        close(fb_fd);
        return -1;
    }

    screen_w = (int)vinfo.xres;
    screen_h = (int)vinfo.yres;
    line_length = (int)finfo.line_length;

    size_t screensize = (size_t)vinfo.yres_virtual * (size_t)finfo.line_length;

    fb_mem = (uint32_t *)mmap(NULL, screensize, PROT_READ | PROT_WRITE, MAP_SHARED, fb_fd, (off_t)0);
    if (fb_mem == MAP_FAILED) {
        close(fb_fd);
        return -1;
    }

    return 0;
}

void render_cleanup(void) {
    if (fb_mem != MAP_FAILED && fb_mem != NULL) {
        size_t screensize = (size_t)vinfo.yres_virtual * (size_t)finfo.line_length;
        munmap(fb_mem, screensize);
        fb_mem = NULL;
    }
    if (fb_fd >= 0) {
        close(fb_fd);
        fb_fd = -1;
    }
}

static inline void put_pixel(int x, int y, uint32_t color) {
    if (x < 0 || x >= screen_w || y < 0 || y >= screen_h) return;
    int offset = (y * line_length / 4) + x;
    fb_mem[offset] = color;
}

void render_clear(uint32_t color) {
    for (int y = 0; y < screen_h; y++) {
        for (int x = 0; x < screen_w; x++) {
            put_pixel(x, y, color);
        }
    }
}

void render_text(int x, int y, const char *text, uint32_t color) {
    int cur_x = x;
    int cur_y = y;
    
    while (*text) {
        unsigned char c = (unsigned char)*text;
        if (c == '\n') {
            cur_x = x;
            cur_y += 16;
        } else {
            for (int cy = 0; cy < 16; cy++) {
                uint8_t row = font8x16[c][cy];
                for (int cx = 0; cx < 8; cx++) {
                    if (row & (1 << (7 - cx))) {
                        put_pixel(cur_x + cx, cur_y + cy, color);
                    }
                }
            }
            cur_x += 8;
        }
        text++;
    }
}

void render_text_centered(int y, const char *text, uint32_t color) {
    int len = 0;
    while (text[len]) len++;
    int total_width = len * 8;
    int x = (screen_w - total_width) / 2;
    render_text(x, y, text, color);
}

void render_logo(void) {
    int center_y = screen_h / 2 - 60;
    
    const char *logo[] = {
        "  ███╗   ███╗ █████╗ ██╗  ██╗██╗███╗   ██╗ █████╗ ",
        "  ████╗ ████║██╔══██╗██║  ██║██║████╗  ██║██╔══██╗",
        "  ██╔████╔██║███████║███████║██║██╔██╗ ██║███████║",
        "  ██║╚██╔╝██║██╔══██║██╔══██║██║██║╚██╗██║██╔══██║",
        "  ██║ ╚═╝ ██║██║  ██║██║  ██║██║██║ ╚████║██║  ██║",
        "  ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝",
        NULL
    };
    
    for (int i = 0; logo[i] != NULL; i++) {
        /* This isn't perfect since block characters aren't in our 8x16 font array.
         * For stage 0 bringup, we just print the text "MAHINA".
         */
    }
    
    /* Fallback to simple text logo */
    render_text_centered(center_y, "MAHINA", COLOR_BRAND);
    render_text_centered(center_y + 32, "Initializing System", COLOR_TEXT);
}

void render_progress(const char *msg, int percent) {
    int center_y = screen_h / 2 + 20;
    
    /* Clear the area first (simple brute force) */
    for (int y = center_y; y < center_y + 60; y++) {
        for (int x = 0; x < screen_w; x++) {
            put_pixel(x, y, COLOR_BG);
        }
    }
    
    render_text_centered(center_y, msg, COLOR_TEXT);
    
    /* Draw progress bar */
    int bar_w = 400;
    int bar_h = 10;
    int bar_x = (screen_w - bar_w) / 2;
    int bar_y = center_y + 30;
    
    for (int y = bar_y; y < bar_y + bar_h; y++) {
        for (int x = bar_x; x < bar_x + bar_w; x++) {
            put_pixel(x, y, COLOR_BAR_BG);
        }
    }
    
    int fill_w = (bar_w * percent) / 100;
    for (int y = bar_y; y < bar_y + bar_h; y++) {
        for (int x = bar_x; x < bar_x + fill_w; x++) {
            put_pixel(x, y, COLOR_BRAND);
        }
    }
}
