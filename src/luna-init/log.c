/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * log.c — Mahina Early Logger Implementation
 * Authority: Volume II / 11_logging.md
 * Authority: Volume VI / 01_coding_standards.md
 *
 * Design notes:
 *   - No external library dependencies. Uses only Linux syscalls and libc.
 *   - Each write() call is atomic for entries < PIPE_BUF (4096 bytes).
 *   - Boot log timestamps: CLOCK_MONOTONIC_RAW ms from PID 1 start.
 *   - Runtime log timestamps: clock_gettime(CLOCK_REALTIME) ISO 8601.
 *   - fsync() used only for FATAL entries (boot-critical events per Vol II/11).
 */

#include "log.h"

#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

/* ─── Internal state ─────────────────────────────────────────────────────── */

/* Log file descriptors. -1 = not open. Writes fall back to stderr if closed. */
static int g_boot_fd    = -1;
static int g_runtime_fd = -1;

/* Runtime log path stored at init time, opened at luna_log_switch_to_runtime(). */
static char g_runtime_log_path[256] = {0};

/* true after luna_log_switch_to_runtime() */
static bool g_runtime_mode = false;

/* Current minimum log level (default: INFO during boot) */
static luna_log_level_t g_min_level = LOG_INFO;

/* Current boot stage (set by luna_log_set_stage) */
static int g_current_stage = LUNA_STAGE_UNKNOWN;

/* Monotonic start time captured in luna_log_init() */
static struct timespec g_start_time = {0};

/* Level name strings (index matches luna_log_level_t) */
static const char * const LEVEL_NAMES[] = {
    "FATAL", "ERROR", "WARN ", "INFO ", "DEBUG", "TRACE"
};

/* Maximum single log entry size (must be < PIPE_BUF for atomic write) */
#define LOG_ENTRY_MAX 4000

/* ─── Internal helpers ───────────────────────────────────────────────────── */

/*
 * log_get_boot_ms() — Milliseconds since CLOCK_MONOTONIC_RAW at init.
 * Used for boot log timestamps. Monotonic, unaffected by NTP adjustments.
 */
static uint64_t log_get_boot_ms(void) {
    struct timespec now = {0};
    clock_gettime(CLOCK_MONOTONIC_RAW, &now);

    int64_t diff_s  = (int64_t)(now.tv_sec  - g_start_time.tv_sec);
    int64_t diff_ns = (int64_t)(now.tv_nsec - g_start_time.tv_nsec);

    return (uint64_t)((diff_s * 1000) + (diff_ns / 1000000));
}

/*
 * log_get_iso8601() — Write current UTC time in ISO 8601 format.
 * Format: YYYY-MM-DDTHH:MM:SS.mmmZ (Volume II / 11_logging.md §Log Format)
 * buf must be at least 28 bytes.
 */
static void log_get_iso8601(char *buf, size_t buf_size) {
    struct timespec ts = {0};
    clock_gettime(CLOCK_REALTIME, &ts);

    struct tm tm_utc = {0};
    gmtime_r(&ts.tv_sec, &tm_utc);

    int ms = (int)(ts.tv_nsec / 1000000);

    snprintf(buf, buf_size,
             "%04d-%02d-%02dT%02d:%02d:%02d.%03dZ",
             tm_utc.tm_year + 1900,
             tm_utc.tm_mon  + 1,
             tm_utc.tm_mday,
             tm_utc.tm_hour,
             tm_utc.tm_min,
             tm_utc.tm_sec,
             ms);
}

/*
 * log_write_entry() — Format and write a single log entry atomically.
 *
 * Boot mode format  : [TIMESTAMP_MS] [STAGE] [COMPONENT] [LEVEL] message\n
 * Runtime mode format: [ISO8601] [LEVEL] [COMPONENT] message\n
 */
static void log_write_entry(luna_log_level_t level, int stage,
                             const char *component, const char *message) {
    char entry[LOG_ENTRY_MAX];
    int  len = 0;
    int  fd  = -1;

    if (!g_runtime_mode) {
        /* Boot log format */
        uint64_t boot_ms = log_get_boot_ms();
        int      s       = (stage == -1) ? g_current_stage : stage;

        if (s == LUNA_STAGE_UNKNOWN) {
            len = snprintf(entry, sizeof(entry),
                           "[%06" PRIu64 "] [?] [%-16s] [%s] %s\n",
                           boot_ms, component,
                           LEVEL_NAMES[level], message);
        } else {
            len = snprintf(entry, sizeof(entry),
                           "[%06" PRIu64 "] [%d] [%-16s] [%s] %s\n",
                           boot_ms, s, component,
                           LEVEL_NAMES[level], message);
        }
        fd = (g_boot_fd >= 0) ? g_boot_fd : STDERR_FILENO;
    } else {
        /* Runtime log format */
        char ts[32];
        log_get_iso8601(ts, sizeof(ts));

        len = snprintf(entry, sizeof(entry),
                       "[%s] [%s] [%-16s] %s\n",
                       ts, LEVEL_NAMES[level], component, message);
        fd = (g_runtime_fd >= 0) ? g_runtime_fd : STDERR_FILENO;
    }

    if (len <= 0) {
        return; /* snprintf failed — nothing to write */
    }

    /* Clamp to buffer size (should never happen with LOG_ENTRY_MAX) */
    if (len >= (int)sizeof(entry)) {
        len = (int)sizeof(entry) - 1;
        entry[len - 1] = '\n';
    }

    /* Single atomic write (< PIPE_BUF = 4096 on Linux) */
    (void)write(fd, entry, (size_t)len);

    /* fsync only for FATAL entries — performance vs durability tradeoff */
    /* (Volume II / 11_logging.md §Log File Implementation) */
    if (level == LOG_FATAL && fd != STDERR_FILENO) {
        (void)fsync(fd);
    }
}

/* ─── Public API ─────────────────────────────────────────────────────────── */

int luna_log_init(const char *boot_log_path, const char *runtime_log_path) {
    /* Capture start time for boot timestamps */
    clock_gettime(CLOCK_MONOTONIC_RAW, &g_start_time);

    /* Open boot log (create if absent, append if exists) */
    g_boot_fd = open(boot_log_path,
                     O_WRONLY | O_CREAT | O_APPEND | O_CLOEXEC,
                     0644);
    if (g_boot_fd < 0) {
        /* Fall back to stderr — cannot log this failure to the log itself */
        int saved_errno = errno;
        dprintf(STDERR_FILENO,
                "[luna-init] [FATAL] Failed to open boot log '%s': %s\n",
                boot_log_path, strerror(saved_errno));
        /* Not fatal — continue with stderr logging */
    }

    /* Store runtime log path for later use in luna_log_switch_to_runtime(). */
    /* The file is not opened yet — only opened after the boot log is closed   */
    /* (Stage 5 graphics transition per Volume II / 11_logging.md).            */
    if (runtime_log_path) {
        strncpy(g_runtime_log_path, runtime_log_path,
                sizeof(g_runtime_log_path) - 1);
    }

    return 0;
}

void luna_log(luna_log_level_t level, int stage,
              const char *component, const char *fmt, ...) {
    /* Discard entries below the minimum level */
    if (level > g_min_level) {
        return;
    }

    /* Format the message */
    char message[LOG_ENTRY_MAX - 128]; /* leave room for timestamp/level prefix */
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(message, sizeof(message), fmt, ap);
    va_end(ap);

    log_write_entry(level, stage, component, message);
}

void luna_log_signal_safe(const char *msg) {
    /*
     * Async-signal-safe logging path. Only uses write(2).
     * Called from the signalfd handler in signal.c.
     * Pre-formatted by the caller.
     */
    static const char prefix[] = "[luna-init] SIGNAL: ";
    int fd = (g_boot_fd >= 0) ? g_boot_fd : STDERR_FILENO;

    (void)write(fd, prefix, sizeof(prefix) - 1);
    (void)write(fd, msg, strlen(msg));
    (void)write(fd, "\n", 1);
}

void luna_log_set_stage(int stage) {
    g_current_stage = stage;
}

void luna_log_set_level(luna_log_level_t min_level) {
    g_min_level = min_level;
}

void luna_log_switch_to_runtime(void) {
    g_runtime_mode = true;
    g_min_level    = LOG_WARN; /* runtime default: WARN per Volume II/11 */

    if (g_boot_fd >= 0) {
        (void)fsync(g_boot_fd);
        (void)close(g_boot_fd);
        g_boot_fd = -1;
    }

    /* Open the runtime log file now that boot logging is done.
     * This was the critical bug identified in CODE_AUDIT_REPORT §2:
     * g_runtime_fd was declared but never assigned, causing all post-boot
     * log entries to silently fall through to STDERR_FILENO. */
    if (g_runtime_log_path[0] != '\0') {
        g_runtime_fd = open(g_runtime_log_path,
                            O_WRONLY | O_CREAT | O_APPEND | O_CLOEXEC,
                            0644);
        if (g_runtime_fd < 0) {
            /* Cannot log this to the just-closed boot log; use stderr. */
            dprintf(STDERR_FILENO,
                    "[luna-init] [ERROR] Failed to open runtime log '%s': %s\n",
                    g_runtime_log_path, strerror(errno));
        }
    }
}



uint64_t luna_log_boot_time_ms(void) {
    return log_get_boot_ms();
}

void luna_log_close(void) {
    if (g_boot_fd >= 0) {
        (void)fsync(g_boot_fd);
        (void)close(g_boot_fd);
        g_boot_fd = -1;
    }
    if (g_runtime_fd >= 0) {
        (void)fsync(g_runtime_fd);
        (void)close(g_runtime_fd);
        g_runtime_fd = -1;
    }
}
