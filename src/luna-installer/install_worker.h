/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * install_worker.h — Installer backend worker declarations.
 *
 * The worker runs in a forked child process and communicates with the
 * UI parent via a pipe using the protocol:
 *
 *   "PERCENT|MSG\n"
 *
 * This is the same wire format as luna-splash's IPC protocol so the
 * installer progress bar can reuse the same reader logic.
 */

#ifndef MAHINA_INSTALL_WORKER_H
#define MAHINA_INSTALL_WORKER_H

#include <stdint.h>

/* Configuration collected by the UI pages before install begins */
typedef struct install_config_t {
    char disk[128];        /* Target block device, e.g. /dev/vda */
    char username[64];     /* Primary user account */
    char hostname[64];     /* Machine hostname */
    char timezone[64];     /* IANA timezone string, e.g. Asia/Kolkata */
    char language[32];     /* POSIX locale, e.g. en_US.UTF-8 */
    char keyboard[32];     /* XKB layout name, e.g. us */
    char password_hash[128]; /* SHA-512 crypt(3) hash of the password */
} install_config_t;

/*
 * install_worker_run() — Entry point for the forked install worker.
 *
 * pipe_write_fd: writable end of the progress pipe (parent reads it).
 * cfg:           pointer to the install configuration.
 *
 * Never returns — calls _exit(0) on success, _exit(1) on failure.
 */
void install_worker_run(int pipe_write_fd, const install_config_t *cfg);

/*
 * install_hash_password() — Compute a crypt(3) SHA-512 hash for the
 * given plaintext password. Writes into out (must be >= 128 bytes).
 * Returns 0 on success, -1 on error.
 */
int install_hash_password(const char *plaintext, char *out, size_t out_sz);

#endif /* MAHINA_INSTALL_WORKER_H */
