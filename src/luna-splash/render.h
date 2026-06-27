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
void render_clear(uint32_t color);
void render_text(int x, int y, const char *text, uint32_t color);
void render_text_centered(int y, const char *text, uint32_t color);
void render_logo(void);
void render_progress(const char *msg, int percent);

#endif /* MAHINA_SPLASH_RENDER_H */
