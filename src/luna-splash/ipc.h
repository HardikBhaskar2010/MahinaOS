/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef MAHINA_SPLASH_IPC_H
#define MAHINA_SPLASH_IPC_H

#include <stdbool.h>

#define MAX_IPC_MSG 256

/* Message format: "PERCENT|Message text" e.g. "25|Mounting filesystems" */

int ipc_init(int fd);
void ipc_cleanup(void);
bool ipc_read_event(char *msg_out, int *percent_out);

#endif /* MAHINA_SPLASH_IPC_H */
