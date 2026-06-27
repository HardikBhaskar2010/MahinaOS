/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * panic.c — Panic Handler Implementation
 * Authority: Volume II / 04_init_system.md §Fail-Fast at PID 1
 */

#include "panic.h"
#include "log.h"

#include <fcntl.h>
#include <string.h>
#include <unistd.h>

#define COMP "panic"

/* Emergency shell candidates — tried in order */
static const char * const SHELL_CANDIDATES[] = {
    "/bin/busybox",
    "/usr/bin/busybox",
    "/bin/sh",
    "/usr/bin/sh",
    NULL
};

void panic_drop_to_shell(const char *reason) {
    LUNA_FATAL(COMP, "PANIC: %s", reason);
    LUNA_FATAL(COMP, "Dropping to emergency shell on TTY1");

    luna_log_signal_safe("PANIC — dropping to emergency shell");

    /* Write directly to TTY1 in case log is also broken */
    static const char banner[] =
        "\r\n"
        "╔══════════════════════════════════════════╗\r\n"
        "║         MAHINA EMERGENCY SHELL           ║\r\n"
        "║  luna-init encountered a fatal error.    ║\r\n"
        "║  Type 'exit' to attempt reboot.          ║\r\n"
        "╚══════════════════════════════════════════╝\r\n"
        "\r\n";

    int tty = open("/dev/tty1", O_WRONLY | O_NOCTTY);
    if (tty >= 0) {
        (void)write(tty, banner, sizeof(banner) - 1);
        if (reason) {
            (void)write(tty, "Reason: ", 8);
            (void)write(tty, reason, strlen(reason));
            (void)write(tty, "\r\n\r\n", 4);
        }
        close(tty);
    }

    /* Try each shell candidate */
    for (int i = 0; SHELL_CANDIDATES[i]; i++) {
        const char *argv[] = { SHELL_CANDIDATES[i], NULL };
        const char *envp[] = {
            "PATH=/usr/bin:/usr/sbin:/bin:/sbin",
            "TERM=linux",
            "HOME=/root",
            NULL
        };
        execve(SHELL_CANDIDATES[i], (char * const *)argv, (char * const *)envp);
        /* execve failed — try next candidate */
    }

    /* No shell found — spin forever (kernel will show a panic message) */
    (void)write(STDERR_FILENO, "FATAL: No emergency shell found.\n", 33);
    while (1) { pause(); }
}
