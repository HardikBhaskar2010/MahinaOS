#ifndef MAHINA_REAPER_H
#define MAHINA_REAPER_H

/*
 * reaper.h — Zombie Process Reaper
 * Authority: Volume II / 04_init_system.md §Zombie Reaping
 *
 * PID 1 is responsible for reaping ALL orphaned child processes.
 * Failure to reap zombies causes process table exhaustion.
 * This is a correctness requirement, not a feature.
 */

#include <stdbool.h>
#include <sys/types.h>

/*
 * reaper_reap_all() — Reap all pending zombie children.
 *
 * Calls waitpid(-1, WNOHANG) in a loop until no more zombies are available.
 * For each reaped process, notifies the supervisor via reaper_on_exit().
 *
 * Must be called whenever SIGCHLD is received (SIGNAL_ACTION_REAP).
 * Safe to call spuriously — returns immediately if no zombies present.
 */
void reaper_reap_all(void);

/*
 * reaper_on_exit — Callback invoked when a supervised process exits.
 *
 * Implemented in supervisor.c. The reaper calls this for each reaped PID
 * so the supervisor can update service state and apply restart policy.
 *
 * pid      : the process that exited
 * exit_code: the exit code (WEXITSTATUS) or signal number (WTERMSIG)
 * by_signal: true if the process was killed by a signal
 */
void reaper_on_exit(pid_t pid, int exit_code, bool by_signal);

#endif /* MAHINA_REAPER_H */
