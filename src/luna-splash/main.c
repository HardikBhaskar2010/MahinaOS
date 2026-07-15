/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * luna-splash/main.c — Animated Boot Splash
 *
 * Displays the MahinaReveal.mp4 animation (pre-extracted as PNG frames in
 * /usr/share/mahina/splash/) centred on the framebuffer at native 1:1
 * resolution, with a boot-progress overlay composited on top.
 *
 * Modes:
 *   Frame-sequence mode  — if a valid frames directory is found.
 *   Static fallback mode — if the directory is missing or empty: renders the
 *                          existing ASCII MAHINA logo + progress bar.
 *
 * IPC:  luna-init communicates progress via a pipe (--fd <N>).
 * Exit: SIGTERM / SIGINT → render_fade_out() → clean exit.
 */

#include "frames.h"
#include "ipc.h"
#include "render.h"

#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/ioctl.h>
#include <sys/signalfd.h>
#include <sys/timerfd.h>
#include <unistd.h>
#include <fcntl.h>
#include <linux/kd.h>

#define MAX_EVENTS         8
#define DEFAULT_FRAMES_DIR "/usr/share/mahina/splash"

/* ── Signal setup ───────────────────────────────────────────────────────── */

static int setup_signals(void) {
    sigset_t mask;
    sigemptyset(&mask);
    sigaddset(&mask, SIGTERM);
    sigaddset(&mask, SIGINT);
    if (sigprocmask(SIG_BLOCK, &mask, NULL) == -1) return -1;
    return signalfd(-1, &mask, SFD_NONBLOCK | SFD_CLOEXEC);
}

/* ── Entry point ────────────────────────────────────────────────────────── */

int main(int argc, char **argv) {
    int         pipe_fd     = -1;
    int         sigfd       = -1;
    int         frame_timer = -1;
    int         epfd        = -1;
    bool        frame_mode  = false;
    int         last_pct    = -1;
    char        last_msg[MAX_IPC_MSG] = {0};
    const char *frames_dir  = DEFAULT_FRAMES_DIR;

    /* ── Parse arguments ─────────────────────────────────────────────── */
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--fd") == 0 && i + 1 < argc) {
            pipe_fd = atoi(argv[++i]);
            int flags = fcntl(pipe_fd, F_GETFL, 0);
            if (flags >= 0) fcntl(pipe_fd, F_SETFL, flags | O_NONBLOCK);
        } else if (strcmp(argv[i], "--frames") == 0 && i + 1 < argc) {
            frames_dir = argv[++i];
        }
    }

    /* ── Switch tty to graphics mode ─────────────────────────────────── */
    int tty_fd = open("/dev/tty0", O_RDWR | O_CLOEXEC);
    if (tty_fd >= 0) ioctl(tty_fd, KDSETMODE, KD_GRAPHICS);

    /* ── Framebuffer init ────────────────────────────────────────────── */
    if (render_init() < 0) {
        fprintf(stderr, "luna-splash: Failed to open framebuffer\n");
        goto cleanup;
    }
    dprintf(STDERR_FILENO, "luna-splash: framebuffer ready\n");

    /* ── IPC pipe ────────────────────────────────────────────────────── */
    if (ipc_init(pipe_fd) < 0)
        dprintf(STDERR_FILENO, "luna-splash: IPC not initialised (no pipe)\n");

    /* ── Signals ─────────────────────────────────────────────────────── */
    epfd = epoll_create1(EPOLL_CLOEXEC);
    if (epfd < 0) goto cleanup;

    sigfd = setup_signals();
    if (sigfd < 0) goto cleanup;

    struct epoll_event ev = {0};
    ev.events  = EPOLLIN;
    ev.data.fd = sigfd;
    epoll_ctl(epfd, EPOLL_CTL_ADD, sigfd, &ev);

    if (pipe_fd >= 0) {
        ev.events  = EPOLLIN;
        ev.data.fd = pipe_fd;
        epoll_ctl(epfd, EPOLL_CTL_ADD, pipe_fd, &ev);
    }

    /* ── Try to load frame sequence ──────────────────────────────────── */
    int n_frames = frames_init(frames_dir);
    if (n_frames > 0) {
        frame_mode = true;
        int fps = frames_get_fps();   /* from meta.txt, e.g. 30 */

        dprintf(STDERR_FILENO,
                "luna-splash: frame-sequence mode — %d frames @ %dfps from %s\n",
                n_frames, fps, frames_dir);

        /* timerfd fires at the video's native FPS so playback speed matches
         * the original recording exactly. */
        frame_timer = timerfd_create(CLOCK_MONOTONIC, TFD_NONBLOCK | TFD_CLOEXEC);
        if (frame_timer < 0) {
            dprintf(STDERR_FILENO,
                    "luna-splash: timerfd_create failed — fallback to static\n");
            frame_mode = false;
        } else {
            long frame_ns = 1000000000L / (fps > 0 ? fps : 30);
            struct itimerspec ts = {
                .it_interval = { 0, frame_ns },
                .it_value    = { 0, frame_ns },
            };
            timerfd_settime(frame_timer, 0, &ts, NULL);

            ev.events  = EPOLLIN;
            ev.data.fd = frame_timer;
            epoll_ctl(epfd, EPOLL_CTL_ADD, frame_timer, &ev);

            /* Render first frame immediately (black bg + first PNG) */
            frames_load_next((uint32_t *)render_get_fb_mem(),
                             screen_w, screen_h,
                             render_get_stride());
        }
    }

    /* ── Static fallback ─────────────────────────────────────────────── */
    if (!frame_mode) {
        dprintf(STDERR_FILENO, "luna-splash: static fallback mode\n");
        render_clear(COLOR_BG);
        render_logo();
        render_progress("Awaiting instructions...", 0);
    }

    /* ── Main event loop ─────────────────────────────────────────────── */
    bool running = true;
    struct epoll_event events[MAX_EVENTS];

    while (running) {
        int n = epoll_wait(epfd, events, MAX_EVENTS, -1);
        if (n < 0) {
            if (errno == EINTR) continue;
            break;
        }

        for (int i = 0; i < n; i++) {
            int fd = events[i].data.fd;

            /* ── Signal ─────────────────────────────────────────────── */
            if (fd == sigfd) {
                struct signalfd_siginfo fdsi;
                ssize_t s = read(sigfd, &fdsi, sizeof(fdsi));
                if (s == sizeof(fdsi)) {
                    if (fdsi.ssi_signo == SIGTERM ||
                        fdsi.ssi_signo == SIGINT) {
                        running = false;
                    }
                }

            /* ── Frame timer tick ────────────────────────────────────── */
            } else if (fd == frame_timer && frame_mode) {
                uint64_t exp;
                (void)read(frame_timer, &exp, sizeof(exp));

                /* Decode next PNG frame into framebuffer */
                frames_load_next((uint32_t *)render_get_fb_mem(),
                                 screen_w, screen_h,
                                 render_get_stride());

                /* Re-composite the IPC progress overlay on top */
                if (last_pct >= 0)
                    render_overlay_progress(last_msg, last_pct);

            /* ── IPC progress update ─────────────────────────────────── */
            } else if (fd == pipe_fd) {
                char msg[MAX_IPC_MSG];
                int  pct;

                /* Drain all buffered messages; only the last one matters */
                while (ipc_read_event(msg, &pct)) {
                    strncpy(last_msg, msg, sizeof(last_msg) - 1);
                    last_msg[sizeof(last_msg) - 1] = '\0';
                    last_pct = pct;
                }

                if (!frame_mode) {
                    /* Static path: full redraw */
                    render_clear(COLOR_BG);
                    render_logo();
                    render_progress(last_msg, last_pct);
                }
                /* Frame-sequence path: overlay applied on next timer tick */
            }
        }
    }

    dprintf(STDERR_FILENO, "luna-splash: exiting — fading out\n");
    render_fade_out();

cleanup:
    frames_cleanup();
    if (frame_timer >= 0) close(frame_timer);
    if (sigfd       >= 0) close(sigfd);
    if (epfd        >= 0) close(epfd);
    ipc_cleanup();
    render_cleanup();
    if (tty_fd >= 0) {
        ioctl(tty_fd, KDSETMODE, KD_TEXT);
        close(tty_fd);
    }
    return 0;
}
