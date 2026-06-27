/*
 * shutdown.c — Orderly Shutdown / Reboot Implementation
 * Authority: Volume II / 04_init_system.md §Shutdown Sequence
 *
 * Sequence (per spec):
 *   1. Mark system shutting down — no new services started
 *   2. Stop services in reverse dependency order
 *   3. Unmount filesystems in reverse mount order
 *   4. Call reboot(2) syscall
 */

#include "shutdown.h"
#include "log.h"
#include "mount.h"
#include "service.h"
#include "supervisor.h"

#include <errno.h>
#include <linux/reboot.h>
#include <string.h>
#include <sys/reboot.h>
#include <time.h>
#include <unistd.h>

#define COMP "shutdown"

/* Global flag read by the event loop to stop accepting new service starts */
volatile int g_shutting_down = 0;

static void sleep_ms(int ms) {
    struct timespec ts = { .tv_sec = ms / 1000, .tv_nsec = (ms % 1000) * 1000000 };
    nanosleep(&ts, NULL);
}

void shutdown_run(shutdown_mode_t mode) {
    g_shutting_down = 1;

    LUNA_INFO(COMP, "Shutdown initiated. Mode: %s",
              mode == SHUTDOWN_REBOOT ? "REBOOT" : "POWEROFF");

    /* Step 1: Flush the log */
    luna_log_close();

    /* Step 2: Stop all services in reverse start order (reverse dep order).
     * Simple approach: stop in reverse index order (services were started
     * in topological order so reversing gives a safe stop order). */
    for (int i = g_service_count - 1; i >= 0; i--) {
        service_t *svc = &g_services[i];
        if (svc->state == SERVICE_STATE_RUNNING ||
            svc->state == SERVICE_STATE_STARTING) {
            supervisor_stop_one(svc->name);
        }
    }

    /* Give processes a moment to clean up after SIGKILL */
    sleep_ms(200);

    /* Step 3: Unmount filesystems in reverse order */
    mount_unmount_all();

    /* Step 4: Kernel reboot/poweroff */
    sync();

    if (mode == SHUTDOWN_REBOOT) {
        reboot(LINUX_REBOOT_CMD_RESTART);
    } else {
        reboot(LINUX_REBOOT_CMD_POWER_OFF);
    }

    /* Should never reach here */
    while (1) { sleep_ms(1000); }
}
