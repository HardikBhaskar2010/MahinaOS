/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

/*
 * main.c — luna-ai-d Entry Point (C Daemon)
 * Unix Socket Server + Presence + Inference Engine
 */

#include "presence.h"
#include "inference.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/epoll.h>
#include <grp.h>
#include <sys/stat.h>

#define SOCKET_PATH "/run/luna-ai.sock"
#define MAX_EVENTS 16
#define MAX_CLIENTS 10
#define BUF_SIZE 2048

static int g_clients[MAX_CLIENTS];
static int g_nclients = 0;

static void set_nonblocking(int fd) {
    int flags = fcntl(fd, F_GETFL, 0);
    fcntl(fd, F_SETFL, flags | O_NONBLOCK);
}

static void add_client(int fd) {
    if (g_nclients < MAX_CLIENTS) {
        g_clients[g_nclients++] = fd;
    } else {
        close(fd);
    }
}

static void remove_client(int fd) {
    for (int i = 0; i < g_nclients; i++) {
        if (g_clients[i] == fd) {
            close(fd);
            g_clients[i] = g_clients[g_nclients - 1];
            g_nclients--;
            break;
        }
    }
}

static void broadcast_mode(ai_mode_t mode) {
    char msg[128];
    int len = snprintf(msg, sizeof(msg),
        "{\"type\":\"mode\",\"mode\":\"%s\"}\n",
        ai_mode_to_string(mode));

    for (int i = 0; i < g_nclients; i++) {
        (void)write(g_clients[i], msg, (size_t)len);
    }
}

static int extract_json_str(const char *json, const char *key, char *out, size_t out_size) {
    char key_pattern[64];
    snprintf(key_pattern, sizeof(key_pattern), "\"%s\":", key);
    const char *pos = strstr(json, key_pattern);
    if (!pos) return -1;
    pos += strlen(key_pattern);
    while (*pos == ' ' || *pos == '\t') pos++;
    if (*pos == '"') {
        pos++;
        size_t idx = 0;
        while (*pos && *pos != '"' && idx + 1 < out_size) {
            out[idx++] = *pos++;
        }
        out[idx] = '\0';
        return 0;
    }
    return -1;
}

static void handle_client_data(int fd) {
    char buf[BUF_SIZE];
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    if (n <= 0) {
        remove_client(fd);
        return;
    }
    buf[n] = '\0';

    char cmd[64] = "";
    if (extract_json_str(buf, "cmd", cmd, sizeof(cmd)) != 0) {
        (void)write(fd, "{\"type\":\"error\",\"text\":\"invalid json: missing cmd\"}\n", 52);
        return;
    }

    if (strcmp(cmd, "get_mode") == 0) {
        char resp[128];
        int rlen = snprintf(resp, sizeof(resp),
            "{\"type\":\"mode\",\"mode\":\"%s\"}\n",
            ai_mode_to_string(presence_get_current_mode()));
        (void)write(fd, resp, (size_t)rlen);
    } 
    else if (strcmp(cmd, "set_active_app") == 0) {
        char app[128] = "";
        if (extract_json_str(buf, "app", app, sizeof(app)) == 0) {
            ai_mode_t old_mode = presence_get_current_mode();
            presence_update_active_app(app);
            ai_mode_t new_mode = presence_get_current_mode();
            if (old_mode != new_mode) {
                broadcast_mode(new_mode);
            } else {
                /* Reply to sender anyway */
                char resp[128];
                int rlen = snprintf(resp, sizeof(resp),
                    "{\"type\":\"mode\",\"mode\":\"%s\"}\n",
                    ai_mode_to_string(new_mode));
                (void)write(fd, resp, (size_t)rlen);
            }
        } else {
            (void)write(fd, "{\"type\":\"error\",\"text\":\"missing app field\"}\n", 44);
        }
    } 
    else if (strcmp(cmd, "prompt") == 0) {
        char text[2048] = "";
        if (extract_json_str(buf, "text", text, sizeof(text)) == 0) {
            /* Streams tokens directly to client fd */
            inference_prompt(fd, text);
        } else {
            (void)write(fd, "{\"type\":\"error\",\"text\":\"missing text field\"}\n", 45);
        }
    } 
    else {
        (void)write(fd, "{\"type\":\"error\",\"text\":\"unknown command\"}\n", 41);
    }
}

int main(void) {
    presence_init();

    /* Clean up stale socket */
    unlink(SOCKET_PATH);

    int listen_fd = socket(AF_UNIX, SOCK_STREAM | SOCK_NONBLOCK, 0);
    if (listen_fd < 0) {
        fprintf(stderr, "luna-ai-d: socket() failed: %s\n", strerror(errno));
        return 1;
    }

    struct sockaddr_un addr = {0};
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, SOCKET_PATH, sizeof(addr.sun_path) - 1);

    if (bind(listen_fd, (struct sockaddr *)&addr, sizeof(addr)) != 0) {
        fprintf(stderr, "luna-ai-d: bind() to %s failed: %s\n", SOCKET_PATH, strerror(errno));
        close(listen_fd);
        return 1;
    }

    if (listen(listen_fd, 10) != 0) {
        fprintf(stderr, "luna-ai-d: listen() failed: %s\n", strerror(errno));
        close(listen_fd);
        return 1;
    }

    /* Set socket permissions */
    chmod(SOCKET_PATH, 0666);

    int epoll_fd = epoll_create1(EPOLL_CLOEXEC);
    if (epoll_fd < 0) {
        fprintf(stderr, "luna-ai-d: epoll_create1() failed\n");
        close(listen_fd);
        return 1;
    }

    struct epoll_event ev, events[MAX_EVENTS];
    ev.events = EPOLLIN;
    ev.data.fd = listen_fd;
    if (epoll_ctl(epoll_fd, EPOLL_CTL_ADD, listen_fd, &ev) != 0) {
        fprintf(stderr, "luna-ai-d: epoll_ctl ADD listen_fd failed\n");
        close(epoll_fd);
        close(listen_fd);
        return 1;
    }

    printf("luna-ai-d: listening on %s\n", SOCKET_PATH);

    while (true) {
        int nfds = epoll_wait(epoll_fd, events, MAX_EVENTS, -1);
        if (nfds < 0) {
            if (errno == EINTR) continue;
            break;
        }

        for (int i = 0; i < nfds; i++) {
            if (events[i].data.fd == listen_fd) {
                struct sockaddr_un client_addr;
                socklen_t client_len = sizeof(client_addr);
                int client_fd = accept(listen_fd, (struct sockaddr *)&client_addr, &client_len);
                if (client_fd >= 0) {
                    set_nonblocking(client_fd);
                    add_client(client_fd);
                    
                    struct epoll_event cev;
                    cev.events = EPOLLIN | EPOLLHUP;
                    cev.data.fd = client_fd;
                    epoll_ctl(epoll_fd, EPOLL_CTL_ADD, client_fd, &cev);
                }
            } else {
                int fd = events[i].data.fd;
                if (events[i].events & (EPOLLHUP | EPOLLERR)) {
                    epoll_ctl(epoll_fd, EPOLL_CTL_DEL, fd, NULL);
                    remove_client(fd);
                } else if (events[i].events & EPOLLIN) {
                    handle_client_data(fd);
                }
            }
        }
    }

    close(epoll_fd);
    close(listen_fd);
    unlink(SOCKET_PATH);
    return 0;
}
