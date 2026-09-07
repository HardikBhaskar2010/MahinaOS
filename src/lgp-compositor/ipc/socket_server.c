/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "socket_server.h"
#include "../logging/log.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/un.h>
#include <unistd.h>

/* Direct /etc/group lookup to avoid libc NSS dlopen dependency in static binaries */
static gid_t lookup_video_gid(void) {
    FILE *f = fopen("/etc/group", "r");
    if (!f) return (gid_t)-1;
    char line[256];
    gid_t gid = (gid_t)-1;
    while (fgets(line, sizeof(line), f)) {
        if (strncmp(line, "video:", 6) == 0) {
            /* Format: video:password:gid:... */
            char *colon2 = strchr(line + 6, ':');
            if (colon2) {
                long val = strtol(colon2 + 1, NULL, 10);
                gid = (gid_t)val;
                break;
            }
        }
    }
    fclose(f);
    return gid;
}

int lgp_socket_server_init(void) {
    /* Ensure the directory exists */
    mkdir("/run/lgp", 0755);

    /* Clean up stale socket from previous crash */
    struct stat st = {0};
    if (stat(LGP_SOCKET_PATH, &st) == 0) {
        if (unlink(LGP_SOCKET_PATH) != 0) {
            LGP_ERROR("ipc", "Failed to unlink stale socket: %s", strerror(errno));
            return -1;
        }
    }

    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_NONBLOCK | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        LGP_ERROR("ipc", "Failed to create unix socket: %s", strerror(errno));
        return -1;
    }

    struct sockaddr_un addr = {0};
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, LGP_SOCKET_PATH, sizeof(addr.sun_path) - 1);

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) != 0) {
        LGP_ERROR("ipc", "Failed to bind to %s: %s", LGP_SOCKET_PATH, strerror(errno));
        close(fd);
        return -1;
    }

    if (listen(fd, 32) != 0) {
        LGP_ERROR("ipc", "Failed to listen on socket: %s", strerror(errno));
        close(fd);
        return -1;
    }

    /* Set permissions and ownership (parse /etc/group directly to avoid NSS dependency) */
    gid_t video_gid = lookup_video_gid();
    if (video_gid != (gid_t)-1) {
        chmod(LGP_SOCKET_PATH, 0660);
        chown(LGP_SOCKET_PATH, 0, video_gid);
    } else {
        LGP_WARN("ipc", "Group 'video' not found in /etc/group — relaxing socket permissions to 0666");
        chmod(LGP_SOCKET_PATH, 0666);
    }

    LGP_INFO("ipc", "Listening for LGP connections on %s", LGP_SOCKET_PATH);
    return fd;
}

int lgp_socket_server_accept(int listen_fd) {
    struct sockaddr_un client_addr;
    socklen_t client_len = sizeof(client_addr);
    int client_fd = accept4(listen_fd, (struct sockaddr *)&client_addr, &client_len, SOCK_NONBLOCK | SOCK_CLOEXEC);
    
    if (client_fd < 0) {
        if (errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR) {
            LGP_ERROR("ipc", "accept4 failed: %s", strerror(errno));
        }
        return -1;
    }

    LGP_DEBUG("ipc", "Accepted new LGP connection (fd %d)", client_fd);
    return client_fd;
}
