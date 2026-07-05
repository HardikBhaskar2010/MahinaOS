/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

#include "presence.h"
#include <string.h>
#include <ctype.h>

static ai_mode_t g_current_mode = MODE_AMBIENT;

const char *ai_mode_to_string(ai_mode_t mode) {
    switch (mode) {
        case MODE_AMBIENT:  return "AMBIENT";
        case MODE_DEVSHELL: return "DEVSHELL";
        case MODE_FOCUS:    return "FOCUS";
        case MODE_STUDY:    return "STUDY";
        case MODE_CREATIVE: return "CREATIVE";
        default:            return "AMBIENT";
    }
}

ai_mode_t ai_mode_from_string(const char *str) {
    if (!str) return MODE_AMBIENT;
    if (strcmp(str, "AMBIENT") == 0)  return MODE_AMBIENT;
    if (strcmp(str, "DEVSHELL") == 0) return MODE_DEVSHELL;
    if (strcmp(str, "FOCUS") == 0)    return MODE_FOCUS;
    if (strcmp(str, "STUDY") == 0)    return MODE_STUDY;
    if (strcmp(str, "CREATIVE") == 0) return MODE_CREATIVE;
    return MODE_AMBIENT;
}

void presence_init(void) {
    g_current_mode = MODE_AMBIENT;
}

void presence_update_active_app(const char *app_name) {
    if (!app_name || app_name[0] == '\0') {
        g_current_mode = MODE_AMBIENT;
        return;
    }

    /* Convert to lowercase for comparison */
    char buf[128];
    size_t i = 0;
    for (; app_name[i] && i < sizeof(buf) - 1; i++) {
        buf[i] = (char)tolower((unsigned char)app_name[i]);
    }
    buf[i] = '\0';

    if (strstr(buf, "terminal") || strstr(buf, "shell") || strstr(buf, "code")) {
        g_current_mode = MODE_DEVSHELL;
    } else if (strstr(buf, "tasks") || strstr(buf, "calc") || strstr(buf, "math")) {
        g_current_mode = MODE_FOCUS;
    } else if (strstr(buf, "text") || strstr(buf, "writer") || strstr(buf, "draw") || strstr(buf, "paint")) {
        g_current_mode = MODE_CREATIVE;
    } else if (strstr(buf, "browser") || strstr(buf, "study") || strstr(buf, "web")) {
        g_current_mode = MODE_STUDY;
    } else {
        g_current_mode = MODE_AMBIENT;
    }
}

ai_mode_t presence_get_current_mode(void) {
    return g_current_mode;
}
