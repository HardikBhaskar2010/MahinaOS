/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * service.c — Service File Parser
 * Authority: Volume II / 04_init_system.md §Service File Format
 */

#include "service.h"
#include "log.h"
#include "toml.h"

#include <dirent.h>
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>

#define COMP "service"

/* Global service table */
service_t g_services[SERVICE_MAX_COUNT];
int       g_service_count = 0;

/* ─── Helpers ────────────────────────────────────────────────────────────── */

static restart_policy_t parse_restart_policy(const char *s) {
    if (strcmp(s, "always")     == 0) return RESTART_ALWAYS;
    if (strcmp(s, "on-failure") == 0) return RESTART_ON_FAILURE;
    return RESTART_NEVER;
}

static ready_method_t parse_ready_method(const char *s) {
    if (strcmp(s, "file")   == 0) return READY_FILE;
    if (strcmp(s, "socket") == 0) return READY_SOCKET;
    if (strcmp(s, "http")   == 0) return READY_HTTP;
    if (strcmp(s, "signal") == 0) return READY_SIGNAL;
    return READY_NONE;
}

static int parse_signal_name(const char *s) {
    if (strcmp(s, "SIGTERM") == 0) return SIGTERM;
    if (strcmp(s, "SIGKILL") == 0) return SIGKILL;
    if (strcmp(s, "SIGHUP")  == 0) return SIGHUP;
    if (strcmp(s, "SIGQUIT") == 0) return SIGQUIT;
    return SIGTERM; /* default per spec */
}

/* ─── Single service file parser ─────────────────────────────────────────── */

int service_load_one(const char *path, service_t *out) {
    toml_error_t err;
    toml_doc_t  *doc = toml_load_file(path, &err);

    if (!doc) {
        LUNA_ERROR(COMP, "Failed to parse %s: %s", path, toml_strerror(err));
        return -1;
    }

    /* Zero-initialize output struct */
    memset(out, 0, sizeof(*out));
    out->state             = SERVICE_STATE_PENDING;
    out->restart_policy    = RESTART_ON_FAILURE;
    out->restart_attempts  = 3;
    out->restart_delay_ms  = 1000;
    out->ready_method      = READY_NONE;
    out->ready_timeout_ms  = 5000;
    out->stop_signal       = SIGTERM;
    out->stop_timeout_ms   = 3000;

    toml_value_t v = {0};

#define GET_STR(sec, key, dst, required)                                        \
    do {                                                                        \
        if (toml_get(doc, (sec), (key), &v) == TOML_OK &&                      \
            v.type == TOML_TYPE_STRING) {                                       \
            strncpy((dst), v.v.str, sizeof(dst) - 1);                          \
        } else if (required) {                                                  \
            LUNA_ERROR(COMP, "%s: missing required field [%s].%s", path,       \
                       (sec) ? (sec) : "top", (key));                          \
            toml_free(doc);                                                     \
            return -1;                                                          \
        }                                                                       \
    } while (0)

#define GET_INT(sec, key, dst)                                                  \
    do {                                                                        \
        if (toml_get(doc, (sec), (key), &v) == TOML_OK &&                      \
            v.type == TOML_TYPE_INTEGER) {                                      \
            (dst) = (int)v.v.integer;                                           \
        }                                                                       \
    } while (0)

    /* Required fields */
    GET_STR("service", "name",        out->name,        true);
    GET_STR("service", "binary",      out->binary,      true);
    GET_STR("service", "description", out->description, false);

    /* Optional: workdir */
    GET_STR("service", "workdir", out->workdir, false);

    /* Optional: args array */
    if (toml_get(doc, "service", "args", &v) == TOML_OK &&
        v.type == TOML_TYPE_ARRAY) {
        out->arg_count = (int)v.v.array.count;
        for (int i = 0; i < out->arg_count && i < SERVICE_MAX_ARGS; i++) {
            strncpy(out->args[i], v.v.array.items[i], SERVICE_MAX_ARG_LEN - 1);
        }
    }

    /* Optional: identity */
    GET_STR("service.identity", "user",  out->run_user,  false);
    GET_STR("service.identity", "group", out->run_group, false);

    /* Dependencies: after = ["name1", "name2"] */
    if (toml_get_nested(doc, "service", "depends", "after", &v) == TOML_OK &&
        v.type == TOML_TYPE_ARRAY) {
        out->after_count = (int)v.v.array.count;
        for (int i = 0; i < out->after_count && i < SERVICE_MAX_DEPS; i++) {
            strncpy(out->after[i], v.v.array.items[i], SERVICE_MAX_NAME_LEN - 1);
        }
    }

    /* Dependencies: before = ["name1"] */
    if (toml_get_nested(doc, "service", "depends", "before", &v) == TOML_OK &&
        v.type == TOML_TYPE_ARRAY) {
        out->before_count = (int)v.v.array.count;
        for (int i = 0; i < out->before_count && i < SERVICE_MAX_DEPS; i++) {
            strncpy(out->before[i], v.v.array.items[i], SERVICE_MAX_NAME_LEN - 1);
        }
    }

    /* Restart policy */
    char policy_str[32] = "on-failure";
    if (toml_get_nested(doc, "service", "restart", "policy", &v) == TOML_OK &&
        v.type == TOML_TYPE_STRING)
        strncpy(policy_str, v.v.str, sizeof(policy_str) - 1);
    out->restart_policy = parse_restart_policy(policy_str);
    GET_INT("service.restart", "attempts", out->restart_attempts);
    GET_INT("service.restart", "delay_ms", out->restart_delay_ms);

    /* Readiness */
    char method_str[32] = "none";
    if (toml_get_nested(doc, "service", "ready", "method", &v) == TOML_OK &&
        v.type == TOML_TYPE_STRING)
        strncpy(method_str, v.v.str, sizeof(method_str) - 1);
    out->ready_method = parse_ready_method(method_str);
    GET_STR("service.ready", "path",       out->ready_path,      false);
    GET_INT("service.ready", "timeout_ms", out->ready_timeout_ms);

    /* Stop behavior */
    char stop_sig_str[16] = "SIGTERM";
    if (toml_get_nested(doc, "service", "stop", "signal", &v) == TOML_OK &&
        v.type == TOML_TYPE_STRING)
        strncpy(stop_sig_str, v.v.str, sizeof(stop_sig_str) - 1);
    out->stop_signal = parse_signal_name(stop_sig_str);
    GET_INT("service.stop", "timeout_ms", out->stop_timeout_ms);

#undef GET_STR
#undef GET_INT

    toml_free(doc);
    LUNA_DEBUG(COMP, "Loaded service: %s (%s)", out->name, out->binary);
    return 0;
}

/* ─── Directory scanner ──────────────────────────────────────────────────── */

int service_load_all(const char *services_dir) {
    DIR *dir = opendir(services_dir);
    if (!dir) {
        LUNA_ERROR(COMP, "Cannot open services dir %s: %s",
                   services_dir, strerror(errno));
        return -1;
    }

    g_service_count = 0;
    struct dirent *ent;

    while ((ent = readdir(dir)) != NULL) {
        /* Only .toml files */
        const char *name = ent->d_name;
        size_t      nlen = strlen(name);
        if (nlen < 6 || strcmp(name + nlen - 5, ".toml") != 0) continue;

        char path[512];
        snprintf(path, sizeof(path), "%s/%s", services_dir, name);

        if (g_service_count >= SERVICE_MAX_COUNT) {
            LUNA_ERROR(COMP, "Service table full — skipping %s", path);
            break;
        }

        service_t svc = {0};
        if (service_load_one(path, &svc) < 0) continue;

        /* Check for duplicate names */
        bool dup = false;
        for (int i = 0; i < g_service_count; i++) {
            if (strcmp(g_services[i].name, svc.name) == 0) {
                LUNA_ERROR(COMP, "Duplicate service name '%s' in %s — skipped",
                           svc.name, path);
                dup = true;
                break;
            }
        }
        if (!dup) g_services[g_service_count++] = svc;
    }

    closedir(dir);
    LUNA_INFO(COMP, "Loaded %d service definitions from %s",
              g_service_count, services_dir);
    return g_service_count;
}

/* ─── Lookup ─────────────────────────────────────────────────────────────── */

service_t *service_find(const char *name) {
    for (int i = 0; i < g_service_count; i++) {
        if (strcmp(g_services[i].name, name) == 0) return &g_services[i];
    }
    return NULL;
}

service_t *service_find_by_pid(pid_t pid) {
    if (pid <= 0) return NULL;
    for (int i = 0; i < g_service_count; i++) {
        if (g_services[i].pid == pid) return &g_services[i];
    }
    return NULL;
}

const char *service_state_name(service_state_t state) {
    switch (state) {
        case SERVICE_STATE_PENDING:  return "PENDING";
        case SERVICE_STATE_STARTING: return "STARTING";
        case SERVICE_STATE_RUNNING:  return "RUNNING";
        case SERVICE_STATE_DEGRADED: return "DEGRADED";
        case SERVICE_STATE_STOPPING: return "STOPPING";
        case SERVICE_STATE_STOPPED:  return "STOPPED";
        case SERVICE_STATE_FAILED:   return "FAILED";
        default:                     return "UNKNOWN";
    }
}
