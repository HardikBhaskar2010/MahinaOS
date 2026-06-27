/*
 * luna-init-ctl/main.c — Control CLI for luna-init
 * Authority: Volume II / 04_init_system.md §Control Interface — luna-init-ctl
 *
 * Connects to /run/luna-init.sock, sends a JSON command, prints the response.
 *
 * Usage:
 *   luna-init-ctl list
 *   luna-init-ctl status [service-name]
 *   luna-init-ctl start <service-name>
 *   luna-init-ctl stop <service-name>
 *   luna-init-ctl restart <service-name>
 *   luna-init-ctl reload <service-name>
 *   luna-init-ctl shutdown
 *   luna-init-ctl reboot
 */

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#define SOCKET_PATH "/run/luna-init.sock"
#define BUF_SIZE    8192

static void usage(void) {
    fprintf(stderr,
        "luna-init-ctl — Mahina service control\n"
        "\n"
        "Usage:\n"
        "  luna-init-ctl list\n"
        "  luna-init-ctl status [service-name]\n"
        "  luna-init-ctl start <service-name>\n"
        "  luna-init-ctl stop <service-name>\n"
        "  luna-init-ctl restart <service-name>\n"
        "  luna-init-ctl reload <service-name>\n"
        "  luna-init-ctl shutdown\n"
        "  luna-init-ctl reboot\n"
        "\n"
        "Socket: " SOCKET_PATH "\n");
}

static int connect_to_daemon(void) {
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        fprintf(stderr, "luna-init-ctl: socket() failed: %s\n", strerror(errno));
        return -1;
    }

    struct sockaddr_un addr = {0};
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, SOCKET_PATH, sizeof(addr.sun_path) - 1);

    if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        fprintf(stderr,
                "luna-init-ctl: cannot connect to %s: %s\n"
                "  Is luna-init running? Is this system booted with Mahina?\n",
                SOCKET_PATH, strerror(errno));
        close(fd);
        return -1;
    }

    return fd;
}

static int send_command(int fd, const char *json) {
    if (write(fd, json, strlen(json)) < 0) {
        fprintf(stderr, "luna-init-ctl: write failed: %s\n", strerror(errno));
        return -1;
    }

    char buf[BUF_SIZE];
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    if (n <= 0) {
        fprintf(stderr, "luna-init-ctl: read failed: %s\n", strerror(errno));
        return -1;
    }
    buf[n] = '\0';

    /* Pretty-print the JSON response (minimal indentation for legibility) */
    /* For Stage 0 — print raw. A formatter can be added in v0.5. */
    printf("%s", buf);
    if (buf[n - 1] != '\n') printf("\n");

    /* Return exit code based on "ok" field */
    return (strstr(buf, "\"ok\":true") != NULL) ? 0 : 1;
}

int main(int argc, char *argv[]) {
    if (argc < 2) {
        usage();
        return 1;
    }

    const char *cmd  = argv[1];
    const char *name = (argc >= 3) ? argv[2] : "";

    /* Build JSON request */
    char json[512];

    if (strcmp(cmd, "list") == 0) {
        snprintf(json, sizeof(json), "{\"cmd\":\"list\"}\n");
    } else if (strcmp(cmd, "status") == 0) {
        snprintf(json, sizeof(json),
                 "{\"cmd\":\"status\",\"name\":\"%s\"}\n", name);
    } else if (strcmp(cmd, "start") == 0) {
        if (!name[0]) { fprintf(stderr, "error: start requires a service name\n"); return 1; }
        snprintf(json, sizeof(json),
                 "{\"cmd\":\"start\",\"name\":\"%s\"}\n", name);
    } else if (strcmp(cmd, "stop") == 0) {
        if (!name[0]) { fprintf(stderr, "error: stop requires a service name\n"); return 1; }
        snprintf(json, sizeof(json),
                 "{\"cmd\":\"stop\",\"name\":\"%s\"}\n", name);
    } else if (strcmp(cmd, "restart") == 0) {
        if (!name[0]) { fprintf(stderr, "error: restart requires a service name\n"); return 1; }
        snprintf(json, sizeof(json),
                 "{\"cmd\":\"restart\",\"name\":\"%s\"}\n", name);
    } else if (strcmp(cmd, "reload") == 0) {
        if (!name[0]) { fprintf(stderr, "error: reload requires a service name\n"); return 1; }
        snprintf(json, sizeof(json),
                 "{\"cmd\":\"reload\",\"name\":\"%s\"}\n", name);
    } else if (strcmp(cmd, "shutdown") == 0) {
        snprintf(json, sizeof(json), "{\"cmd\":\"shutdown\"}\n");
    } else if (strcmp(cmd, "reboot") == 0) {
        snprintf(json, sizeof(json), "{\"cmd\":\"reboot\"}\n");
    } else {
        fprintf(stderr, "luna-init-ctl: unknown command '%s'\n", cmd);
        usage();
        return 1;
    }

    int fd = connect_to_daemon();
    if (fd < 0) return 1;

    int ret = send_command(fd, json);
    close(fd);
    return ret;
}
