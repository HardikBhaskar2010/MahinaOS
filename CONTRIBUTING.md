# Contributing to Mahina OS

Thank you for your interest in contributing to Mahina OS! 

## Core Philosophy

Before contributing, please read the entire Document Control Knowledge Library (DCKL) in the `docs/DCKL/` directory. 
Mahina strictly follows **Documentation-First Engineering**. No code is merged unless the architectural decisions have been formally accepted in the Decision Log.

Specifically, read:
1. `docs/DCKL/Volume I - Foundation/core_laws.md`
2. `docs/DCKL/Volume VI - Development Bible/01_coding_standards.md`

## Getting Started

1. Check the `Volume VII - Implementation Roadmap` to see what is currently in scope. Do not submit PRs for v2.0 features when we are building v1.0.
2. Ensure your C17 code compiles with zero warnings under `clang`.
3. Run `make lint` (clang-tidy) and ensure it passes.
4. Run `make test-unit` to ensure no memory leaks (ASan/UBSan are enabled).

## Submitting Pull Requests

1. Fork the repository and create your branch from `main`.
2. Include the copyright header in all new `.c` and `.h` files.
3. Keep PRs small and focused on a single architectural goal.
