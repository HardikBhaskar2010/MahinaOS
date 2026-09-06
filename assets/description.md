# Mahina OS: A Deterministic, Lightweight, and AI-Native Operating System

## Executive Overview

Mahina OS is an independent operating system initiative engineered on the discipline of Documentation-First Engineering. Designed from bare metal to eliminate decades of accumulated Unix legacy complexity, Mahina delivers uncompromising determinism, zero-allocation early boot graphics, and a lightweight native desktop environment optimized for autonomous agent control.

## Architectural Philosophy: Documentation-First Engineering

In Mahina, no source code is accepted or compiled unless its behavior, data structures, and failure modes are codified within the Divine Collection of Knowledge about Luna (DCKL). The codebase serves strictly as the executable verification of its formal architectural specifications.

## Core System Subsystems

### 1. luna-init (PID 1 Service Manager)
- Implemented entirely in C17 with zero third-party dependencies.
- Replaces legacy sysvinit and systemd with a deterministic Directed Acyclic Graph (DAG) dependency solver.
- Service definitions are authored in strict TOML specifications, allowing cyclic dependency detection at parse time.
- Integrated zombie process reaping and deterministic teardown sequences guaranteeing zero orphaned background processes.

### 2. luna-splash (Early Boot Graphics Engine)
- Decoupled early boot rendering subsystem interfacing directly with the Linux framebuffer device (/dev/fb0).
- Engineered entirely without dynamic memory allocation (malloc-free) to guarantee absolute memory safety before kernel userland memory allocators stabilize.
- Renders smooth transition animations using pre-rasterized hardware-aligned scanlines.

### 3. Luna Graphics Protocol (LGP) & Compositor
- A compact display protocol and compositor designed to bypass the complexity and overhead of legacy display servers.
- Native shared-memory buffer transport with hardware-synchronized vertical retrace flipping.
- Built-in alpha blending, surface clipping, and privileged Window Manager extensions.

### 4. LunaGUI Toolkit & Desktop Shell (luna-shell)
- Retained-mode graphical toolkit written in C17 featuring a hierarchical widget tree (VBox, HBox, Canvas, Terminal).
- Brutalist, high-contrast visual language with sub-pixel text rasterization.
- Integrated system monitoring displaying real-time memory allocations, IPC packet latency, and active kernel threads.

## Technology Stack & Toolchain

- Languages: C17 (Strict ISO/IEC 9899:2018), Assembly (x86_64)
- Bootloader: Limine Bootloader protocol
- Kernel Base: Stripped and hardened custom Linux kernel configuration
- Build Infrastructure: GNU Make, Clang/LLVM 18, AddressSanitizer (ASan), UndefinedBehaviorSanitizer (UBSan)
- Emulation & Verification: QEMU System Emulation with GDB remote target debugging

## Performance & System Benchmarks

- Idle Memory Footprint: Under 142 Megabytes of total system RAM including active compositor and desktop shell.
- Cold Boot Time: Less than 850 milliseconds from bootloader handoff to interactive desktop shell in QEMU virtualization.
- Binary Footprint: Entire userland toolchain and system utilities occupy less than 18 Megabytes of storage.