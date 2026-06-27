/*
 * log_test.c — Unit tests for Mahina log
 */

#include "unity.h"
#include "../../../src/luna-init/log.h"
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <stdlib.h>
#include <sys/stat.h>

void setUp(void) {}
void tearDown(void) {
    luna_log_close();
    unlink("/tmp/test_boot.log");
    unlink("/tmp/test_runtime.log");
}

void test_log_init(void);
void test_log_init(void) {
    luna_log_init("/tmp/test_boot.log", "/tmp/test_runtime.log");
    LUNA_INFO("test", "Test log message");
    luna_log_close();

    struct stat st;
    TEST_ASSERT_EQUAL(0, stat("/tmp/test_boot.log", &st));
    TEST_ASSERT_TRUE(st.st_size > 0);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_log_init);
    return UNITY_END();
}
