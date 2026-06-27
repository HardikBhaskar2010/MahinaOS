/*
 * toml_test.c
 */

#include "unity.h"
#include "../../../src/luna-init/toml.h"
#include <string.h>

void setUp(void) {}
void tearDown(void) {}

void test_toml_valid_basic(void);
void test_toml_valid_basic(void) {
    const char *doc_str =
        "[service]\n"
        "name = \"dbus\"\n"
        "attempts = 5\n"
        "enabled = true\n"
        "options = [\"foo\", \"bar\"]\n";

    toml_error_t err;
    toml_doc_t *doc = toml_load_buffer(doc_str, strlen(doc_str), &err);
    TEST_ASSERT_NOT_NULL(doc);
    TEST_ASSERT_EQUAL(TOML_OK, err);
    toml_free(doc);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_toml_valid_basic);
    return UNITY_END();
}
