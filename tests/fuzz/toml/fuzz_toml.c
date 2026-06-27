/*
 * fuzz_toml.c — AFL++ fuzzing harness for Mahina TOML parser
 * Authority: Volume VI / 01_coding_standards.md §5 (Fuzzing Requirements)
 *
 * This file is meant to be compiled with afl-clang-lto.
 * It feeds arbitrary fuzz data directly into the TOML parser to ensure
 * no buffer overflows, infinite loops, or crashes occur.
 */

#include "../../../src/luna-init/toml.h"
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Use AFL++ persistent mode for high performance */
__AFL_FUZZ_INIT();

#pragma clang optimize off

int main(void) {
    /* Persistent loop: afl-fuzz will feed data via stdin/shared-mem */
#ifdef __AFL_HAVE_MANUAL_CONTROL
    __AFL_INIT();
#endif

    unsigned char *buf = __AFL_FUZZ_TESTCASE_BUF;

    while (__AFL_LOOP(10000)) {
        int len = __AFL_FUZZ_TESTCASE_LEN;

        /* Skip trivially small or overly huge inputs for performance */
        if (len < 3 || len > TOML_MAX_DOC_BYTES) {
            continue;
        }

        /* Create a null-terminated copy for the parser */
        char *str = malloc(len + 1);
        if (!str) continue;
        memcpy(str, buf, len);
        str[len] = '\0';

        /* Run the parser */
        toml_error_t err;
        toml_doc_t *doc = toml_load_buffer(str, len, &err);

        /* Clean up */
        if (doc) toml_free(doc);
        free(str);
    }

    return 0;
}
