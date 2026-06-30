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

lgui_widget_t *lgui_label_create(const char *text) {
    lgui_widget_t *lbl = calloc(1, sizeof(lgui_widget_t));
    if (lbl) {
        lbl->type = 1; /* LABEL */
        if (text) strncpy(lbl->text, text, sizeof(lbl->text) - 1);
        lbl->width = 100;
        lbl->height = 20;
    }
    return lbl;
}

void lgui_label_set_text(lgui_widget_t *label, const char *text) {
    if (label && text) {
        strncpy(label->text, text, sizeof(label->text) - 1);
    }
}
