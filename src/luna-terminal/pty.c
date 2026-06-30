/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#include <pty.h>
#include <unistd.h>
#include <stdlib.h>

int pty_spawn(pid_t *out_pid, int *out_fd);

int pty_spawn(pid_t *out_pid, int *out_fd) {
    int master_fd = -1;
    pid_t pid = forkpty(&master_fd, NULL, NULL, NULL);
    
    if (pid < 0) return -1;
    
    if (pid == 0) {
        execl("/bin/sh", "sh", NULL);
        exit(1);
    }
    
    if (out_pid) *out_pid = pid;
    if (out_fd) *out_fd = master_fd;
    return 0;
}
