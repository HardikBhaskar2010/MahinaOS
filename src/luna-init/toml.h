#ifndef MAHINA_TOML_H
#define MAHINA_TOML_H

/*
 * toml.h — Minimal TOML Parser for luna-init
 * Authority: Volume II / 04_init_system.md (service file format)
 * Authority: Volume II / 02_boot_flow.md §Stage 2 (fstab.toml)
 * Authority: Volume VI / 01_coding_standards.md §5 (bounded buffers, fuzzing)
 *
 * Supports only the TOML subset required by Mahina configuration files:
 *   - [section] headers
 *   - [[array-of-tables]] (for fstab [[mount]] entries)
 *   - key = "string"
 *   - key = ["array", "of", "strings"]
 *   - key = integer
 *   - key = true | false
 *
 * Does NOT support: dates, inline tables, multi-line strings, hex/octal/float.
 * Any unsupported syntax produces TOML_ERR_UNSUPPORTED — never silent failure.
 *
 * All buffers are strictly bounded. No dynamic allocation inside the parser.
 * The caller provides all memory. This makes the parser async-signal-safe and
 * suitable for use in the initramfs before the heap is initialized.
 *
 * Fuzzing: tests/fuzz/toml/fuzz_toml.c covers this parser with AFL++.
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* ─── Limits ─────────────────────────────────────────────────────────────── */

#define TOML_MAX_KEY_LEN      64
#define TOML_MAX_STR_LEN      256
#define TOML_MAX_SECTION_LEN  64
#define TOML_MAX_ARRAY_ITEMS  16

/* ─── Error codes ────────────────────────────────────────────────────────── */

typedef enum {
    TOML_OK              =  0,
    TOML_ERR_INVALID     = -1,  /* Malformed TOML syntax */
    TOML_ERR_OVERFLOW    = -2,  /* Value exceeds buffer limit */
    TOML_ERR_NOT_FOUND   = -3,  /* Key or section does not exist */
    TOML_ERR_TYPE        = -4,  /* Value type mismatch */
    TOML_ERR_UNSUPPORTED = -5,  /* Valid TOML but not in our subset */
    TOML_ERR_IO          = -6,  /* File read error */
    TOML_ERR_TOO_LARGE   = -7,  /* Input file too large */
} toml_error_t;

/* ─── Value types ────────────────────────────────────────────────────────── */

typedef enum {
    TOML_TYPE_STRING  = 0,
    TOML_TYPE_INTEGER = 1,
    TOML_TYPE_BOOLEAN = 2,
    TOML_TYPE_ARRAY   = 3,
} toml_value_type_t;

/* ─── Parsed value ───────────────────────────────────────────────────────── */

typedef struct {
    toml_value_type_t type;
    union {
        char        str[TOML_MAX_STR_LEN];
        int64_t     integer;
        bool        boolean;
        struct {
            char   items[TOML_MAX_ARRAY_ITEMS][TOML_MAX_STR_LEN];
            size_t count;
        } array;
    } v;
} toml_value_t;

/* ─── Document handle ────────────────────────────────────────────────────── */

/*
 * toml_doc_t is an opaque handle. Internals are in toml.c.
 * Maximum document size: 64 KB (sufficient for all Mahina config files).
 */
#define TOML_MAX_DOC_BYTES (64 * 1024)

typedef struct toml_doc toml_doc_t;

/* ─── API ────────────────────────────────────────────────────────────────── */

/*
 * toml_load_file() — Parse a TOML file from disk.
 *
 * Allocates a toml_doc_t with calloc. Caller must free with toml_free().
 * Returns NULL and sets *out_error on failure.
 */
toml_doc_t *toml_load_file(const char *path, toml_error_t *out_error);

/*
 * toml_load_buffer() — Parse a TOML document from a memory buffer.
 *
 * buf must be null-terminated. buf_len must equal strlen(buf).
 * Does not take ownership of buf. Returns NULL on failure.
 */
toml_doc_t *toml_load_buffer(const char *buf, size_t buf_len,
                              toml_error_t *out_error);

/*
 * toml_free() — Free all resources associated with a parsed document.
 */
void toml_free(toml_doc_t *doc);

/*
 * toml_get() — Look up a value in a section.
 *
 * section: the [section] name, or NULL for top-level keys.
 * key    : the key name.
 * out    : caller-provided toml_value_t to receive the result.
 *
 * Returns TOML_OK on success, TOML_ERR_NOT_FOUND if key/section absent.
 *
 * Example:
 *   toml_value_t v;
 *   toml_get(doc, "service", "name", &v);
 *   // v.type == TOML_TYPE_STRING, v.v.str == "dbus"
 */
toml_error_t toml_get(const toml_doc_t *doc, const char *section,
                       const char *key, toml_value_t *out);

/*
 * toml_get_nested() — Look up a value in a nested section (e.g. [service.restart]).
 *
 * parent : the parent section name (e.g. "service")
 * child  : the child section name (e.g. "restart")
 * key    : the key name (e.g. "policy")
 *
 * Returns TOML_OK on success.
 */
toml_error_t toml_get_nested(const toml_doc_t *doc,
                              const char *parent, const char *child,
                              const char *key, toml_value_t *out);

/*
 * toml_array_table_count() — Count [[array-of-tables]] entries.
 *
 * Returns the number of [[section]] entries in the document.
 * Used to count [[mount]] entries in fstab.toml.
 */
int toml_array_table_count(const toml_doc_t *doc, const char *section);

/*
 * toml_array_table_get() — Get a value from the Nth [[array-of-tables]] entry.
 *
 * index : zero-based index into the array-of-tables.
 * key   : key name within that table entry.
 *
 * Returns TOML_OK on success, TOML_ERR_NOT_FOUND if index out of range.
 */
toml_error_t toml_array_table_get(const toml_doc_t *doc, const char *section,
                                   int index, const char *key,
                                   toml_value_t *out);

/*
 * toml_strerror() — Human-readable description of a toml_error_t.
 */
const char *toml_strerror(toml_error_t err);

#endif /* MAHINA_TOML_H */
