/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef MAHINA_PANIC_H
#define MAHINA_PANIC_H

/*
 * panic.h — Panic Handler
 * Authority: Volume II / 04_init_system.md §Fail-Fast at PID 1
 * Authority: Core Law V (The User Owns the Machine — always a way out)
 *
 * luna-init panics by dropping to an emergency shell on TTY1.
 * A hung PID 1 is never acceptable. The user must always have a way out.
 */

/*
 * panic_drop_to_shell() — Drop to emergency busybox shell on TTY1.
 *
 * Logs the panic message at FATAL level, then exec()'s busybox sh.
 * Used when Stage 1 or Stage 2 fails unrecoverably.
 * This function does not return under normal circumstances.
 */
void panic_drop_to_shell(const char *reason);

/*
 * LUNA_PANIC() — Convenience macro for panic with file/line context.
 */
#define LUNA_PANIC(reason) \
    panic_drop_to_shell("[" __FILE__ ":" LUNA_STRINGIFY(__LINE__) "] " reason)

#define LUNA_STRINGIFY(x) LUNA_STRINGIFY2(x)
#define LUNA_STRINGIFY2(x) #x

#endif /* MAHINA_PANIC_H */
