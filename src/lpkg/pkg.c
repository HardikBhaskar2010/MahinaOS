/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

#include "pkg.h"
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <errno.h>
#include <string.h>

int extract_package(const char *lpkg_path, const char *dest_dir) {
    pid_t pid = fork();
    if (pid == 0) {
        /* Child process: run tar */
        /* Command: tar -xf <lpkg_path> -C <dest_dir> */
        char *argv[] = {
            "tar",
            "-xf",
            (char *)lpkg_path,
            "-C",
            (char *)dest_dir,
            NULL
        };
        execvp("tar", argv);
        /* If exec fails */
        fprintf(stderr, "lpkg: execvp(tar) failed: %s\n", strerror(errno));
        _exit(127);
    } else if (pid < 0) {
        fprintf(stderr, "lpkg: fork() failed: %s\n", strerror(errno));
        return -1;
    }

    int status;
    if (waitpid(pid, &status, 0) < 0) {
        return -1;
    }

    if (WIFEXITED(status) && WEXITSTATUS(status) == 0) {
        return 0;
    }

    fprintf(stderr, "lpkg: tar extraction returned exit code %d\n", WEXITSTATUS(status));
    return -1;
}
