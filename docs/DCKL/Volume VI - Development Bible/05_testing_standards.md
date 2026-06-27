# Mahina — Testing Standards
**Volume VI · Chapter 05**
**Classification:** Development Bible — Quality Assurance
**Status:** Canonical · All merged code must comply with these testing standards

---

## Purpose

This document defines the testing standards for Mahina. It answers: **what kind of tests are required, where they live, how they are run, and what constitutes a passing test suite.**

Mahina is a systems-level project. Memory corruption, race conditions, and incorrect boot sequencing are not "bugs to fix later" — they are failures that make the system unusable or insecure. Testing standards reflect this seriousness.

---

## Testing Philosophy

> **If it is not tested, it is not trusted.**

This applies to documentation too. A function that is not tested may not behave as its Volume II–V documentation claims. The test suite is the proof that the documentation is accurate.

Three properties every Mahina test must have:

**1. Reproducible**
A test that passes sometimes and fails sometimes is not a test. It is noise. Flaky tests are treated as failures and removed until fixed.

**2. Isolated**
Tests must not depend on system state left by previous tests. Each test sets up its own environment and tears it down completely. Shared global state between test cases is forbidden.

**3. Fast**
The unit test suite must complete in under 30 seconds on reference hardware. Slow tests belong in the integration suite and must be labeled as such.

---

## Test Categories

### Unit Tests

```
Scope:      A single function or struct method in isolation.
Language:   C (using a lightweight testing framework — see below).
Location:   tests/unit/<component>/<file>_test.c
Runner:     make test-unit
Speed:      Entire suite < 30 seconds.

Rules:
  - Every public function exported in a .h file must have at least one unit test.
  - Every error path (non-zero return code) must have a corresponding test.
  - No network calls. No disk writes (use tmpfs or mock).
  - No sleep() or time-dependent code. Timers must be injectable.
```

### Integration Tests

```
Scope:      Two or more components working together.
            Examples: lgp-compositor accepting a connection from luna-shell.
                      luna-init starting lgp-compositor and receiving readiness.
Language:   C or Shell scripts (POSIX sh).
Location:   tests/integration/<scenario>/
Runner:     make test-integration
Speed:      Each test < 60 seconds. Full suite < 10 minutes.

Rules:
  - Must run inside a QEMU VM (not on the host machine).
  - Must start from a clean snapshot.
  - Must leave no persistent state after completion.
```

### End-to-End Tests (Stage 3+)

```
Scope:      Full system boot to interactive desktop.
            Examples: Boot Mahina, verify compositor is running,
                      verify luna-ai-d enters READY or DEGRADED state.
Language:   Python (test harness).
Location:   tests/e2e/
Runner:     make test-e2e
Speed:      Each test < 5 minutes. Full suite < 30 minutes.

Rules:
  - Requires full QEMU boot.
  - Automated via QEMU serial console and D-Bus inspection.
  - Must include a screenshot comparison for visual regression (Stage 3+).
```

### Fuzz Tests

```
Scope:      Parser modules that handle external input.
            Required for: LGP TLV parser, TOML config parser, lpkg manifest parser.
Language:   C (AFL++ integration).
Location:   tests/fuzz/<parser>/
Runner:     make test-fuzz (CI: runs for 5 minutes minimum per parser)

Rules:
  - Every external-input parser MUST have a fuzz target.
  - AFL++ corpus stored in tests/fuzz/<parser>/corpus/.
  - Any crash found by fuzzer = P0 bug, blocks release.
```

---

## Testing Framework

Mahina uses **Unity** (by ThrowTheSwitch) as the C unit testing framework.

```
Why Unity:
  - Single-file implementation — zero external dependency
  - C89/C99/C17 compatible
  - Minimal macros — does not obscure the code under test
  - Output is TAP-compatible (parseable by CI systems)

Installation: Vendored at tests/vendor/unity/
```

### Test File Structure

```c
// tests/unit/lgp/surface_test.c

#include "unity.h"
#include "lgp/surface.h"

// setUp / tearDown run before and after every test
void setUp(void) {
    // Initialize test fixtures
}

void tearDown(void) {
    // Clean up test fixtures
}

void test_lgp_surface_create_returns_valid_surface(void) {
    lgp_client_t client = {0};
    lgp_surface_t *surface = NULL;

    int result = lgp_surface_create(&client, &surface);

    TEST_ASSERT_EQUAL(0, result);
    TEST_ASSERT_NOT_NULL(surface);
}

void test_lgp_surface_create_rejects_null_client(void) {
    lgp_surface_t *surface = NULL;

    int result = lgp_surface_create(NULL, &surface);

    TEST_ASSERT_EQUAL(-EINVAL, result);
    TEST_ASSERT_NULL(surface);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_lgp_surface_create_returns_valid_surface);
    RUN_TEST(test_lgp_surface_create_rejects_null_client);
    return UNITY_END();
}
```

---

## Memory Safety in Tests

All test builds must use the mandatory sanitizers defined in `01_coding_standards.md`:

```makefile
# Test build flags — mandatory
CFLAGS_TEST = $(CFLAGS) \
    -fsanitize=address \
    -fsanitize=undefined \
    -fno-omit-frame-pointer \
    -g

# Fuzz build flags
CFLAGS_FUZZ = $(CFLAGS) \
    -fsanitize=fuzzer,address \
    -g
```

**Any test that passes with sanitizers disabled but fails with sanitizers enabled is a real bug in the production code.**

---

## CI Pipeline Test Gates

Every pull request must pass all of the following before merge:

```
Gate 1: Build
  make all           — must compile without warnings (-Wall -Wextra -Werror)
  clang-tidy         — must produce zero warnings

Gate 2: Unit Tests
  make test-unit     — must pass 100% of tests with ASan + UBSan enabled

Gate 3: Static Analysis
  cppcheck           — no new errors introduced
  
Gate 4: Fuzz Regression
  make test-fuzz-regression  — run known crash inputs against fuzz targets
                               (verifies previous crashes are fixed)

Gate 5: Integration Tests (if component boundary was modified)
  make test-integration — must pass all relevant integration tests

Gate 6: Documentation Sync
  Manual review:     If a struct or function signature changed,
                     the corresponding Volume II–V document must be updated.
```

---

## What Does NOT Require Tests

The following are excluded from unit test requirements:

```
Excluded from unit testing:
  - luna-init main boot sequence     (integration-tested only)
  - lgp-compositor render loop       (integration and visual regression tested)
  - luna-ai-d Python daemon          (unit-tested with pytest, not Unity)
  - Build scripts and Makefiles
  - Documentation (*.md files)
```

### Python Tests for luna-ai-d

Because `luna-ai-d` is written in Python (DL-049), its unit tests use **pytest**:

```
Location:   luna-ai-d/tests/
Runner:     pytest luna-ai-d/tests/
Coverage:   pytest --cov=luna_ai_d --cov-fail-under=80

Rules:
  - All async functions must be tested with pytest-asyncio.
  - Ollama client calls must be mocked in all unit tests.
  - D-Bus calls must be mocked in all unit tests.
  - No test may make a real network call or start a real Ollama instance.
```

---

## Performance Regression Tests

Performance benchmarks from `04_benchmarks.md` are enforced in the CI pipeline via automated measurement:

```
Measured automatically in CI:
  - Boot time:              luna-init to compositor ready (QEMU serial console timing)
  - Compositor frame time:  measured via LGP performance counters
  - luna-ai-d startup:      time from process start to D-Bus READY signal

If a PR causes any benchmark to regress by > 10% from baseline:
  → PR is flagged for manual performance review
  → Merge is blocked until the regression is explained or fixed
```

---

*Document: `Volume VI / 05_testing_standards.md`*
*Author: Hardik Bhaskar (Luna Kitsune)*
*Version: 1.0*
*Depends on: 01_coding_standards.md, 04_benchmarks.md, DL-049*
*Informs: Volume VII/implementation_roadmap.md, CI pipeline configuration*
