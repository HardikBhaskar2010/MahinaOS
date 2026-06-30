/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef MAHINA_LGP_INPUT_MOUSE_H
#define MAHINA_LGP_INPUT_MOUSE_H

#include <stdbool.h>
#include <stdint.h>
#include "../main.h"

/*
 * lgp_mouse_init() — Open /dev/input/mice and set the display dimensions
 * used for cursor clamping. Must be called before lgp_mouse_pump().
 * screen_width / screen_height are taken from the live DRM mode.
 */
void lgp_mouse_init(uint32_t screen_width, uint32_t screen_height);

/*
 * lgp_mouse_pump() — Drain all pending PS/2 events from /dev/input/mice
 * and dispatch pointer-motion events to the relevant LGP clients.
 * Safe to call on every epoll wakeup; returns immediately if no events.
 */
void lgp_mouse_pump(lgp_compositor_state_t *state);

/*
 * lgp_mouse_get_fd() — Return the /dev/input/mice file descriptor so the
 * caller can add it to epoll.
 * Returns -1 if the device could not be opened.
 */
int lgp_mouse_get_fd(void);

#endif
