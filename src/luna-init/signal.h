#ifndef MAHINA_SIGNAL_H
#define MAHINA_SIGNAL_H

/*
 * signal.h — Signal Handler for luna-init
 * Authority: Volume II / 04_init_system.md §Signal Handling
 *
 * Uses signalfd() to handle signals in the epoll event loop.
 * This avoids signal handler race conditions (async-signal-safety issues).
 *
 * Signal table (04_init_system.md):
 *   SIGCHLD  → zombie reap + service supervisor notification
 *   SIGTERM  → orderly shutdown
 *   SIGINT   → orderly shutdown (Ctrl+Alt+Del)
 *   SIGHUP   → reload service files from disk
 *   SIGUSR1  → dump state to /var/log/luna-init/state-dump.log
 */

#include <stdbool.h>
#include <sys/signalfd.h>

/* Signal actions dispatched from the epoll loop */
typedef enum {
    SIGNAL_ACTION_NONE     = 0,
    SIGNAL_ACTION_REAP     = 1,  /* SIGCHLD — reap children    */
    SIGNAL_ACTION_SHUTDOWN = 2,  /* SIGTERM — orderly shutdown */
    SIGNAL_ACTION_REBOOT   = 3,  /* SIGINT  — orderly reboot   */
    SIGNAL_ACTION_RELOAD   = 4,  /* SIGHUP  — reload services  */
    SIGNAL_ACTION_DUMP     = 5,  /* SIGUSR1 — dump state       */
} signal_action_t;

/*
 * signal_init() — Block signals and create signalfd.
 *
 * Must be called before the epoll loop starts.
 * Returns the signalfd file descriptor (add to epoll EPOLLIN),
 * or -1 on failure.
 */
int signal_init(void);

/*
 * signal_read() — Read one signal from signalfd and return its action.
 *
 * Call when epoll reports EPOLLIN on the signalfd descriptor.
 * Returns the action to dispatch, or SIGNAL_ACTION_NONE on error.
 */
signal_action_t signal_read(int sigfd);

/*
 * signal_close() — Close the signalfd descriptor.
 */
void signal_close(int sigfd);

#endif /* MAHINA_SIGNAL_H */
