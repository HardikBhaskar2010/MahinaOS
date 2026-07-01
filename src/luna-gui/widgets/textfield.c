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

lgui_widget_t *lgui_textfield_create(const char *placeholder) {
    lgui_widget_t *tf = calloc(1, sizeof(lgui_widget_t));
    if (tf) {
        tf->type = 7; /* TEXTFIELD */
        tf->width = 200;
        tf->height = 32;
        tf->cursor_pos = 0;
        tf->view_offset = 0;
        tf->focused = false;
        tf->max_length = 511;
        tf->text_len = 0;
        tf->text[0] = '\0';
        if (placeholder) strncpy(tf->placeholder, placeholder, sizeof(tf->placeholder) - 1);
    }
    return tf;
}

void lgui_textfield_set_text(lgui_widget_t *textfield, const char *text) {
    if (!textfield || !text) return;
    strncpy(textfield->text, text, sizeof(textfield->text) - 1);
    textfield->text_len = (int)strlen(textfield->text);
    if (textfield->cursor_pos > textfield->text_len)
        textfield->cursor_pos = textfield->text_len;
}

const char *lgui_textfield_get_text(lgui_widget_t *textfield) {
    if (!textfield) return "";
    return textfield->text;
}

void lgui_textfield_set_on_change(lgui_widget_t *textfield, lgui_textfield_change_cb cb, void *user_data) {
    if (!textfield) return;
    textfield->on_text_change = cb;
    textfield->user_data = user_data;
}

void lgui_textfield_set_password(lgui_widget_t *textfield, bool password) {
    if (textfield) textfield->password_mode = password;
}

void lgui_textfield_set_max_length(lgui_widget_t *textfield, int max_len) {
    if (textfield && max_len > 0 && max_len < 512)
        textfield->max_length = max_len;
}
