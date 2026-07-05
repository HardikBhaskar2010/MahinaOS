/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

#ifndef LUNA_AI_PRESENCE_H
#define LUNA_AI_PRESENCE_H

typedef enum {
    MODE_AMBIENT   = 0,
    MODE_DEVSHELL  = 1,
    MODE_FOCUS     = 2,
    MODE_STUDY     = 3,
    MODE_CREATIVE  = 4
} ai_mode_t;

const char *ai_mode_to_string(ai_mode_t mode);
ai_mode_t ai_mode_from_string(const char *str);

/*
 * presence_init() — Initialise state machine.
 */
void presence_init(void);

/*
 * presence_update_active_app() — Update the current mode based on the active application.
 */
void presence_update_active_app(const char *app_name);

/*
 * presence_get_current_mode() — Return the current AI mode.
 */
ai_mode_t presence_get_current_mode(void);

#endif /* LUNA_AI_PRESENCE_H */
