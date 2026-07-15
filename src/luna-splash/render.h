/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef MAHINA_SPLASH_RENDER_H
#define MAHINA_SPLASH_RENDER_H

#include <stdint.h>
#include <stdbool.h>

/* Colors (XRGB8888) */
#define COLOR_BG    0x00000000 /* Black */
#define COLOR_TEXT  0x00FFFFFF /* White */
#define COLOR_BRAND 0x0000FFFF /* Cyan */
#define COLOR_BAR_BG 0x00333333 /* Dark Gray */

/* Screen constraints */
extern int screen_w;
extern int screen_h;

int render_init(void);
void render_cleanup(void);
void render_fade_out(void);
void render_clear(uint32_t color);
void render_text(int x, int y, const char *text, uint32_t color);
void render_text_centered(int y, const char *text, uint32_t color);
void render_logo(void);
void render_progress(const char *msg, int percent);

/*
 * render_overlay_progress() — Draw a progress bar and status message ON TOP
 * of the current framebuffer contents without clearing the frame underneath.
 *
 * Used in video (frame-sequence) mode so that each decoded video frame is
 * preserved and the boot-progress info is composited over it as a dark
 * semi-transparent band at the bottom of the screen.
 */
void render_overlay_progress(const char *msg, int percent);

/*
 * Accessors for the mmap'd framebuffer memory and line stride (in pixels).
 * Used by frames.c to blit decoded PNG frames directly.
 */
void    *render_get_fb_mem(void);
int      render_get_stride(void);   /* line length in pixels */

#endif /* MAHINA_SPLASH_RENDER_H */
