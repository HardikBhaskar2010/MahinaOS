/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

#ifndef MAHINA_HOSTNAME_H
#define MAHINA_HOSTNAME_H

/*
 * hostname.h — Hostname Configuration (Stage 3)
 * Authority: Volume II / 02_boot_flow.md §Stage 3
 * Authority: Volume II / 09_filesystem.md (/etc/luna/hostname)
 */

/*
 * hostname_set() — Read /etc/luna/hostname and call sethostname(2).
 *
 * Non-fatal on failure: logs WARN and continues (Stage 3 operations are
 * non-fatal per 02_boot_flow.md §Stage 3).
 */
void hostname_set(const char *hostname_path);

#endif /* MAHINA_HOSTNAME_H */
