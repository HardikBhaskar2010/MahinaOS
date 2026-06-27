/*
 * test_depgraph.c — Unit tests for Mahina Dependency Graph
 *
 * Uses the Unity test framework.
 */

#include "unity.h"
#include "../../src/luna-init/depgraph.h"
#include "../../src/luna-init/service.h"
#include <string.h>

void setUp(void) {
    /* Reset service table before each test */
    memset(g_services, 0, sizeof(g_services));
    g_service_count = 0;
}

void tearDown(void) {}

void test_depgraph_valid_linear(void) {
    /* Service A -> Service B -> Service C */
    strcpy(g_services[0].name, "C");
    g_services[0].after_count = 1;
    strcpy(g_services[0].after[0], "B");
    g_service_count++;

    strcpy(g_services[1].name, "A");
    g_services[1].after_count = 0;
    g_service_count++;

    strcpy(g_services[2].name, "B");
    g_services[2].after_count = 1;
    strcpy(g_services[2].after[0], "A");
    g_service_count++;

    TEST_ASSERT_EQUAL(0, depgraph_build());

    int order[SERVICE_MAX_COUNT];
    int count = depgraph_topo_sort(order, SERVICE_MAX_COUNT);
    TEST_ASSERT_EQUAL(3, count);

    /* Check resolved order is A, B, C */
    TEST_ASSERT_EQUAL_STRING("A", g_services[order[0]].name);
    TEST_ASSERT_EQUAL_STRING("B", g_services[order[1]].name);
    TEST_ASSERT_EQUAL_STRING("C", g_services[order[2]].name);
}

void test_depgraph_circular_dependency(void) {
    /* Service A -> Service B -> Service A */
    strcpy(g_services[0].name, "A");
    g_services[0].after_count = 1;
    strcpy(g_services[0].after[0], "B");
    g_service_count++;

    strcpy(g_services[1].name, "B");
    g_services[1].after_count = 1;
    strcpy(g_services[1].after[0], "A");
    g_service_count++;

    TEST_ASSERT_EQUAL(0, depgraph_build());

    int order[SERVICE_MAX_COUNT];
    int count = depgraph_topo_sort(order, SERVICE_MAX_COUNT);
    TEST_ASSERT_EQUAL(-1, count); /* Should fail topologically */
}

void test_depgraph_missing_dependency(void) {
    /* Service A depends on nonexistent Service X */
    strcpy(g_services[0].name, "A");
    g_services[0].after_count = 1;
    strcpy(g_services[0].after[0], "X");
    g_service_count++;

    TEST_ASSERT_EQUAL(-1, depgraph_build()); /* Should fail build */
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_depgraph_valid_linear);
    RUN_TEST(test_depgraph_circular_dependency);
    RUN_TEST(test_depgraph_missing_dependency);
    return UNITY_END();
}
