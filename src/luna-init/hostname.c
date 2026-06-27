/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * hostname.c — Hostname Configuration (Stage 3)
 * Authority: Volume II / 02_boot_flow.md §Stage 3
 */

#include "hostname.h"
#include "log.h"

#include <errno.h>
#include <fcntl.h>
#include <string.h>
#include <sys/utsname.h>
#include <unistd.h>

#define COMP     "hostname"
#define MAX_HOST 256

void hostname_set(const char *hostname_path) {
    int fd = open(hostname_path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        LUNA_WARN(COMP, "Cannot open %s: %s — using default 'mahina'",
                  hostname_path, strerror(errno));
        sethostname("mahina", 6);
        return;
    }

    char buf[MAX_HOST];
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    close(fd);

    if (n <= 0) {
        LUNA_WARN(COMP, "Empty hostname file %s — using default 'mahina'",
                  hostname_path);
        sethostname("mahina", 6);
        return;
    }

    buf[n] = '\0';

    /* Strip newline / carriage return */
    for (ssize_t i = n - 1; i >= 0; i--) {
        if (buf[i] == '\n' || buf[i] == '\r') buf[i] = '\0';
        else break;
    }

    if (sethostname(buf, strlen(buf)) != 0) {
        LUNA_WARN(COMP, "sethostname('%s') failed: %s", buf, strerror(errno));
        return;
    }

    LUNA_INFO(COMP, "Hostname: %s", buf);
}
