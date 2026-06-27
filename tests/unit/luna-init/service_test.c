/*
 * service_test.c
 */

#include "unity.h"
#include "../../../src/luna-init/service.h"
#include <string.h>

void setUp(void) {}
void tearDown(void) {}

void test_service_find(void);
void test_service_find(void) {
    g_service_count = 1;
    strcpy(g_services[0].name, "test_svc");

    service_t *svc = service_find("test_svc");
    TEST_ASSERT_NOT_NULL(svc);
    TEST_ASSERT_EQUAL_STRING("test_svc", svc->name);

    TEST_ASSERT_NULL(service_find("nonexistent"));
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_service_find);
    return UNITY_END();
}
