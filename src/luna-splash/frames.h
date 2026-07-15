/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * frames.h — PNG Frame Sequence Loader for luna-splash
 *
 * Loads pre-extracted PNG frames from a directory into the framebuffer.
 * Frames are centered on the screen with a black background (letterboxing).
 *
 * The video is displayed at its native resolution, not scaled up, so a 1:1
 * square video (e.g. 1080x1080) is centered with black bars on all sides.
 */

#ifndef MAHINA_SPLASH_FRAMES_H
#define MAHINA_SPLASH_FRAMES_H

#include <stdbool.h>
#include <stdint.h>

/*
 * frames_init() — Scan a directory for frame%04d.png files and read meta.txt.
 *
 * Returns the number of frames found, or 0 if the directory is empty/missing.
 * Must be called before any other frames_* function.
 */
int frames_init(const char *dir);

/*
 * frames_count() — Return total number of loaded frames.
 */
int frames_count(void);

/*
 * frames_get_native_size() — Fill *w/*h with the video's native pixel size.
 * Filled from meta.txt if present, otherwise from the first PNG.
 */
void frames_get_native_size(int *w, int *h);

/*
 * frames_get_fps() — Return the video's native FPS (from meta.txt, default 30).
 */
int frames_get_fps(void);

/*
 * frames_load_next() — Decode the next frame and composite it centred on the
 * framebuffer.  The region outside the frame is left black (cleared once at
 * init time).  Advances the internal frame index; wraps around automatically
 * for seamless looping.
 *
 *   fb_mem    — pointer to the mmap'd framebuffer base
 *   fb_w      — framebuffer width  in pixels
 *   fb_h      — framebuffer height in pixels
 *   fb_stride — framebuffer line length in pixels (may differ from fb_w)
 *
 * Returns true on success, false if the frame could not be decoded (skipped).
 */
bool frames_load_next(uint32_t *fb_mem,
                      int fb_w, int fb_h, int fb_stride);

/*
 * frames_cleanup() — Free all allocated memory.
 */
void frames_cleanup(void);

#endif /* MAHINA_SPLASH_FRAMES_H */
