/*
 * depgraph_test.c
 */

#include "unity.h"
#include "../../../src/luna-init/depgraph.h"
#include "../../../src/luna-init/service.h"
#include <string.h>

void setUp(void) {
    memset(g_services, 0, sizeof(g_services));
    g_service_count = 0;
}
void tearDown(void) {}

void test_depgraph_valid_linear(void);
void test_depgraph_valid_linear(void) {
    strcpy(g_services[0].name, "B");
    g_services[0].after_count = 1;
    strcpy(g_services[0].after[0], "A");
    g_service_count++;

    strcpy(g_services[1].name, "A");
    g_services[1].after_count = 0;
    g_service_count++;

    TEST_ASSERT_EQUAL(0, depgraph_build());
}

void test_depgraph_cycle(void);
void test_depgraph_cycle(void) {
    /* A -> B -> A */
    strcpy(g_services[0].name, "A");
    g_services[0].after_count = 1;
    strcpy(g_services[0].after[0], "B");
    g_service_count++;

    strcpy(g_services[1].name, "B");
    g_services[1].after_count = 1;
    strcpy(g_services[1].after[0], "A");
    g_service_count++;

    TEST_ASSERT_EQUAL(-1, depgraph_build());
}

void test_depgraph_missing(void);
void test_depgraph_missing(void) {
    /* A -> B, but B does not exist */
    strcpy(g_services[0].name, "A");
    g_services[0].after_count = 1;
    strcpy(g_services[0].after[0], "B");
    g_service_count++;

    TEST_ASSERT_EQUAL(-1, depgraph_build());
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_depgraph_valid_linear);
    RUN_TEST(test_depgraph_cycle);
    RUN_TEST(test_depgraph_missing);
    return UNITY_END();
}
