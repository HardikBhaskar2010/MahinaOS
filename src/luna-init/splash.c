/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include "splash.h"
#include "log.h"

#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>

static int splash_pipe[2] = {-1, -1};
static pid_t splash_pid = -1;

void splash_start(void) {
    if (pipe(splash_pipe) < 0) {
        LUNA_WARN("splash", "Failed to create IPC pipe: %s", strerror(errno));
        return;
    }
    
    splash_pid = fork();
    if (splash_pid < 0) {
        LUNA_WARN("splash", "Failed to fork: %s", strerror(errno));
        close(splash_pipe[0]);
        close(splash_pipe[1]);
        splash_pipe[0] = -1;
        splash_pipe[1] = -1;
        return;
    }
    
    if (splash_pid == 0) {
        /* Child process */
        close(splash_pipe[1]); /* Close write end */
        
        char fd_str[16];
        snprintf(fd_str, sizeof(fd_str), "%d", splash_pipe[0]);
        
        const char *argv[] = {
            "/sbin/luna-splash",
            "--fd",     fd_str,
            "--frames", "/usr/share/mahina/splash",
            NULL
        };
        const char *envp[] = { "PATH=/sbin:/bin:/usr/sbin:/usr/bin", NULL };
        
        execve("/sbin/luna-splash", (char * const *)argv, (char * const *)envp);
        
        /* Dev fallback */
        execve("./build/luna-splash/luna-splash", (char * const *)argv, (char * const *)envp);
        
        exit(1);
    }
    
    /* Parent process */
    close(splash_pipe[0]); /* Close read end */
    LUNA_INFO("splash", "Started luna-splash (PID %d)", splash_pid);
}

void splash_update(const char *msg, int percent) {
    if (splash_pipe[1] < 0) return;
    
    char buffer[256];
    int len = snprintf(buffer, sizeof(buffer), "%d|%s\n", percent, msg);
    if (len > 0 && len < (int)sizeof(buffer)) {
        (void)write(splash_pipe[1], buffer, (size_t)len);
    }
}

void splash_stop(void) {
    if (splash_pipe[1] >= 0) {
        close(splash_pipe[1]);
        splash_pipe[1] = -1;
    }
    
    if (splash_pid > 0) {
        kill(splash_pid, SIGTERM);
        /* Wait synchronously so KD_TEXT is restored before we continue */
        waitpid(splash_pid, NULL, 0);
        splash_pid = -1;
    }
}
