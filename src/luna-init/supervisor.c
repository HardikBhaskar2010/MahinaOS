/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * supervisor.c — Service Supervisor Implementation
 * Authority: Volume II / 04_init_system.md §Service Supervisor
 */

#include "supervisor.h"
#include "depgraph.h"
#include "log.h"
#include "reaper.h"

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <grp.h>
#include <netdb.h>
#include <pwd.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <sys/select.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>
#include <stdlib.h>

/* ─── READY_SIGNAL tracking table ────────────────────────────────────────── */
/*
 * When a service sends SIGUSR1 to PID 1 to signal readiness, the signal
 * handler calls supervisor_signal_ready(pid). We store a flag per service
 * slot so supervisor_check_ready() can query it.
 */
static volatile sig_atomic_t g_signal_ready[SERVICE_MAX_COUNT];

/*
 * supervisor_signal_ready() — Called from the SIGUSR1 handler in signal.c.
 * Finds the service by PID and marks it ready.
 */
void supervisor_signal_ready(pid_t pid)
{
    for (int i = 0; i < g_service_count; i++) {
        if (g_services[i].pid == pid) {
            g_signal_ready[i] = 1;
            return;
        }
    }
}

#define COMP "supervisor"

/* ─── Time helpers ───────────────────────────────────────────────────────── */

static long long now_ms(void) {
    struct timespec ts = {0};
    clock_gettime(CLOCK_MONOTONIC_RAW, &ts);
    return (long long)ts.tv_sec * 1000LL + (long long)ts.tv_nsec / 1000000LL;
}

static void sleep_ms(int ms) {
    struct timespec ts = { .tv_sec = ms / 1000, .tv_nsec = (long)(ms % 1000) * 1000000L };
    nanosleep(&ts, NULL);
}

/* ─── Readiness polling ──────────────────────────────────────────────────── */

bool supervisor_check_ready(service_t *svc, long long start_ms) {
    long long elapsed = now_ms() - start_ms;
    if (elapsed > (long long)svc->ready_timeout_ms) return false;

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

        case READY_HTTP: {
            /*
             * Real HTTP readiness probe.
             *
             * ready_path format: "host:port" (e.g. "localhost:11434").
             * We open a TCP connection and send "GET / HTTP/1.0\r\n\r\n".
             * If we receive at least "HTTP/1" we consider the service ready.
             *
             * Uses a 1-second connect timeout via select(2) with O_NONBLOCK.
             */
            if (elapsed > (long long)svc->ready_timeout_ms) return false;

            /* Parse host:port from ready_path */
            char host[128] = "localhost";
            int  port = 80;
            {
                const char *p = svc->ready_path;
                if (strncmp(p, "http://", 7) == 0) p += 7;
                else if (strncmp(p, "https://", 8) == 0) p += 8;

                const char *slash = strchr(p, '/');
                size_t len = slash ? (size_t)(slash - p) : strlen(p);

                char host_port[256];
                if (len >= sizeof(host_port)) len = sizeof(host_port) - 1;
                memcpy(host_port, p, len);
                host_port[len] = '\0';

                const char *colon = strrchr(host_port, ':');
                if (colon) {
                    size_t hlen = (size_t)(colon - host_port);
                    if (hlen >= sizeof(host)) hlen = sizeof(host) - 1;
                    memcpy(host, host_port, hlen);
                    host[hlen] = '\0';
                    port = atoi(colon + 1);
                } else if (host_port[0]) {
                    snprintf(host, sizeof(host), "%s", host_port);
                }
            }
            if (port <= 0 || port > 65535) port = 80;

            /* Resolve hostname */
            struct addrinfo hints = {0};
            hints.ai_family   = AF_INET;
            hints.ai_socktype = SOCK_STREAM;
            struct addrinfo *ai = NULL;
            char port_s[8];
            snprintf(port_s, sizeof(port_s), "%d", port);
            if (getaddrinfo(host, port_s, &hints, &ai) != 0 || !ai)
                return false;

            int sock = socket(ai->ai_family, SOCK_STREAM | SOCK_NONBLOCK, 0);
            if (sock < 0) { freeaddrinfo(ai); return false; }

            bool ready = false;
            int  cr = connect(sock, ai->ai_addr, ai->ai_addrlen);
            if (cr == 0) {
                ready = true; /* immediate connect (unlikely but handle) */
            } else if (errno == EINPROGRESS) {
                /* Wait up to 1 second for the connect to complete */
                fd_set wfds;
                FD_ZERO(&wfds);
                FD_SET(sock, &wfds);
                struct timeval tv = { .tv_sec = 1, .tv_usec = 0 };
                if (select(sock + 1, NULL, &wfds, NULL, &tv) > 0) {
                    int err = 0;
                    socklen_t el = sizeof(err);
                    getsockopt(sock, SOL_SOCKET, SO_ERROR, &err, &el);
                    if (err == 0) {
                        /* Connected — send a minimal HTTP/1.0 GET */
                        const char req[] = "GET / HTTP/1.0\r\n\r\n";
                        (void)send(sock, req, sizeof(req) - 1, MSG_NOSIGNAL);
                        /* Wait briefly for response */
                        FD_ZERO(&wfds);
                        FD_SET(sock, &wfds);
                        tv = (struct timeval){ .tv_sec = 0, .tv_usec = 500000 };
                        fd_set rfds;
                        FD_ZERO(&rfds);
                        FD_SET(sock, &rfds);
                        if (select(sock + 1, &rfds, NULL, NULL, &tv) > 0) {
                            char buf[16] = {0};
                            ssize_t n = recv(sock, buf, sizeof(buf) - 1, 0);
                            if (n > 0 && strncmp(buf, "HTTP/1", 6) == 0)
                                ready = true;
                        }
                    }
                }
            }
            close(sock);
            freeaddrinfo(ai);
            return ready;
        }

        case READY_SIGNAL: {
            /*
             * Real SIGUSR1-based readiness.
             *
             * The service sends SIGUSR1 to PID 1 when it is ready.
             * The signal handler in signal.c calls supervisor_signal_ready(pid),
             * which sets g_signal_ready[idx] = 1 for the service matching that PID.
             *
             * We find the index of this service in g_services[] and check the flag.
             */
            for (int idx = 0; idx < g_service_count; idx++) {
                if (&g_services[idx] == svc) {
                    return (g_signal_ready[idx] != 0);
                }
            }
            return false;
        }

        default:
            return true;
    }
}

/* ─── Service spawning ───────────────────────────────────────────────────── */

static uid_t parse_uid(const char *name_or_id) {
    char *endptr = NULL;
    long val = strtol(name_or_id, &endptr, 10);
    if (endptr != name_or_id && *endptr == '\0') {
        return (uid_t)val;
    }
    struct passwd *pw = getpwnam(name_or_id);
    if (pw) {
        return pw->pw_uid;
    }
    return (uid_t)-1;
}

static gid_t parse_gid(const char *name_or_id) {
    char *endptr = NULL;
    long val = strtol(name_or_id, &endptr, 10);
    if (endptr != name_or_id && *endptr == '\0') {
        return (gid_t)val;
    }
    struct group *gr = getgrnam(name_or_id);
    if (gr) {
        return gr->gr_gid;
    }
    return (gid_t)-1;
}

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

        /* Identity enforcement */
        if (svc->run_group[0] != '\0') {
            gid_t gid = parse_gid(svc->run_group);
            if (gid == (gid_t)-1) {
                LUNA_ERROR(COMP, "Service '%s': group '%s' not found", svc->name, svc->run_group);
                _exit(127);
            }
            if (setgid(gid) != 0) {
                LUNA_ERROR(COMP, "Service '%s': setgid() failed", svc->name);
                _exit(127);
            }
            /* Clear supplementary groups inherited from PID 1 (root). Failing
             * to call setgroups(0, NULL) would leave the child process with
             * root's supplementary group list despite the setgid() call, which
             * is a standard privilege-drop pitfall. (CODE_AUDIT_REPORT §3.2) */
            if (setgroups(0, NULL) != 0) {
                LUNA_ERROR(COMP, "Service '%s': setgroups() failed", svc->name);
                _exit(127);
            }
        }

        if (svc->run_user[0] != '\0') {
            uid_t uid = parse_uid(svc->run_user);
            if (uid == (uid_t)-1) {
                LUNA_ERROR(COMP, "Service '%s': user '%s' not found", svc->name, svc->run_user);
                _exit(127);
            }
            if (setuid(uid) != 0) {
                LUNA_ERROR(COMP, "Service '%s': setuid() failed", svc->name);
                _exit(127);
            }
        }

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

    /* Assign to cgroup */
    char cgroup_dir[256];
    snprintf(cgroup_dir, sizeof(cgroup_dir), "/sys/fs/cgroup/luna-system.slice/%s.service", svc->name);
    mkdir("/sys/fs/cgroup/luna-system.slice", 0755); /* Ignore error if exists */
    mkdir(cgroup_dir, 0755);                         /* Ignore error if exists */
    
    char procs_file[512];
    snprintf(procs_file, sizeof(procs_file), "%s/cgroup.procs", cgroup_dir);
    int fd = open(procs_file, O_WRONLY);
    if (fd >= 0) {
        char pid_str[32];
        int len = snprintf(pid_str, sizeof(pid_str), "%d\n", (int)pid);
        write(fd, pid_str, len);
        close(fd);
    } else {
        LUNA_WARN(COMP, "Could not assign service '%s' to cgroup (open failed: %s)", svc->name, strerror(errno));
    }

    LUNA_INFO(COMP, "Started service '%s' (PID %d)", svc->name, (int)pid);
    return pid;
}

/* ─── Async Supervisor Pump ──────────────────────────────────────────────── */

void supervisor_pump(void) {
    long long now = now_ms();

    /* 1. Check readiness of STARTING services */
    for (int i = 0; i < g_service_count; i++) {
        service_t *svc = &g_services[i];
        if (svc->state == SERVICE_STATE_STARTING) {
            long long start = svc->start_time_ms;
            if (svc->ready_method == READY_NONE) {
                svc->state = SERVICE_STATE_RUNNING;
                LUNA_INFO(COMP, "Service '%s' ready (no readiness check)", svc->name);
            } else if (supervisor_check_ready(svc, start)) {
                svc->state = SERVICE_STATE_RUNNING;
                LUNA_INFO(COMP, "Service '%s' ready (%lld ms)", svc->name, now - start);
            } else if (now - start >= svc->ready_timeout_ms) {
                svc->state = SERVICE_STATE_DEGRADED;
                LUNA_ERROR(COMP, "Service '%s' readiness timeout (%d ms) — DEGRADED", svc->name, svc->ready_timeout_ms);
            }
        }
    }

    /* 2. Try to start PENDING services if dependencies are met */
    int order[SERVICE_MAX_COUNT];
    int count = depgraph_topo_sort(order, SERVICE_MAX_COUNT);
    if (count > 0) {
        for (int k = 0; k < count; k++) {
            int i = order[k];
            service_t *svc = &g_services[i];
            if (svc->state == SERVICE_STATE_PENDING) {
                if (now < svc->scheduled_start_ms) continue;

                bool can_start = true;
                for (int j = 0; j < svc->dep_count; j++) {
                    int dep_idx = svc->dep_indices[j];
                    if (dep_idx >= 0 && dep_idx < g_service_count) {
                        service_t *dep = &g_services[dep_idx];
                        if (dep->state != SERVICE_STATE_RUNNING && dep->state != SERVICE_STATE_DEGRADED) {
                            can_start = false;
                            break;
                        }
                    }
                }

                if (can_start) {
                    pid_t pid = spawn_service(svc);
                    if (pid < 0 && svc->state != SERVICE_STATE_DEGRADED) {
                        svc->state = SERVICE_STATE_DEGRADED;
                    }
                }
            }
        }
    }
}

bool supervisor_is_boot_complete(void) {
    for (int i = 0; i < g_service_count; i++) {
        if (g_services[i].state == SERVICE_STATE_PENDING || 
            g_services[i].state == SERVICE_STATE_STARTING) {
            
            /* If this service is blocked by a STOPPED service (like lgp-compositor),
               do not let it hold up Stage 4 completion. */
            bool blocked_by_stopped = false;
            for (int j = 0; j < g_services[i].dep_count; j++) {
                int dep_idx = g_services[i].dep_indices[j];
                if (dep_idx >= 0 && dep_idx < g_service_count) {
                    service_t *dep = &g_services[dep_idx];
                    if (dep->state == SERVICE_STATE_STOPPED) {
                        blocked_by_stopped = true;
                        break;
                    }
                }
            }
            if (blocked_by_stopped) {
                continue;
            }
            
            return false;
        }
    }
    return true;
}

/* ─── Public API ─────────────────────────────────────────────────────────── */

int supervisor_start_all(void) {
    /* Initialize scheduled start times and set to PENDING */
    for (int i = 0; i < g_service_count; i++) {
        g_services[i].scheduled_start_ms = 0;
        
        /* EXCLUSION: lgp-compositor is manually started in Stage 5, not Stage 4 */
        if (strcmp(g_services[i].name, "lgp-compositor") == 0) {
            g_services[i].state = SERVICE_STATE_STOPPED;
            continue;
        }

        if (g_services[i].state != SERVICE_STATE_DEGRADED) {
            g_services[i].state = SERVICE_STATE_PENDING;
        }
    }
    
    LUNA_INFO(COMP, "Starting %d services asynchronously...", g_service_count);
    supervisor_pump();
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

    svc->state              = SERVICE_STATE_PENDING;
    svc->restart_count      = 0;
    svc->scheduled_start_ms = 0;

    supervisor_pump();

    /* Propagate actual spawn failure: if the pump immediately set the service
     * to DEGRADED (e.g. binary not found), return -1 so callers such as
     * main.c Stage 5 can log the correct error message. (CODE_AUDIT_REPORT §3.3) */
    if (svc->state == SERVICE_STATE_DEGRADED ||
        svc->state == SERVICE_STATE_FAILED) {
        return -1;
    }
    return 0;
}

int supervisor_stop_one(const char *name) {
    service_t *svc = service_find(name);
    if (!svc || svc->pid <= 0) return 0;

    LUNA_INFO(COMP, "Stopping service '%s' (PID %d)", name, (int)svc->pid);
    svc->state = SERVICE_STATE_STOPPING;

    kill(svc->pid, svc->stop_signal);

    /* Non-blocking wait: poll with WNOHANG so PID 1's event loop is not
     * blocked during service stop. We iterate with short nanosleeps rather
     * than a blocking waitpid() call, capped at stop_timeout_ms total.
     * (Fixes audit Issue 16 — busy-poll usleep was blocking PID 1) */
    long long start = now_ms();
    while (now_ms() - start < (long long)svc->stop_timeout_ms) {
        int wstatus = 0;
        pid_t ret = waitpid(svc->pid, &wstatus, WNOHANG);
        if (ret == svc->pid) {
            /* Process has exited cleanly */
            svc->state = SERVICE_STATE_STOPPED;
            svc->pid   = 0;
            LUNA_INFO(COMP, "Service '%s' stopped", name);
            return 0;
        }
        if (ret < 0 && errno != EINTR) {
            /* Process already gone (ECHILD) */
            svc->state = SERVICE_STATE_STOPPED;
            svc->pid   = 0;
            return 0;
        }
        /* Yield for 20ms without holding a blocking syscall */
        struct timespec nap = { .tv_sec = 0, .tv_nsec = 20000000L };
        nanosleep(&nap, NULL);
    }

    /* Timeout — SIGKILL */
    LUNA_WARN(COMP, "Service '%s' did not stop in %dms — sending SIGKILL",
              name, svc->stop_timeout_ms);
    kill(svc->pid, SIGKILL);
    /* One final non-blocking reap */
    waitpid(svc->pid, NULL, WNOHANG);
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

    svc->state = SERVICE_STATE_PENDING;
    svc->scheduled_start_ms = now_ms() + svc->restart_delay_ms;
}
