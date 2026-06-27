/*
 * supervisor.c — Service Supervisor Implementation
 * Authority: Volume II / 04_init_system.md §Service Supervisor
 */

#include "supervisor.h"
#include "depgraph.h"
#include "log.h"
#include "reaper.h"

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

#define COMP "supervisor"

/* ─── Time helpers ───────────────────────────────────────────────────────── */

static long long now_ms(void) {
    struct timespec ts = {0};
    clock_gettime(CLOCK_MONOTONIC_RAW, &ts);
    return (long long)(ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
}

static void sleep_ms(int ms) {
    struct timespec ts = { .tv_sec = ms / 1000, .tv_nsec = (ms % 1000) * 1000000 };
    nanosleep(&ts, NULL);
}

/* ─── Readiness polling ──────────────────────────────────────────────────── */

bool supervisor_check_ready(service_t *svc, long long start_ms) {
    long long elapsed = now_ms() - start_ms;
    if (elapsed > svc->ready_timeout_ms) return false;

    switch (svc->ready_method) {
        case READY_NONE:
            return true; /* Assume ready immediately */

        case READY_FILE: {
            struct stat st = {0};
            return (stat(svc->ready_path, &st) == 0);
        }

        case READY_SOCKET: {
            int fd = socket(AF_UNIX, SOCK_STREAM, 0);
            if (fd < 0) return false;

            struct sockaddr_un addr = {0};
            addr.sun_family = AF_UNIX;
            strncpy(addr.sun_path, svc->ready_path, sizeof(addr.sun_path) - 1);

            bool ok = (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) == 0);
            close(fd);
            return ok;
        }

        case READY_HTTP:
            /* Stubbed in v0.1 — HTTP readiness requires libcurl or similar.
             * For Stage 0, services using method=http are treated as READY_NONE.
             * TODO: Implement HTTP readiness before v0.5 service integration. */
            LUNA_WARN(COMP, "Service '%s': ready method 'http' not implemented "
                      "in v0.1 — treating as immediate", svc->name);
            return true;

        case READY_SIGNAL:
            /* Stubbed — signal readiness requires tracking per-service SIGUSR1.
             * Treated as READY_NONE for v0.1. */
            return true;

        default:
            return true;
    }
}

/* ─── Service spawning ───────────────────────────────────────────────────── */

static pid_t spawn_service(service_t *svc) {
    /* Verify binary exists before forking */
    struct stat st = {0};
    if (stat(svc->binary, &st) != 0) {
        LUNA_WARN(COMP, "Service '%s': binary not found '%s' — DEGRADED",
                  svc->name, svc->binary);
        svc->state = SERVICE_STATE_DEGRADED;
        return -1;
    }

    pid_t pid = fork();
    if (pid < 0) {
        LUNA_ERROR(COMP, "fork() failed for '%s': %s", svc->name, strerror(errno));
        return -1;
    }

    if (pid == 0) {
        /* ── Child process ── */

        /* Set working directory */
        if (svc->workdir[0] && chdir(svc->workdir) != 0) {
            /* Non-fatal — continue from / */
        }

        /* Build argv: [binary, arg0, arg1, ..., NULL] */
        const char *argv[SERVICE_MAX_ARGS + 2];
        argv[0] = svc->binary;
        for (int i = 0; i < svc->arg_count; i++) {
            argv[i + 1] = svc->args[i];
        }
        argv[svc->arg_count + 1] = NULL;

        /* Build envp from service env vars + minimal base environment */
        char env_strs[SERVICE_MAX_ENV_VARS + 4][SERVICE_MAX_ENV_LEN * 2];
        const char *envp[SERVICE_MAX_ENV_VARS + 8];
        int env_count = 0;

        snprintf(env_strs[env_count], sizeof(env_strs[0]),
                 "PATH=/usr/bin:/usr/sbin:/bin:/sbin");
        envp[env_count] = env_strs[env_count];
        env_count++;

        for (int i = 0; i < svc->env_count; i++) {
            snprintf(env_strs[env_count], sizeof(env_strs[0]),
                     "%s=%s", svc->env[i].key, svc->env[i].val);
            envp[env_count] = env_strs[env_count];
            env_count++;
        }
        envp[env_count] = NULL;

        execve(svc->binary, (char * const *)argv, (char * const *)envp);

        /* execve() only returns on failure */
        _exit(127); /* 127 = command not found (POSIX convention) */
    }

    /* ── Parent process ── */
    svc->pid            = pid;
    svc->state          = SERVICE_STATE_STARTING;
    svc->start_time_ms  = now_ms();

    LUNA_INFO(COMP, "Started service '%s' (PID %d)", svc->name, (int)pid);
    return pid;
}

/* ─── Wait for readiness ─────────────────────────────────────────────────── */

static bool wait_for_ready(service_t *svc) {
    if (svc->ready_method == READY_NONE) {
        svc->state = SERVICE_STATE_RUNNING;
        LUNA_INFO(COMP, "Service '%s' ready (no readiness check)", svc->name);
        return true;
    }

    long long start = now_ms();
    while (now_ms() - start < svc->ready_timeout_ms) {
        if (supervisor_check_ready(svc, start)) {
            svc->state = SERVICE_STATE_RUNNING;
            LUNA_INFO(COMP, "Service '%s' ready (%lld ms)",
                      svc->name, now_ms() - start);
            return true;
        }
        sleep_ms(100); /* poll every 100ms */
    }

    LUNA_ERROR(COMP, "Service '%s' readiness timeout (%d ms) — DEGRADED",
               svc->name, svc->ready_timeout_ms);
    svc->state = SERVICE_STATE_DEGRADED;
    return false;
}

/* ─── Public API ─────────────────────────────────────────────────────────── */

int supervisor_start_all(void) {
    int order[SERVICE_MAX_COUNT];
    int count = depgraph_topo_sort(order, SERVICE_MAX_COUNT);
    if (count < 0) return -1;

    for (int i = 0; i < count; i++) {
        service_t *svc = &g_services[order[i]];

        if (svc->state == SERVICE_STATE_DEGRADED) continue; /* already marked */

        LUNA_INFO(COMP, "Starting service [%d/%d]: %s", i + 1, count, svc->name);

        pid_t pid = spawn_service(svc);
        if (pid < 0) {
            /* spawn_service already marked DEGRADED if binary missing */
            if (svc->state != SERVICE_STATE_DEGRADED) {
                svc->state = SERVICE_STATE_DEGRADED;
            }
            LUNA_ERROR(COMP, "Service '%s' could not be started — DEGRADED",
                       svc->name);
            continue;
        }

        /* Wait for readiness before moving to next service */
        wait_for_ready(svc);
    }

    return 0;
}

int supervisor_start_one(const char *name) {
    service_t *svc = service_find(name);
    if (!svc) {
        LUNA_ERROR(COMP, "Service '%s' not found", name);
        return -1;
    }
    if (svc->state == SERVICE_STATE_RUNNING ||
        svc->state == SERVICE_STATE_STARTING) {
        LUNA_WARN(COMP, "Service '%s' already running", name);
        return 0;
    }

    svc->state         = SERVICE_STATE_PENDING;
    svc->restart_count = 0;

    pid_t pid = spawn_service(svc);
    if (pid < 0) return -1;
    wait_for_ready(svc);
    return 0;
}

int supervisor_stop_one(const char *name) {
    service_t *svc = service_find(name);
    if (!svc || svc->pid <= 0) return 0;

    LUNA_INFO(COMP, "Stopping service '%s' (PID %d)", name, (int)svc->pid);
    svc->state = SERVICE_STATE_STOPPING;

    kill(svc->pid, svc->stop_signal);

    long long start = now_ms();
    while (now_ms() - start < svc->stop_timeout_ms) {
        if (kill(svc->pid, 0) != 0) { /* process gone */
            svc->state = SERVICE_STATE_STOPPED;
            svc->pid   = 0;
            LUNA_INFO(COMP, "Service '%s' stopped", name);
            return 0;
        }
        sleep_ms(50);
    }

    /* Timeout — SIGKILL */
    LUNA_WARN(COMP, "Service '%s' did not stop in %dms — sending SIGKILL",
              name, svc->stop_timeout_ms);
    kill(svc->pid, SIGKILL);
    svc->state = SERVICE_STATE_STOPPED;
    svc->pid   = 0;
    return 0;
}

/* ─── Reaper callback (declared in reaper.h) ─────────────────────────────── */

void reaper_on_exit(pid_t pid, int exit_code, bool by_signal) {
    service_t *svc = service_find_by_pid(pid);
    if (!svc) {
        LUNA_DEBUG(COMP, "Reaped unknown PID %d (exit %d)", (int)pid, exit_code);
        return;
    }

    svc->pid           = 0;
    svc->stop_time_ms  = now_ms();

    if (svc->state == SERVICE_STATE_STOPPING ||
        svc->state == SERVICE_STATE_STOPPED) {
        svc->state = SERVICE_STATE_STOPPED;
        LUNA_INFO(COMP, "Service '%s' exited cleanly (exit %d)", svc->name, exit_code);
        return;
    }

    /* Unexpected exit */
    svc->state = SERVICE_STATE_FAILED;
    if (by_signal) {
        LUNA_ERROR(COMP, "Service '%s' killed by signal %d", svc->name, exit_code);
    } else {
        LUNA_ERROR(COMP, "Service '%s' exited with code %d", svc->name, exit_code);
    }

    /* Apply restart policy */
    bool should_restart = false;
    switch (svc->restart_policy) {
        case RESTART_ALWAYS:     should_restart = true;                    break;
        case RESTART_ON_FAILURE: should_restart = (exit_code != 0 || by_signal); break;
        case RESTART_NEVER:      should_restart = false;                   break;
    }

    if (!should_restart) {
        svc->state = SERVICE_STATE_STOPPED;
        LUNA_INFO(COMP, "Service '%s': restart policy=never — not restarting",
                  svc->name);
        return;
    }

    svc->restart_count++;
    if (svc->restart_count > svc->restart_attempts) {
        svc->state = SERVICE_STATE_DEGRADED;
        LUNA_ERROR(COMP, "Service '%s': exceeded %d restart attempts — DEGRADED",
                   svc->name, svc->restart_attempts);
        return;
    }

    LUNA_WARN(COMP, "Service '%s': restarting (attempt %d/%d) in %dms",
              svc->name, svc->restart_count, svc->restart_attempts,
              svc->restart_delay_ms);

    sleep_ms(svc->restart_delay_ms);
    spawn_service(svc);
    wait_for_ready(svc);
}
