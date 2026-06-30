/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "luna-gui/include/lunagui.h"
#include "luna-gui/include/widget_private.h"

void lgui_widget_render(lgui_canvas_t *canvas, lgui_widget_t *w, int offset_x, int offset_y) {
    if (!w || !canvas) return;
    
    int cx = offset_x + w->x;
    int cy = offset_y + w->y;
    
    if (w->type == 1) { /* Label */
        lgui_canvas_draw_text(canvas, cx, cy, w->text, 0xFFFFFFFF);
    } else if (w->type == 2) { /* Button */
        lgui_canvas_fill_rect(canvas, cx, cy, w->width, w->height, 0xFF444444);
        lgui_canvas_draw_text(canvas, cx + 5, cy + 5, w->text, 0xFFFFFFFF);
    }
    
    /* Naive layout for MVP */
    if (w->type == 3) { /* VBox */
        int y_accum = 0;
        for (int i = 0; i < w->child_count; i++) {
            w->children[i]->x = 0;
            w->children[i]->y = y_accum;
            if (w->children[i]->width == 0) w->children[i]->width = w->width;
            if (w->children[i]->height == 0) w->children[i]->height = 30;
            y_accum += w->children[i]->height;
            lgui_widget_render(canvas, w->children[i], cx, cy);
        }
    } else if (w->type == 4) { /* HBox */
        int x_accum = 0;
        for (int i = 0; i < w->child_count; i++) {
            w->children[i]->x = x_accum;
            w->children[i]->y = 0;
            if (w->children[i]->width == 0) w->children[i]->width = 100;
            if (w->children[i]->height == 0) w->children[i]->height = w->height;
            x_accum += w->children[i]->width;
            lgui_widget_render(canvas, w->children[i], cx, cy);
        }
    } else {
        for (int i = 0; i < w->child_count; i++) {
            lgui_widget_render(canvas, w->children[i], cx, cy);
        }
    }
}
