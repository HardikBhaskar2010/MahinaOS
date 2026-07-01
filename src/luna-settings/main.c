/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * luna-settings — System Settings application for MahinaOS.
 * Backend (C): reads /sys, /proc, /etc for real system info.
 * UI (C/LunaGUI): multi-page settings with Display, Wallpaper, Network,
 * Audio, Users & Accounts, and About System.
 */

#include "lunagui.h"
#include "window_private.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/utsname.h>
#include <sys/statvfs.h>
#include <dirent.h>

#define WIN_W 480
#define WIN_H 520

#define COL_BG        0xFF12121Cu
#define COL_SURFACE   0xFF1E1E28u
#define COL_ACCENT    0xFF00D4FFu
#define COL_TEXT      0xFFEEEEF4u
#define COL_TEXT_DIM  0xFF9898AAu
#define COL_GREEN     0xFF00FF88u
#define COL_BORDER    0xFF303044u
#define COL_BTN       0xFF262636u
#define COL_BTN_HOVER 0xFF2E2E42u

enum { PAGE_HOME, PAGE_DISPLAY, PAGE_WALLPAPER, PAGE_NETWORK,
       PAGE_AUDIO, PAGE_USERS, PAGE_ABOUT };

#define MAX_CLICK_ZONES 20
static struct { int x, y, w, h; int action; } g_zones[MAX_CLICK_ZONES];
static int g_zone_count = 0;

static int g_page = PAGE_HOME;

/* ── Backend helpers ────────────────────────────────────────────────────────── */

static void read_file_line(const char *path, char *buf, size_t len) {
    FILE *f = fopen(path, "r");
    if (!f) { buf[0] = '\0'; return; }
    if (fgets(buf, (int)len, f)) {
        size_t l = strlen(buf);
        while (l > 0 && (buf[l-1]=='\n'||buf[l-1]=='\r')) buf[--l]='\0';
    } else buf[0]='\0';
    fclose(f);
}

static void get_kernel(char *b, size_t l) {
    struct utsname u;
    if (uname(&u)==0) snprintf(b,l,"%s %s",u.sysname,u.release);
    else read_file_line("/proc/version",b,l);
}
static void get_cpu(char *b, size_t l) {
    FILE *f=fopen("/proc/cpuinfo","r"); if(!f){b[0]=0;return;}
    char line[256];
    while(fgets(line,sizeof(line),f)){
        if(strncmp(line,"model name",10)==0){
            char *p=strchr(line,':');
            if(p){p+=2; size_t n=strlen(p);
            while(n>0&&(p[n-1]=='\n'||p[n-1]=='\r'))p[--n]='\0';
            strncpy(b,p,l-1);b[l-1]='\0';fclose(f);return;}
        }
    }
    fclose(f);strncpy(b,"Unknown",l);
}
static void get_ram(char *b, size_t l) {
    FILE *f=fopen("/proc/meminfo","r"); if(!f){b[0]=0;return;}
    char line[256];
    while(fgets(line,sizeof(line),f)){
        unsigned long kb;
        if(sscanf(line,"MemTotal: %lu kB",&kb)==1){snprintf(b,l,"%lu MB",kb/1024);fclose(f);return;}
    }
    fclose(f);strncpy(b,"Unknown",l);
}
static void get_display(char *b, size_t l) {
    DIR *d=opendir("/sys/class/drm");
    if(d){struct dirent *de;
    while((de=readdir(d))){if(de->d_name[0]=='.')continue;
        char path[256];snprintf(path,sizeof(path),"/sys/class/drm/%s/status",de->d_name);
        char st[32];read_file_line(path,st,sizeof(st));
        if(strcmp(st,"connected")==0){
            snprintf(path,sizeof(path),"/sys/class/drm/%s/modes",de->d_name);
            FILE *f=fopen(path,"r");
            if(f){char mode[128];if(fgets(mode,sizeof(mode),f)){
                size_t n=strlen(mode);while(n>0&&(mode[n-1]=='\n'||mode[n-1]=='\r'))mode[--n]='\0';
                snprintf(b,l,"%s @ %s",de->d_name,mode);fclose(f);closedir(d);return;}
            fclose(f);}
        }
    }closedir(d);}
    strncpy(b,"1920x1080 (QEMU VirtIO GPU)",l);
}
static void get_hostname_(char *b, size_t l) { if(gethostname(b,l)!=0)strncpy(b,"mahina",l); }
static void get_disk(char *b, size_t l) {
    struct statvfs st;
    if(statvfs("/",&st)==0){unsigned long t=(unsigned long)(st.f_blocks*st.f_frsize);
    unsigned long fr=(unsigned long)(st.f_bfree*st.f_frsize);unsigned long u=t-fr;
    snprintf(b,l,"%lu GB / %lu GB (%lu%%)",u/(1024*1024*1024),t/(1024*1024*1024),
    t>0?u*100/t:0);}else strncpy(b,"Unknown",l);
}
static void get_network(char *b, size_t l) {
    FILE *f=fopen("/proc/net/dev","r");if(!f){b[0]=0;return;}
    char line[256];int off=0;
    fgets(line,sizeof(line),f);fgets(line,sizeof(line),f);
    while(fgets(line,sizeof(line),f)&&off<(int)l-64){
        char iface[32];unsigned long rx,tx;
        if(sscanf(line," %31[^:]: %lu %*u %*u %*u %*u %*u %*u %*u %lu",iface,&rx,&tx)>=3){
            if(strcmp(iface,"lo")==0)continue;
            off+=snprintf(b+off,(size_t)(l-off),"%s: RX %.1fMB TX %.1fMB",iface,rx/1048576.0,tx/1048576.0);
        }
    }fclose(f);if(off==0)strncpy(b,"No active interfaces",l);
}
static void get_users(char *b, size_t l) {
    FILE *f=fopen("/etc/passwd","r");if(!f){b[0]=0;return;}
    int off=0;char line[256];
    while(fgets(line,sizeof(line),f)&&off<(int)l-64){
        char name[64];int uid;
        if(sscanf(line,"%63[^:]:*:%d",name,&uid)==2){
            if(uid<1000&&uid!=0)continue;
            off+=snprintf(b+off,(size_t)(l-off),"%s (uid=%d) ",name,uid);
        }
    }fclose(f);if(off==0)strncpy(b,"No users",l);
}
static int get_vol(void) {
    char buf[128];FILE *p=popen("amixer get Master 2>/dev/null","r");
    if(!p)return -1;int v=-1;
    while(fgets(buf,sizeof(buf),p)){int vv;
    if(sscanf(buf,"  Front Playback: Playback %d",&vv)==1||sscanf(buf,"  Playback %d",&vv)==1)v=vv;}
    pclose(p);return v;
}
static bool is_muted_(void) {
    char buf[128];FILE *p=popen("amixer get Master 2>/dev/null","r");
    if(!p)return false;
    while(fgets(buf,sizeof(buf),p)){if(strstr(buf,"[off]")){pclose(p);return true;}}
    pclose(p);return false;
}

/* ── Canvas render helpers ──────────────────────────────────────────────────── */

static void draw_bg(lgui_canvas_t *c) {
    lgui_canvas_fill_rect(c, 0, 0, WIN_W, WIN_H, COL_BG);
}
static void draw_btn(lgui_canvas_t *c, int x, int y, int w, int h, const char *text) {
    lgui_canvas_fill_rect(c, x, y, w, h, COL_BTN);
    lgui_canvas_draw_rect_outline(c, x, y, w, h, COL_BORDER);
    lgui_canvas_draw_text(c, x+10, y+(h-16)/2, text, COL_TEXT);
}
static void draw_label(lgui_canvas_t *c, int x, int y, const char *text) {
    lgui_canvas_draw_text(c, x, y, text, COL_TEXT);
}
static void draw_label_dim(lgui_canvas_t *c, int x, int y, const char *text) {
    lgui_canvas_draw_text(c, x, y, text, COL_TEXT_DIM);
}
static void draw_accent(lgui_canvas_t *c, int x, int y, const char *text) {
    lgui_canvas_draw_text(c, x, y, text, COL_ACCENT);
}
static void add_zone(int x, int y, int w, int h, int action) {
    if(g_zone_count<MAX_CLICK_ZONES){
        g_zones[g_zone_count].x = x;
        g_zones[g_zone_count].y = y;
        g_zones[g_zone_count].w = w;
        g_zones[g_zone_count].h = h;
        g_zones[g_zone_count].action = action;
        g_zone_count++;
    }
}

/* ── Page renderers ─────────────────────────────────────────────────────────── */

static void render_home(lgui_canvas_t *c) {
    draw_label(c, 20, 16, "System Settings");
    lgui_canvas_fill_rect(c, 16, 36, WIN_W-32, 1, COL_ACCENT);
    int y=52;
    const char *items[]={"Display","Wallpaper","Network","Audio","Users & Accounts","About System"};
    int actions[]={PAGE_DISPLAY,PAGE_WALLPAPER,PAGE_NETWORK,PAGE_AUDIO,PAGE_USERS,PAGE_ABOUT};
    for(int i=0;i<6;i++){
        draw_btn(c,16,y,WIN_W-32,36,items[i]);
        add_zone(16,y,WIN_W-32,36,actions[i]);
        y+=44;
    }
}

static void render_display(lgui_canvas_t *c) {
    draw_label(c,20,16,"Display");
    draw_accent(c,20,16,"< Back");add_zone(20,16,60,16,PAGE_HOME);
    lgui_canvas_fill_rect(c,16,36,WIN_W-32,1,COL_ACCENT);
    int y=52;
    draw_label(c,20,y,"Current Resolution:");y+=20;
    char info[256];get_display(info,sizeof(info));
    draw_accent(c,20,y,info);y+=28;
    draw_label(c,20,y,"Refresh Rate:");y+=20;
    draw_label_dim(c,20,y,"60 Hz (QEMU default)");y+=28;
    draw_label(c,20,y,"Color Depth:");y+=20;
    draw_label_dim(c,20,y,"32-bit (XRGB8888)");y+=28;
    draw_label(c,20,y,"GPU:");y+=20;
    draw_label_dim(c,20,y,"VirtIO GPU (QEMU)");y+=28;
    draw_label(c,20,y,"Compositor: lgpmpositor (LGP Protocol)");y+=20;
    draw_label_dim(c,20,y,"DRM/KMS modesetting, software compositing");
}

static void render_wallpaper(lgui_canvas_t *c) {
    draw_label(c,20,16,"Wallpaper");
    draw_accent(c,20,16,"< Back");add_zone(20,16,60,16,PAGE_HOME);
    lgui_canvas_fill_rect(c,16,36,WIN_W-32,1,COL_ACCENT);
    int y=52;
    draw_label(c,20,y,"Select Desktop Wallpaper:");y+=32;
    draw_btn(c,16,y,WIN_W-32,36,"Synthwave Sunrise");add_zone(16,y,WIN_W-32,36,100);y+=44;
    draw_btn(c,16,y,WIN_W-32,36,"Retro Lake at Night");add_zone(16,y,WIN_W-32,36,101);y+=44;
    draw_label_dim(c,20,y,"Changes apply on next compositor refresh.");
}

static void render_network(lgui_canvas_t *c) {
    draw_label(c,20,16,"Network");
    draw_accent(c,20,16,"< Back");add_zone(20,16,60,16,PAGE_HOME);
    lgui_canvas_fill_rect(c,16,36,WIN_W-32,1,COL_ACCENT);
    int y=52;
    draw_label(c,20,y,"Network Interfaces:");y+=24;
    char info[1024];get_network(info,sizeof(info));
    char *line=strtok(info,"\n");
    while(line&&y<WIN_H-40){draw_label_dim(c,20,y,line);y+=18;line=strtok(NULL,"\n");}
}

static void render_audio(lgui_canvas_t *c) {
    draw_label(c,20,16,"Audio");
    draw_accent(c,20,16,"< Back");add_zone(20,16,60,16,PAGE_HOME);
    lgui_canvas_fill_rect(c,16,36,WIN_W-32,1,COL_ACCENT);
    int y=52;
    char info[64];int vol=get_vol();
    snprintf(info,sizeof(info),"Volume: %d%%",vol>=0?vol:0);
    draw_label(c,20,y,info);y+=20;
    draw_label(c,20,y,is_muted_()?"Status: MUTED":"Status: Unmuted");y+=32;
    draw_btn(c,16,y,80,36,"  -  ");add_zone(16,y,80,36,200);y+=0;
    draw_btn(c,100,y,80,36,"  +  ");add_zone(100,y,80,36,201);y+=0;
    draw_btn(c,184,y,140,36,"Toggle Mute");add_zone(184,y,140,36,202);y+=44;
    draw_label_dim(c,20,y,"Audio Server: PipeWire");y+=20;
    draw_label_dim(c,20,y,"Backend: ALSA (amixer)");
}

static void render_users(lgui_canvas_t *c) {
    draw_label(c,20,16,"Users & Accounts");
    draw_accent(c,20,16,"< Back");add_zone(20,16,60,16,PAGE_HOME);
    lgui_canvas_fill_rect(c,16,36,WIN_W-32,1,COL_ACCENT);
    int y=52;
    draw_label(c,20,y,"System Users:");y+=24;
    char info[2048];get_users(info,sizeof(info));
    char *line=strtok(info," ");
    while(line&&y<WIN_H-60){draw_label_dim(c,20,y,line);y+=18;line=strtok(NULL," ");}
}

static void render_about(lgui_canvas_t *c) {
    draw_label(c,20,16,"About System");
    draw_accent(c,20,16,"< Back");add_zone(20,16,60,16,PAGE_HOME);
    lgui_canvas_fill_rect(c,16,36,WIN_W-32,1,COL_ACCENT);
    int y=52;
    char info[256];
    draw_accent(c,20,y,"Mahina OS v0.1.0 (Waxing)");y+=28;
    draw_label(c,20,y,"Kernel:");y+=18;
    get_kernel(info,sizeof(info));draw_label_dim(c,20,y,info);y+=24;
    draw_label(c,20,y,"CPU:");y+=18;
    get_cpu(info,sizeof(info));draw_label_dim(c,20,y,info);y+=24;
    draw_label(c,20,y,"Memory:");y+=18;
    get_ram(info,sizeof(info));draw_label_dim(c,20,y,info);y+=24;
    draw_label(c,20,y,"Disk:");y+=18;
    get_disk(info,sizeof(info));draw_label_dim(c,20,y,info);y+=24;
    draw_label(c,20,y,"Hostname:");y+=18;
    get_hostname_(info,sizeof(info));draw_label_dim(c,20,y,info);y+=24;
    draw_label(c,20,y,"Compositor: lgp-compositor");y+=18;
    draw_label(c,20,y,"GUI Toolkit: LunaGUI v1.0");y+=18;
    draw_label(c,20,y,"License: MIT");
}

/* ── Canvas render callback (file-scope) ────────────────────────────────────── */

static void settings_render(lgui_widget_t *w, lgui_canvas_t *c, int x, int y) {
    (void)w;
    g_zone_count = 0;
    draw_bg(c);
    lgui_canvas_push_clip(c, x, y, WIN_W, WIN_H);

    switch(g_page) {
        case PAGE_HOME:      render_home(c); break;
        case PAGE_DISPLAY:   render_display(c); break;
        case PAGE_WALLPAPER: render_wallpaper(c); break;
        case PAGE_NETWORK:   render_network(c); break;
        case PAGE_AUDIO:     render_audio(c); break;
        case PAGE_USERS:     render_users(c); break;
        case PAGE_ABOUT:     render_about(c); break;
    }
    lgui_canvas_pop_clip(c);
}

/* ── Actions on button click ────────────────────────────────────────────────── */

static void do_action(int action) {
    switch(action) {
        case PAGE_HOME: case PAGE_DISPLAY: case PAGE_WALLPAPER:
        case PAGE_NETWORK: case PAGE_AUDIO: case PAGE_USERS: case PAGE_ABOUT:
            g_page = action; break;
        case 100: /* Synthwave */
            system("cp /usr/share/wallpaper/synthwave.lraw /usr/share/wallpaper/live.lraw 2>/dev/null");
            system("echo wallpaper > /tmp/luna-wallpaper-trigger 2>/dev/null");
            break;
        case 101: /* Retro Lake */
            system("cp /usr/share/wallpaper/retro_lake.lraw /usr/share/wallpaper/live.lraw 2>/dev/null");
            system("echo wallpaper > /tmp/luna-wallpaper-trigger 2>/dev/null");
            break;
        case 200: /* Vol down */
            system("amixer set Master 5%- 2>/dev/null"); break;
        case 201: /* Vol up */
            system("amixer set Master 5%+ 2>/dev/null"); break;
        case 202: /* Mute toggle */
            system("amixer set Master toggle 2>/dev/null"); break;
    }
}

/* ── Pointer callback (file-scope) ──────────────────────────────────────────── */

static lgui_window_t *g_win = NULL;

static void on_pointer(int mx, int my, bool pressed, bool is_btn, void *ud) {
    (void)ud;
    if (!pressed || !is_btn) return;
    for (int i = 0; i < g_zone_count; i++) {
        if (mx >= g_zones[i].x && mx < g_zones[i].x + g_zones[i].w &&
            my >= g_zones[i].y && my < g_zones[i].y + g_zones[i].h) {
            do_action(g_zones[i].action);
            if (g_win) g_win->dirty = true;
            return;
        }
    }
}

/* ── Main ───────────────────────────────────────────────────────────────────── */

int main(void) {
    lgui_application_t *app = lgui_application_create("luna-settings");
    if (!app) return 1;

    g_win = lgui_window_create(app, WIN_W, WIN_H, LGUI_LAYER_APPLICATION);
    if (!g_win) { lgui_application_destroy(app); return 1; }

    lgui_widget_t *cw = lgui_canvas_widget_create();
    lgui_widget_set_size(cw, WIN_W, WIN_H);
    lgui_canvas_widget_set_render(cw, settings_render);
    lgui_window_set_root_widget(g_win, cw);
    lgui_window_show(g_win);

    lgui_application_set_global_pointer_cb(app, on_pointer, NULL);

    lgui_application_run(app);
    lgui_application_destroy(app);
    return 0;
}
