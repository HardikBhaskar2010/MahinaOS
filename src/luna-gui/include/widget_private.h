/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef LUNAGUI_WIDGET_PRIVATE_H
#define LUNAGUI_WIDGET_PRIVATE_H

#include "lunagui.h"

struct lgui_widget_t {
    int type; /* 1=Label, 2=Button, 3=VBox, 4=HBox, 5=Scroll, 6=Canvas, 7=TextField */
    int x, y, width, height;
    char text[512];
    int text_len;
    
    int scroll_offset_x;
    int scroll_offset_y;

    lgui_button_click_cb  on_click;
    lgui_key_cb           on_key;
    lgui_canvas_render_cb canvas_render;
    lgui_textfield_change_cb on_text_change;
    void *user_data;
    
    lgui_widget_t *children[32];
    int child_count;

    /* TextField-specific state */
    int cursor_pos;
    int view_offset;
    bool focused;
    char placeholder[128];
    bool password_mode;
    int max_length;
};

#endif
