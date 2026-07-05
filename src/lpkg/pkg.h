/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

#ifndef LPKG_PKG_H
#define LPKG_PKG_H

#include <stdbool.h>
#include <stddef.h>

#define LPKG_MAX_NAME 64
#define LPKG_MAX_VER  32

typedef struct {
    char name[LPKG_MAX_NAME];
    char version[LPKG_MAX_VER];
    char description[256];
    char files[256][256]; /* List of files installed */
    int  file_count;
} lpkg_t;

/* 
 * db_load_installed() — Read all installed packages from db file.
 * Returns count of loaded packages, or -1 on error.
 */
int db_load_installed(lpkg_t *pkgs, int max_pkgs);

/*
 * db_save_installed() — Save list of installed packages to db file.
 * Returns 0 on success, -1 on error.
 */
int db_save_installed(const lpkg_t *pkgs, int count);

/*
 * verify_signature() — Verify .lpkg file signature using libsodium.
 * Returns true if signature is valid, false otherwise.
 */
bool verify_signature(const char *lpkg_path, const char *sig_path);

/*
 * extract_package() — Extracts tar.xz payload to destination (normally /).
 * Returns 0 on success, -1 on error.
 */
int extract_package(const char *lpkg_path, const char *dest_dir);

#endif /* LPKG_PKG_H */
