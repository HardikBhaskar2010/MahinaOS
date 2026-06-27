/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * reaper.c — Zombie Process Reaper Implementation
 * Authority: Volume II / 04_init_system.md §Zombie Reaping
 */

#include "reaper.h"
#include "log.h"

#include <stdbool.h>
#include <sys/wait.h>

#define COMP "reaper"

void reaper_reap_all(void) {
    int   wstatus;
    pid_t pid;

    /*
     * Loop until waitpid returns 0 (no more zombies) or -1 (error/no children).
     * WNOHANG: never block — if no zombie is ready, return immediately.
     */
    while ((pid = waitpid(-1, &wstatus, WNOHANG)) > 0) {
        if (WIFEXITED(wstatus)) {
            int code = WEXITSTATUS(wstatus);
            LUNA_DEBUG(COMP, "Reaped PID %d (exit code %d)", (int)pid, code);
            reaper_on_exit(pid, code, false);
        } else if (WIFSIGNALED(wstatus)) {
            int sig = WTERMSIG(wstatus);
            LUNA_DEBUG(COMP, "Reaped PID %d (killed by signal %d)", (int)pid, sig);
            reaper_on_exit(pid, sig, true);
        }
        /* WIFSTOPPED / WIFCONTINUED are not expected for supervised services */
    }
}
