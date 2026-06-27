/*
 * test_toml.c — Unit tests for Mahina TOML parser
 *
 * Uses the Unity test framework.
 */

#include "unity.h"
#include "../../src/luna-init/toml.h"
#include <string.h>

void setUp(void) {}
void tearDown(void) {}

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

    toml_value_t v;
    TEST_ASSERT_EQUAL(TOML_OK, toml_get(doc, "service", "name", &v));
    TEST_ASSERT_EQUAL(TOML_TYPE_STRING, v.type);
    TEST_ASSERT_EQUAL_STRING("dbus", v.v.str);

    TEST_ASSERT_EQUAL(TOML_OK, toml_get(doc, "service", "attempts", &v));
    TEST_ASSERT_EQUAL(TOML_TYPE_INTEGER, v.type);
    TEST_ASSERT_EQUAL(5, v.v.integer);

    TEST_ASSERT_EQUAL(TOML_OK, toml_get(doc, "service", "enabled", &v));
    TEST_ASSERT_EQUAL(TOML_TYPE_BOOLEAN, v.type);
    TEST_ASSERT_TRUE(v.v.boolean);

    TEST_ASSERT_EQUAL(TOML_OK, toml_get(doc, "service", "options", &v));
    TEST_ASSERT_EQUAL(TOML_TYPE_ARRAY, v.type);
    TEST_ASSERT_EQUAL(2, v.v.array.count);
    TEST_ASSERT_EQUAL_STRING("foo", v.v.array.items[0]);
    TEST_ASSERT_EQUAL_STRING("bar", v.v.array.items[1]);

    toml_free(doc);
}

void test_toml_missing_key(void) {
    const char *doc_str = "[test]\nfoo = \"bar\"\n";
    toml_error_t err;
    toml_doc_t *doc = toml_load_buffer(doc_str, strlen(doc_str), &err);
    TEST_ASSERT_NOT_NULL(doc);

    toml_value_t v;
    TEST_ASSERT_EQUAL(TOML_ERR_NOT_FOUND, toml_get(doc, "test", "baz", &v));
    toml_free(doc);
}

void test_toml_array_of_tables(void) {
    const char *doc_str =
        "[[mount]]\ndevice = \"proc\"\n"
        "[[mount]]\ndevice = \"sysfs\"\n";

    toml_error_t err;
    toml_doc_t *doc = toml_load_buffer(doc_str, strlen(doc_str), &err);
    TEST_ASSERT_NOT_NULL(doc);

    TEST_ASSERT_EQUAL(2, toml_array_table_count(doc, "mount"));

    toml_value_t v;
    TEST_ASSERT_EQUAL(TOML_OK, toml_array_table_get(doc, "mount", 0, "device", &v));
    TEST_ASSERT_EQUAL_STRING("proc", v.v.str);

    TEST_ASSERT_EQUAL(TOML_OK, toml_array_table_get(doc, "mount", 1, "device", &v));
    TEST_ASSERT_EQUAL_STRING("sysfs", v.v.str);

    toml_free(doc);
}

void test_toml_buffer_overflow(void) {
    /* Test key length limit */
    char doc_str[512] = "long_key_";
    for (int i = 0; i < 70; i++) strcat(doc_str, "a");
    strcat(doc_str, " = \"val\"\n");

    toml_error_t err;
    toml_doc_t *doc = toml_load_buffer(doc_str, strlen(doc_str), &err);
    TEST_ASSERT_NULL(doc);
    TEST_ASSERT_EQUAL(TOML_ERR_OVERFLOW, err);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_toml_valid_basic);
    RUN_TEST(test_toml_missing_key);
    RUN_TEST(test_toml_array_of_tables);
    RUN_TEST(test_toml_buffer_overflow);
    return UNITY_END();
}
