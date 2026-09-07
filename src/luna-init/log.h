/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef MAHINA_LUNA_LOG_H
#define MAHINA_LUNA_LOG_H

/*
 * luna_log.h — Mahina Early Logger
 * Authority: Volume II / 11_logging.md
 * Authority: Volume VI / 01_coding_standards.md
 *
 * This is the sole logging implementation for luna-init. No external logging
 * library is used. This file is shared by all Stage 0 system components.
 *
 * Boot log format  : [TIMESTAMP_MS] [STAGE] [COMPONENT] [LEVEL] message
 * Runtime log format: [YYYY-MM-DDTHH:MM:SS.mmmZ] [LEVEL] [COMPONENT] message
 *
 * Boot log path    : /var/log/luna-init/boot.log
 * Runtime log path : /var/log/luna-init/runtime.log
 *
 * Each log entry is a single write() call. Entries must be < 4096 bytes
 * for atomic write safety (Volume II / 11_logging.md §Technical Details).
 *
 * Default log level per component (Volume II / 11_logging.md §Log Levels):
 *   luna-init (boot)    : LOG_INFO
 *   luna-init (runtime) : LOG_WARN
 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>

/* ─── Log Levels ─────────────────────────────────────────────────────────── */

typedef enum {
    LOG_FATAL = 0,  /* Process cannot continue — imminent crash */
    LOG_ERROR = 1,  /* Significant failure — component may be degraded */
    LOG_WARN  = 2,  /* Unexpected condition — system continues normally */
    LOG_INFO  = 3,  /* Normal significant events */
    LOG_DEBUG = 4,  /* Detailed operational information */
    LOG_TRACE = 5,  /* Extremely detailed — function call level */
} luna_log_level_t;

/* ─── Boot stage tag (used in boot log only) ─────────────────────────────── */

/* Current boot stage (0–7). Set by luna-init main as each stage begins. */
#define LUNA_STAGE_UNKNOWN  (-1)
#define LUNA_STAGE_FIRMWARE   0
#define LUNA_STAGE_KERNEL     1
#define LUNA_STAGE_FILESYSTEM 2
#define LUNA_STAGE_HOOKS      3
#define LUNA_STAGE_SERVICES   4
#define LUNA_STAGE_GRAPHICS   5
#define LUNA_STAGE_SHELL      6
#define LUNA_STAGE_READY      7

/* ─── Public API ─────────────────────────────────────────────────────────── */

/*
 * luna_log_init() — Open log files and record start timestamp.
 *
 * Must be called before any luna_log() invocation. Called once at PID 1 start.
 * boot_log_path    : absolute path to boot log (e.g. "/var/log/luna-init/boot.log")
 * runtime_log_path : absolute path to runtime log (e.g. "/var/log/luna-init/runtime.log")
 *
 * Returns 0 on success, -errno on failure.
 * FATAL: if boot log cannot be opened, luna-init falls back to stderr.
 */
int luna_log_init(const char *boot_log_path, const char *runtime_log_path);

/*
 * luna_log() — Write a log entry.
 *
 * Thread-safe via atomic write(). Not async-signal-safe — do not call from
 * signal handlers. Use luna_log_signal_safe() from signal context.
 *
 * level     : log severity level
 * stage     : current boot stage (LUNA_STAGE_* constant), or LUNA_STAGE_UNKNOWN
 * component : component name string (e.g. "luna-init", "supervisor")
 * fmt       : printf-style format string
 */
void luna_log(luna_log_level_t level, int stage,
              const char *component, const char *fmt, ...)
    __attribute__((format(printf, 4, 5)));

/*
 * luna_log_signal_safe() — Write a pre-formatted message from signal context.
 *
 * Only uses async-signal-safe functions (write(2)). The message must be a
 * complete, pre-formatted string. Format: "[luna-init] SIGNAL: <msg>\n"
 */
void luna_log_signal_safe(const char *msg);

/*
 * luna_log_set_stage() — Update the current boot stage written to log entries.
 *
 * Call this at the beginning of each boot stage in luna-init main.
 */
void luna_log_set_stage(int stage);

/*
 * luna_log_set_level() — Set the minimum level for log output.
 *
 * Entries below this level are silently discarded. Default: LOG_INFO for boot.
 */
void luna_log_set_level(luna_log_level_t min_level);

/*
 * luna_log_switch_to_runtime() — Close boot log, open runtime log.
 *
 * Called after Stage 7 (boot complete). After this call:
 *   - boot.log is closed (never written to again)
 *   - runtime.log is opened (wall-clock ISO 8601 timestamps)
 *   - Log level resets to LOG_WARN (Volume II / 11_logging.md §Log Levels)
 */
void luna_log_switch_to_runtime(void);

/*
 * luna_log_boot_time_ms() — Return milliseconds since PID 1 start.
 *
 * Used to compute boot log timestamps. Based on CLOCK_MONOTONIC_RAW.
 */
uint64_t luna_log_boot_time_ms(void);

/*
 * luna_log_close() — Flush and close all log files.
 *
 * Called during orderly shutdown. After this call, luna_log() writes to stderr.
 */
void luna_log_close(void);

/* ─── Convenience macros ─────────────────────────────────────────────────── */

/* Use the current stage stored internally — avoids passing stage everywhere */
#define LUNA_FATAL(component, ...) \
    luna_log(LOG_FATAL, -1, (component), __VA_ARGS__)

#define LUNA_ERROR(component, ...) \
    luna_log(LOG_ERROR, -1, (component), __VA_ARGS__)

#define LUNA_WARN(component, ...) \
    luna_log(LOG_WARN, -1, (component), __VA_ARGS__)

#define LUNA_INFO(component, ...) \
    luna_log(LOG_INFO, -1, (component), __VA_ARGS__)

#define LUNA_DEBUG(component, ...) \
    luna_log(LOG_DEBUG, -1, (component), __VA_ARGS__)

#endif /* MAHINA_LUNA_LOG_H */
