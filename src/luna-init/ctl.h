/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef MAHINA_CTL_H
#define MAHINA_CTL_H

/*
 * ctl.h — luna-init Control Socket Server
 * Authority: Volume II / 04_init_system.md §Control Interface — luna-init-ctl
 *
 * Unix domain socket at /run/luna-init.sock.
 * Protocol: newline-delimited JSON request/response.
 * (JSON format per 04_init_system.md default; DL entry to be recorded.)
 *
 * Commands:
 *   status [name]   — service status
 *   list            — all services and states
 *   start <name>    — start a service
 *   stop <name>     — stop a service
 *   restart <name>  — stop then start
 *   reload <name>   — SIGHUP to service
 *   shutdown        — initiate shutdown
 *   reboot          — initiate reboot
 */

/* Socket path per 09_filesystem.md */
#define CTL_SOCKET_PATH "/run/luna-init.sock"

/*
 * ctl_server_init() — Create and bind the control socket.
 *
 * Returns the listening socket fd (add to epoll EPOLLIN), or -1 on failure.
 */
int ctl_server_init(void);

/*
 * ctl_server_accept() — Accept and handle one client connection.
 *
 * Reads one JSON request, processes it, writes one JSON response.
 * Non-blocking: call when epoll reports EPOLLIN on the server fd.
 */
void ctl_server_accept(int server_fd);

/*
 * ctl_server_close() — Close and remove the control socket.
 */
void ctl_server_close(int server_fd);

#endif /* MAHINA_CTL_H */
