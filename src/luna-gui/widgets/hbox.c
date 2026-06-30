/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "lunagui.h"
#include <stdlib.h>

#include "widget_private.h"

lgui_widget_t *lgui_hbox_create(void) {
    lgui_widget_t *box = calloc(1, sizeof(lgui_widget_t));
    if (box) {
        box->type = 4; /* HBOX */
    }
    return box;
}
