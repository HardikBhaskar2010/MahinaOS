/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef MAHINA_DEPGRAPH_H
#define MAHINA_DEPGRAPH_H

/*
 * depgraph.h — Service Dependency Graph (Kahn's Algorithm)
 * Authority: Volume II / 04_init_system.md §Dependency Graph Resolution
 *
 * Builds a DAG from service after/before declarations.
 * Validates: all referenced names exist, no circular dependencies.
 * Circular dependency detection via topological sort (DFS cycle detection).
 *
 * Fatal boot error if validation fails — malformed service deps must be fixed.
 */

#include "service.h"

/*
 * depgraph_build() — Resolve and validate the dependency graph.
 *
 * Reads g_services[], resolves name references to indices,
 * validates the DAG (no cycles, no missing names).
 *
 * Returns 0 on success.
 * Returns -1 on fatal error (cycle detected or missing dependency name).
 *
 * On success, each service's dep_indices[] and dep_count are populated.
 */
int depgraph_build(void);

/*
 * depgraph_topo_sort() — Produce a topological start order.
 *
 * Writes service indices into out_order[] in dependency-satisfied start order.
 * out_order must have room for SERVICE_MAX_COUNT entries.
 * Returns the number of services written, or -1 on error.
 *
 * Services with no mutual dependency edge may appear in any relative order.
 * The actual parallelism decision is made by the supervisor.
 */
int depgraph_topo_sort(int *out_order, int max_count);

#endif /* MAHINA_DEPGRAPH_H */
