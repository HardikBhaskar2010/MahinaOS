#ifndef MAHINA_SHUTDOWN_H
#define MAHINA_SHUTDOWN_H

/*
 * shutdown.h — Orderly Shutdown and Reboot Sequencer
 * Authority: Volume II / 04_init_system.md §Shutdown Sequence
 *
 * Total shutdown target: under 5 seconds.
 * Services that do not respond to SIGTERM within timeout are SIGKILLed.
 * There is no exception to this rule (04_init_system.md).
 */

typedef enum {
    SHUTDOWN_POWEROFF = 0,
    SHUTDOWN_REBOOT   = 1,
} shutdown_mode_t;

/*
 * shutdown_run() — Execute orderly shutdown or reboot.
 *
 * Stops all services in reverse dependency order.
 * Unmounts filesystems.
 * Calls reboot(2) with LINUX_REBOOT_CMD_POWER_OFF or LINUX_REBOOT_CMD_RESTART.
 * This function does not return.
 */
void shutdown_run(shutdown_mode_t mode);

/* Global flag read by the event loop to stop accepting new service starts */
extern volatile int g_shutting_down;

#endif /* MAHINA_SHUTDOWN_H */
