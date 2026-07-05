/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

#include "pkg.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#define DB_PATH "/var/lib/lpkg/installed.toml"

static void ensure_db_dir(void) {
    (void)mkdir("/var", 0755);
    (void)mkdir("/var/lib", 0755);
    (void)mkdir("/var/lib/lpkg", 0755);
}

int db_load_installed(lpkg_t *pkgs, int max_pkgs) {
    ensure_db_dir();
    FILE *f = fopen(DB_PATH, "r");
    if (!f) return 0; /* No DB yet is normal */

    char line[512];
    int count = 0;
    lpkg_t *cur = NULL;

    while (fgets(line, sizeof(line), f)) {
        /* Strip newline and whitespace */
        char *lnk = line;
        while (*lnk == ' ' || *lnk == '\t') lnk++;
        size_t len = strlen(lnk);
        while (len > 0 && (lnk[len - 1] == '\n' || lnk[len - 1] == '\r')) {
            lnk[len - 1] = '\0';
            len--;
        }

        if (strcmp(lnk, "[[package]]") == 0) {
            if (count >= max_pkgs) break;
            cur = &pkgs[count++];
            memset(cur, 0, sizeof(lpkg_t));
        } else if (cur) {
            if (strncmp(lnk, "name = \"", 8) == 0) {
                char *end = strchr(lnk + 8, '"');
                if (end) { *end = '\0'; snprintf(cur->name, sizeof(cur->name), "%s", lnk + 8); }
            } else if (strncmp(lnk, "version = \"", 11) == 0) {
                char *end = strchr(lnk + 11, '"');
                if (end) { *end = '\0'; snprintf(cur->version, sizeof(cur->version), "%s", lnk + 11); }
            } else if (strncmp(lnk, "description = \"", 15) == 0) {
                char *end = strchr(lnk + 15, '"');
                if (end) { *end = '\0'; snprintf(cur->description, sizeof(cur->description), "%s", lnk + 15); }
            } else if (strncmp(lnk, "\"/", 2) == 0) {
                /* Inside files array */
                char *end = strchr(lnk + 1, '"');
                if (end) {
                    *end = '\0';
                    if (cur->file_count < 256) {
                        snprintf(cur->files[cur->file_count++], 256, "%s", lnk + 1);
                    }
                }
            }
        }
    }
    fclose(f);
    return count;
}

int db_save_installed(const lpkg_t *pkgs, int count) {
    ensure_db_dir();
    FILE *f = fopen(DB_PATH, "w");
    if (!f) return -1;

    fprintf(f, "# /var/lib/lpkg/installed.toml — Managed by lpkg\n\n");
    for (int i = 0; i < count; i++) {
        fprintf(f, "[[package]]\n");
        fprintf(f, "name = \"%s\"\n", pkgs[i].name);
        fprintf(f, "version = \"%s\"\n", pkgs[i].version);
        fprintf(f, "description = \"%s\"\n", pkgs[i].description);
        fprintf(f, "files = [\n");
        for (int j = 0; j < pkgs[i].file_count; j++) {
            fprintf(f, "  \"%s\"%s\n", pkgs[i].files[j], (j + 1 < pkgs[i].file_count) ? "," : "");
        }
        fprintf(f, "]\n\n");
    }
    fclose(f);
    return 0;
}
