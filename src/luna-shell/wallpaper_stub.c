#include <stddef.h>

struct wallpaper { int _unused; };

void wallpaper_init(struct wallpaper *wp, int w, int h);
void wallpaper_render(struct wallpaper *wp, unsigned char *px, unsigned int stride);
void wallpaper_tick(struct wallpaper *wp);
void wallpaper_cleanup(struct wallpaper *wp);

void wallpaper_init(struct wallpaper *wp, int w, int h) {
    (void)wp; (void)w; (void)h;
}

void wallpaper_render(struct wallpaper *wp, unsigned char *px, unsigned int stride) {
    (void)wp; (void)px; (void)stride;
}

void wallpaper_tick(struct wallpaper *wp) {
    (void)wp;
}

void wallpaper_cleanup(struct wallpaper *wp) {
    (void)wp;
}
