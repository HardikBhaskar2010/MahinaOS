/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * ctl.c — luna-init Control Socket Server Implementation
 * Authority: Volume II / 04_init_system.md §Control Interface
 *
 * Protocol: newline-delimited JSON
 * Request:  {"cmd":"list"}
 * Response: {"ok":true,"services":[{"name":"dbus","state":"DEGRADED"},...]}
 *
 * Error:    {"ok":false,"error":"unknown command"}
 */

#include "ctl.h"
#include "log.h"
#include "service.h"
#include "shutdown.h"
#include "supervisor.h"

#include <errno.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

#define COMP     "ctl"
#define BUF_SIZE 4096

/* ─── Minimal JSON builder (no external library) ─────────────────────────── */

static int json_str_escape(char *dst, size_t dst_size, const char *src) {
    size_t out = 0;
    for (size_t i = 0; src[i] && out + 4 < dst_size; i++) {
        if (src[i] == '"' || src[i] == '\\') dst[out++] = '\\';
        dst[out++] = src[i];
    }
    dst[out] = '\0';
    return (int)out;
}

/* ─── Command handlers ───────────────────────────────────────────────────── */

static void handle_list(int client_fd) {
    char resp[BUF_SIZE];
    int  len = 0;

    len += snprintf(resp + len, sizeof(resp) - (size_t)len,
                    "{\"ok\":true,\"services\":[");

    for (int i = 0; i < g_service_count; i++) {
        service_t *svc = &g_services[i];
        char escaped[SERVICE_MAX_NAME_LEN * 2];
        json_str_escape(escaped, sizeof(escaped), svc->name);

        len += snprintf(resp + len, sizeof(resp) - (size_t)len,
                        "%s{\"name\":\"%s\",\"state\":\"%s\",\"pid\":%d}",
                        (i > 0) ? "," : "",
                        escaped,
                        service_state_name(svc->state),
                        (int)svc->pid);
    }

    len += snprintf(resp + len, sizeof(resp) - (size_t)len, "]}\n");
    (void)write(client_fd, resp, (size_t)len);
}

static void handle_status(int client_fd, const char *name) {
    if (!name || name[0] == '\0') {
        /* Overall system status */
        int running = 0, degraded = 0, stopped = 0;
        for (int i = 0; i < g_service_count; i++) {
            switch (g_services[i].state) {
                case SERVICE_STATE_RUNNING:  running++;  break;
                case SERVICE_STATE_DEGRADED: degraded++; break;
                case SERVICE_STATE_STOPPED:  stopped++;  break;
                default: break;
            }
        }
        char resp[256];
        snprintf(resp, sizeof(resp),
                 "{\"ok\":true,\"running\":%d,\"degraded\":%d,\"stopped\":%d}\n",
                 running, degraded, stopped);
        (void)write(client_fd, resp, strlen(resp));
        return;
    }

    service_t *svc = service_find(name);
    if (!svc) {
        char resp[256];
        snprintf(resp, sizeof(resp),
                 "{\"ok\":false,\"error\":\"service not found\"}\n");
        (void)write(client_fd, resp, strlen(resp));
        return;
    }

    char resp[512];
    snprintf(resp, sizeof(resp),
             "{\"ok\":true,\"name\":\"%s\",\"state\":\"%s\","
             "\"pid\":%d,\"restart_count\":%d}\n",
             svc->name, service_state_name(svc->state),
             (int)svc->pid, svc->restart_count);
    (void)write(client_fd, resp, strlen(resp));
}

static void handle_start(int client_fd, const char *name) {
    if (!name || name[0] == '\0') {
        (void)write(client_fd,
                    "{\"ok\":false,\"error\":\"name required\"}\n", 38);
        return;
    }
    if (supervisor_start_one(name) == 0) {
        (void)write(client_fd, "{\"ok\":true}\n", 12);
    } else {
        char resp[256];
        snprintf(resp, sizeof(resp),
                 "{\"ok\":false,\"error\":\"failed to start %s\"}\n", name);
        (void)write(client_fd, resp, strlen(resp));
    }
}

static void handle_stop(int client_fd, const char *name) {
    if (!name || name[0] == '\0') {
        (void)write(client_fd,
                    "{\"ok\":false,\"error\":\"name required\"}\n", 38);
        return;
    }
    supervisor_stop_one(name);
    (void)write(client_fd, "{\"ok\":true}\n", 12);
}

static void handle_reload(int client_fd, const char *name) {
    service_t *svc = name ? service_find(name) : NULL;
    if (!svc || svc->pid <= 0) {
        (void)write(client_fd,
                    "{\"ok\":false,\"error\":\"service not running\"}\n", 43);
        return;
    }
    kill(svc->pid, SIGHUP);
    (void)write(client_fd, "{\"ok\":true}\n", 12);
}

/* ─── Request dispatcher ─────────────────────────────────────────────────── */

/*
 * Minimal JSON field extractor — not a full parser.
 * Extracts the value of "field" from a flat JSON object string.
 * Only handles string values. Returns NULL if field not found.
 */
static const char *json_get_field(const char *json, const char *field,
                                   char *out, size_t out_size) {
    char needle[64];
    snprintf(needle, sizeof(needle), "\"%s\":", field);
    const char *p = strstr(json, needle);
    if (!p) return NULL;
    p += strlen(needle);
    while (*p == ' ') p++;
    if (*p != '"') return NULL;
    p++;
    size_t i = 0;
    while (*p && *p != '"' && i + 1 < out_size) {
        out[i++] = *p++;
    }
    out[i] = '\0';
    return out;
}

static void dispatch(int client_fd, const char *request) {
    char cmd[64]  = {0};
    char name[SERVICE_MAX_NAME_LEN] = {0};

    json_get_field(request, "cmd",  cmd,  sizeof(cmd));
    json_get_field(request, "name", name, sizeof(name));

    LUNA_DEBUG(COMP, "ctl command: '%s' name='%s'", cmd, name);

    if      (strcmp(cmd, "list")     == 0) handle_list(client_fd);
    else if (strcmp(cmd, "status")   == 0) handle_status(client_fd, name);
    else if (strcmp(cmd, "start")    == 0) handle_start(client_fd, name);
    else if (strcmp(cmd, "stop")     == 0) handle_stop(client_fd, name);
    else if (strcmp(cmd, "reload")   == 0) handle_reload(client_fd, name);
    else if (strcmp(cmd, "restart")  == 0) {
        handle_stop(client_fd, name);
        handle_start(client_fd, name);
    }
    else if (strcmp(cmd, "shutdown") == 0) {
        (void)write(client_fd, "{\"ok\":true,\"msg\":\"shutting down\"}\n", 34);
        close(client_fd);
        shutdown_run(SHUTDOWN_POWEROFF);
    }
    else if (strcmp(cmd, "reboot")   == 0) {
        (void)write(client_fd, "{\"ok\":true,\"msg\":\"rebooting\"}\n", 30);
        close(client_fd);
        shutdown_run(SHUTDOWN_REBOOT);
    }
    else {
        char resp[128];
        snprintf(resp, sizeof(resp),
                 "{\"ok\":false,\"error\":\"unknown command: %s\"}\n", cmd);
        (void)write(client_fd, resp, strlen(resp));
    }
}

/* ─── Server lifecycle ───────────────────────────────────────────────────── */

int ctl_server_init(void) {
    /* Remove stale socket file */
    unlink(CTL_SOCKET_PATH);

    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
    if (fd < 0) {
        LUNA_ERROR(COMP, "socket() failed: %s", strerror(errno));
        return -1;
    }

    struct sockaddr_un addr = {0};
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, CTL_SOCKET_PATH, sizeof(addr.sun_path) - 1);

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        LUNA_ERROR(COMP, "bind(%s) failed: %s", CTL_SOCKET_PATH, strerror(errno));
        close(fd);
        return -1;
    }

    /* Only root should communicate with luna-init */
    chmod(CTL_SOCKET_PATH, 0600);

    if (listen(fd, 4) < 0) {
        LUNA_ERROR(COMP, "listen() failed: %s", strerror(errno));
        close(fd);
        return -1;
    }

    LUNA_INFO(COMP, "Control socket listening: %s", CTL_SOCKET_PATH);
    return fd;
}

void ctl_server_accept(int server_fd) {
    int client = accept4(server_fd, NULL, NULL, SOCK_CLOEXEC);
    if (client < 0) {
        if (errno != EAGAIN && errno != EWOULDBLOCK)
            LUNA_ERROR(COMP, "accept() failed: %s", strerror(errno));
        return;
    }

    char buf[BUF_SIZE];
    ssize_t n = read(client, buf, sizeof(buf) - 1);
    if (n > 0) {
        buf[n] = '\0';
        dispatch(client, buf);
    }

    close(client);
}

void ctl_server_close(int server_fd) {
    if (server_fd >= 0) close(server_fd);
    unlink(CTL_SOCKET_PATH);
}
