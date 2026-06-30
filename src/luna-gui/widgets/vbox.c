/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "lunagui.h"
#include <stdlib.h>

#include "widget_private.h"

lgui_widget_t *lgui_vbox_create(void) {
    lgui_widget_t *box = calloc(1, sizeof(lgui_widget_t));
    if (box) {
        box->type = 3; /* VBOX */
    }
    return box;
}

void lgui_box_add_child(lgui_widget_t *box, lgui_widget_t *child) {
    if (box && child && box->child_count < 16) {
        box->children[box->child_count++] = child;
    }
}
