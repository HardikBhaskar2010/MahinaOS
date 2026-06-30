/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "lunagui.h"
#include <stdlib.h>
#include <string.h>

#include "widget_private.h"

lgui_widget_t *lgui_button_create(const char *text) {
    lgui_widget_t *btn = calloc(1, sizeof(lgui_widget_t));
    if (btn) {
        btn->type = 2; /* BUTTON */
        if (text) strncpy(btn->text, text, sizeof(btn->text) - 1);
        btn->width = 100;
        btn->height = 30;
    }
    return btn;
}

void lgui_button_set_on_click(lgui_widget_t *button, lgui_button_click_cb cb, void *user_data) {
    if (button) {
        button->on_click = cb;
        button->user_data = user_data;
    }
}
