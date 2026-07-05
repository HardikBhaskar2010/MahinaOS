/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

/*
 * main.c — lpkg package manager CLI implementation
 */

#include "pkg.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <errno.h>

#define MAX_DB_PKGS 128
#define INDEX_PATH "/var/lib/lpkg/index.toml"

static lpkg_t g_db[MAX_DB_PKGS];
static int    g_db_count = 0;

static void usage(void) {
    fprintf(stderr,
        "lpkg — Mahina package manager\n"
        "\n"
        "Usage:\n"
        "  lpkg install <package-name | local-file.lpkg>\n"
        "  lpkg remove <package-name>\n"
        "  lpkg list\n"
        "  lpkg update\n");
}

static int run_curl(const char *url, const char *dest_path) {
    pid_t pid = fork();
    if (pid == 0) {
        char *argv[] = {
            "curl",
            "-s",
            "-L",
            "-o",
            (char *)dest_path,
            (char *)url,
            NULL
        };
        execvp("curl", argv);
        _exit(127);
    } else if (pid < 0) {
        return -1;
    }
    int status;
    if (waitpid(pid, &status, 0) < 0) return -1;
    return (WIFEXITED(status) && WEXITSTATUS(status) == 0) ? 0 : -1;
}

static int get_tar_file_list(const char *lpkg_path, lpkg_t *pkg) {
    int pfd[2];
    if (pipe(pfd) != 0) return -1;

    pid_t pid = fork();
    if (pid == 0) {
        close(pfd[0]);
        dup2(pfd[1], STDOUT_FILENO);
        close(pfd[1]);

        char *argv[] = { "tar", "-tf", (char *)lpkg_path, NULL };
        execvp("tar", argv);
        _exit(127);
    }
    close(pfd[1]);

    FILE *f = fdopen(pfd[0], "r");
    if (!f) {
        close(pfd[0]);
        return -1;
    }

    char line[256];
    pkg->file_count = 0;
    while (fgets(line, sizeof(line), f) && pkg->file_count < 256) {
        size_t len = strlen(line);
        while (len > 0 && (line[len - 1] == '\n' || line[len - 1] == '\r')) {
            line[len - 1] = '\0';
            len--;
        }
        if (len > 0 && line[len - 1] != '/') { /* Skip directories */
            /* Prepend slash for absolute path representation */
            snprintf(pkg->files[pkg->file_count++], 256, "/%s", line);
        }
    }
    fclose(f);

    int status;
    waitpid(pid, &status, 0);
    return (WIFEXITED(status) && WEXITSTATUS(status) == 0) ? 0 : -1;
}

static int install_local_lpkg(const char *path) {
    /* Check if .sig file exists next to it */
    char sig_path[512];
    snprintf(sig_path, sizeof(sig_path), "%s.sig", path);

    if (access(sig_path, F_OK) != 0) {
        fprintf(stderr, "lpkg: error: no signature file found at %s\n", sig_path);
        return -1;
    }

    if (!verify_signature(path, sig_path)) {
        return -1;
    }

    /* Extract package */
    printf("lpkg: extracting package...\n");
    if (extract_package(path, "/") != 0) {
        fprintf(stderr, "lpkg: extraction failed\n");
        return -1;
    }

    /* Parse filename to guess package name/version */
    /* format: name-version.lpkg */
    char name[LPKG_MAX_NAME] = "unknown";
    char version[LPKG_MAX_VER] = "1.0";
    
    const char *base = strrchr(path, '/');
    if (!base) base = path;
    else base++;

    char name_buf[128];
    snprintf(name_buf, sizeof(name_buf), "%s", base);
    char *dot = strrchr(name_buf, '.');
    if (dot) *dot = '\0'; // strip extension .lpkg

    char *dash = strchr(name_buf, '-');
    if (dash) {
        *dash = '\0';
        snprintf(name, sizeof(name), "%s", name_buf);
        snprintf(version, sizeof(version), "%s", dash + 1);
    } else {
        snprintf(name, sizeof(name), "%s", name_buf);
    }

    /* Check if already installed */
    int idx = -1;
    for (int i = 0; i < g_db_count; i++) {
        if (strcmp(g_db[i].name, name) == 0) {
            idx = i;
            break;
        }
    }

    lpkg_t *pkg;
    if (idx >= 0) {
        pkg = &g_db[idx];
    } else {
        if (g_db_count >= MAX_DB_PKGS) {
            fprintf(stderr, "lpkg: database full\n");
            return -1;
        }
        pkg = &g_db[g_db_count++];
    }

    memset(pkg, 0, sizeof(lpkg_t));
    snprintf(pkg->name, sizeof(pkg->name), "%s", name);
    snprintf(pkg->version, sizeof(pkg->version), "%s", version);
    snprintf(pkg->description, sizeof(pkg->description), "%s package", name);

    /* Enumerate files inside package */
    get_tar_file_list(path, pkg);

    /* Save to DB */
    if (db_save_installed(g_db, g_db_count) != 0) {
        fprintf(stderr, "lpkg: failed to save database\n");
        return -1;
    }

    printf("lpkg: package %s-%s installed successfully\n", name, version);
    return 0;
}

static int handle_install(const char *target) {
    /* If it is a local file path */
    if (strstr(target, ".lpkg") != NULL) {
        return install_local_lpkg(target);
    }

    /* Otherwise, look up in repository index */
    FILE *idx = fopen(INDEX_PATH, "r");
    if (!idx) {
        fprintf(stderr, "lpkg: no repository index. Run 'lpkg update' first.\n");
        return -1;
    }

    char line[512];
    char pkg_name_pattern[128];
    snprintf(pkg_name_pattern, sizeof(pkg_name_pattern), "[[package]]");
    
    bool found = false;
    char found_url[256] = "";
    char name_buf[128] = "";

    while (fgets(line, sizeof(line), idx)) {
        char *lnk = line;
        while (*lnk == ' ' || *lnk == '\t') lnk++;
        size_t len = strlen(lnk);
        while (len > 0 && (lnk[len - 1] == '\n' || lnk[len - 1] == '\r')) {
            lnk[len - 1] = '\0';
            len--;
        }

        if (strcmp(lnk, "[[package]]") == 0) {
            name_buf[0] = '\0';
            found_url[0] = '\0';
        } else {
            if (strncmp(lnk, "name = \"", 8) == 0) {
                char *end = strchr(lnk + 8, '"');
                if (end) { *end = '\0'; snprintf(name_buf, sizeof(name_buf), "%s", lnk + 8); }
            } else if (strncmp(lnk, "url = \"", 7) == 0) {
                char *end = strchr(lnk + 7, '"');
                if (end) { *end = '\0'; snprintf(found_url, sizeof(found_url), "%s", lnk + 7); }
            }
        }

        if (name_buf[0] && strcmp(name_buf, target) == 0 && found_url[0]) {
            found = true;
            break;
        }
    }
    fclose(idx);

    if (!found) {
        fprintf(stderr, "lpkg: package '%s' not found in repository index\n", target);
        return -1;
    }

    /* Download package + sig */
    printf("lpkg: downloading %s...\n", found_url);
    char dest_lpkg[256];
    snprintf(dest_lpkg, sizeof(dest_lpkg), "/tmp/%s.lpkg", target);
    if (run_curl(found_url, dest_lpkg) != 0) {
        fprintf(stderr, "lpkg: download failed\n");
        return -1;
    }

    char sig_url[512], dest_sig[256];
    snprintf(sig_url, sizeof(sig_url), "%s.sig", found_url);
    snprintf(dest_sig, sizeof(dest_sig), "/tmp/%s.lpkg.sig", target);
    printf("lpkg: downloading signature %s...\n", sig_url);
    if (run_curl(sig_url, dest_sig) != 0) {
        fprintf(stderr, "lpkg: signature download failed\n");
        unlink(dest_lpkg);
        return -1;
    }

    int ret = install_local_lpkg(dest_lpkg);
    unlink(dest_lpkg);
    unlink(dest_sig);
    return ret;
}

static int handle_remove(const char *name) {
    int idx = -1;
    for (int i = 0; i < g_db_count; i++) {
        if (strcmp(g_db[i].name, name) == 0) {
            idx = i;
            break;
        }
    }

    if (idx < 0) {
        fprintf(stderr, "lpkg: package '%s' is not installed\n", name);
        return -1;
    }

    printf("lpkg: removing files for package %s...\n", name);
    for (int j = 0; j < g_db[idx].file_count; j++) {
        const char *file_path = g_db[idx].files[j];
        if (unlink(file_path) == 0) {
            printf("  removed: %s\n", file_path);
        } else if (errno != ENOENT) {
            fprintf(stderr, "  failed to remove %s: %s\n", file_path, strerror(errno));
        }
    }

    /* Remove from DB array */
    for (int i = idx; i < g_db_count - 1; i++) {
        g_db[i] = g_db[i + 1];
    }
    g_db_count--;

    if (db_save_installed(g_db, g_db_count) != 0) {
        fprintf(stderr, "lpkg: failed to save database\n");
        return -1;
    }

    printf("lpkg: package %s removed successfully\n", name);
    return 0;
}

static int handle_list(void) {
    if (g_db_count == 0) {
        printf("No packages currently installed.\n");
        return 0;
    }
    printf("Installed packages:\n");
    for (int i = 0; i < g_db_count; i++) {
        printf("  %-20s %-10s %s\n", g_db[i].name, g_db[i].version, g_db[i].description);
    }
    return 0;
}

static int handle_update(void) {
    printf("lpkg: updating repository index...\n");
    (void)mkdir("/var", 0755);
    (void)mkdir("/var/lib", 0755);
    (void)mkdir("/var/lib/lpkg", 0755);

    if (run_curl("http://repo.mahina.os/index.toml", INDEX_PATH) != 0) {
        fprintf(stderr, "lpkg: failed to update repository index from http://repo.mahina.os/index.toml\n");
        return -1;
    }
    printf("lpkg: repository index updated successfully\n");
    return 0;
}

int main(int argc, char *argv[]) {
    if (argc < 2) {
        usage();
        return 1;
    }

    /* Load DB */
    g_db_count = db_load_installed(g_db, MAX_DB_PKGS);
    if (g_db_count < 0) {
        fprintf(stderr, "lpkg: fatal error loading database\n");
        return 1;
    }

    const char *cmd = argv[1];
    if (strcmp(cmd, "install") == 0) {
        if (argc < 3) { fprintf(stderr, "lpkg: error: install requires target\n"); return 1; }
        return handle_install(argv[2]) == 0 ? 0 : 1;
    } 
    else if (strcmp(cmd, "remove") == 0) {
        if (argc < 3) { fprintf(stderr, "lpkg: error: remove requires package name\n"); return 1; }
        return handle_remove(argv[2]) == 0 ? 0 : 1;
    } 
    else if (strcmp(cmd, "list") == 0) {
        return handle_list() == 0 ? 0 : 1;
    } 
    else if (strcmp(cmd, "update") == 0) {
        return handle_update() == 0 ? 0 : 1;
    } 
    else {
        fprintf(stderr, "lpkg: error: unknown command '%s'\n", cmd);
        usage();
        return 1;
    }
}
