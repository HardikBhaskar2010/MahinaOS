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
    int type; /* 1=Label, 2=Button, 3=VBox, 4=HBox */
    int x, y, width, height;
    char text[256];
    lgui_button_click_cb on_click;
    void *user_data;
    
    lgui_widget_t *children[16];
    int child_count;
};

#endif
