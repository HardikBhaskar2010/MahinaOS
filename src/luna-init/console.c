/*
 * console.c — Console Output and Welcome Message
 * Authority: Volume VII / 01_implementation_roadmap.md §Stage 0 Success Criteria
 */

#include "console.h"
#include "log.h"

#include <unistd.h>

#define COMP "console"

/* The Stage 0 success criterion message — exact text per mission brief */
static const char WELCOME_BANNER[] =
    "\r\n"
    "\033[1;36m"   /* Bold cyan */
    "  ███╗   ███╗ █████╗ ██╗  ██╗██╗███╗   ██╗ █████╗ \r\n"
    "  ████╗ ████║██╔══██╗██║  ██║██║████╗  ██║██╔══██╗\r\n"
    "  ██╔████╔██║███████║███████║██║██╔██╗ ██║███████║\r\n"
    "  ██║╚██╔╝██║██╔══██║██╔══██║██║██║╚██╗██║██╔══██║\r\n"
    "  ██║ ╚═╝ ██║██║  ██║██║  ██║██║██║ ╚████║██║  ██║\r\n"
    "  ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝\r\n"
    "\033[0m"
    "\r\n"
    "\033[1;37m"   /* Bold white */
    "  Welcome to Mahina.\r\n"
    "\033[0;37m"   /* Normal white */
    "  Architecture Freeze v1\r\n"
    "  System Initialization Complete.\r\n"
    "\033[0m"
    "\r\n"
    "  Stage 0 (v0.1 Bring-up) — Boot chain verified.\r\n"
    "  Type 'luna-init-ctl list' to see service states.\r\n"
    "\r\n";

void console_print_welcome(void) {
    (void)write(STDOUT_FILENO, WELCOME_BANNER, sizeof(WELCOME_BANNER) - 1);
    LUNA_INFO(COMP, "Welcome message displayed");
}

void console_drop_to_shell(void) {
    LUNA_INFO(COMP, "Dropping to interactive root shell");

    /* In v0.1, drop to busybox sh. In v0.5+ this path is replaced by
     * the login manager (luna-lock). */
    const char *shell_paths[] = {
        "/bin/sh",
        "/usr/bin/sh",
        "/bin/busybox",
        NULL
    };

    const char *argv[] = { "sh", NULL };
    const char *envp[] = {
        "PATH=/usr/bin:/usr/sbin:/bin:/sbin",
        "TERM=linux",
        "HOME=/root",
        "USER=root",
        "MAHINA_VERSION=0.1.0",
        NULL
    };

    for (int i = 0; shell_paths[i]; i++) {
        argv[0] = shell_paths[i];
        execve(shell_paths[i], (char * const *)argv, (char * const *)envp);
    }

    LUNA_FATAL(COMP, "No shell found — system halted");
    while (1) { pause(); }
}
