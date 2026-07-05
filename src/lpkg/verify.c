/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

#include "pkg.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sodium.h>

#define PUBKEY_PATH "/etc/luna/lpkg.pub"

static int hex2val(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

static int hex2bin(const char *hex, unsigned char *bin, size_t bin_len) {
    size_t len = strlen(hex);
    if (len / 2 > bin_len) return -1;
    for (size_t i = 0; i < len / 2; i++) {
        int high = hex2val(hex[i * 2]);
        int low  = hex2val(hex[i * 2 + 1]);
        if (high < 0 || low < 0) return -1;
        bin[i] = (unsigned char)((high << 4) | low);
    }
    return 0;
}

bool verify_signature(const char *lpkg_path, const char *sig_path) {
    if (sodium_init() < 0) {
        fprintf(stderr, "lpkg: error: libsodium initialization failed\n");
        return false;
    }

    /* Load public key */
    FILE *pf = fopen(PUBKEY_PATH, "r");
    if (!pf) {
        fprintf(stderr, "lpkg: warning: no public key found at %s. Verification skipped.\n", PUBKEY_PATH);
        return true; /* Dev/fallback mode */
    }
    unsigned char pubkey[crypto_sign_PUBLICKEYBYTES];
    char key_buf[128] = {0};
    size_t kn = fread(key_buf, 1, sizeof(key_buf) - 1, pf);
    fclose(pf);

    /* Strip newlines */
    while (kn > 0 && (key_buf[kn-1] == '\n' || key_buf[kn-1] == '\r')) {
        key_buf[kn-1] = '\0';
        kn--;
    }

    if (kn == crypto_sign_PUBLICKEYBYTES) {
        memcpy(pubkey, key_buf, crypto_sign_PUBLICKEYBYTES);
    } else if (kn == crypto_sign_PUBLICKEYBYTES * 2) {
        if (hex2bin(key_buf, pubkey, sizeof(pubkey)) != 0) {
            fprintf(stderr, "lpkg: error: invalid public key hex format\n");
            return false;
        }
    } else {
        fprintf(stderr, "lpkg: error: public key file must be %d bytes raw or %d bytes hex\n",
                crypto_sign_PUBLICKEYBYTES, crypto_sign_PUBLICKEYBYTES * 2);
        return false;
    }

    /* Load signature */
    FILE *sf = fopen(sig_path, "r");
    if (!sf) {
        fprintf(stderr, "lpkg: error: signature file %s not found\n", sig_path);
        return false;
    }
    unsigned char sig[crypto_sign_BYTES];
    char sig_buf[256] = {0};
    size_t sn = fread(sig_buf, 1, sizeof(sig_buf) - 1, sf);
    fclose(sf);

    while (sn > 0 && (sig_buf[sn-1] == '\n' || sig_buf[sn-1] == '\r')) {
        sig_buf[sn-1] = '\0';
        sn--;
    }

    if (sn == crypto_sign_BYTES) {
        memcpy(sig, sig_buf, crypto_sign_BYTES);
    } else if (sn == crypto_sign_BYTES * 2) {
        if (hex2bin(sig_buf, sig, sizeof(sig)) != 0) {
            fprintf(stderr, "lpkg: error: invalid signature hex format\n");
            return false;
        }
    } else {
        fprintf(stderr, "lpkg: error: signature file must be %d bytes raw or %d bytes hex\n",
                crypto_sign_BYTES, crypto_sign_BYTES * 2);
        return false;
    }

    /* Load package content for verification */
    FILE *lf = fopen(lpkg_path, "rb");
    if (!lf) {
        fprintf(stderr, "lpkg: error: package file %s not found\n", lpkg_path);
        return false;
    }
    fseek(lf, 0, SEEK_END);
    long size = ftell(lf);
    fseek(lf, 0, SEEK_SET);
    if (size <= 0) {
        fclose(lf);
        return false;
    }

    unsigned char *data = malloc((size_t)size);
    if (!data) {
        fclose(lf);
        return false;
    }
    if (fread(data, 1, (size_t)size, lf) != (size_t)size) {
        free(data);
        fclose(lf);
        return false;
    }
    fclose(lf);

    /* Verify signature */
    int ret = crypto_sign_verify_detached(sig, data, (unsigned long long)size, pubkey);
    free(data);

    if (ret != 0) {
        fprintf(stderr, "lpkg: error: PACKAGE SIGNATURE VERIFICATION FAILED!\n");
        return false;
    }

    printf("lpkg: signature verified successfully\n");
    return true;
}
