/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef MAHINA_LUNA_INIT_SPLASH_H
#define MAHINA_LUNA_INIT_SPLASH_H

void splash_start(void);
void splash_update(const char *msg, int percent);
void splash_stop(void);

#endif /* MAHINA_LUNA_INIT_SPLASH_H */
