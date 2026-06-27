#ifndef MAHINA_MOUNT_H
#define MAHINA_MOUNT_H

/*
 * mount.h — Filesystem Mount Manager (Stage 2)
 * Authority: Volume II / 02_boot_flow.md §Stage 2
 * Authority: Volume II / 09_filesystem.md §tmpfs Configuration
 *
 * Reads /etc/luna/fstab.toml and mounts filesystems in declaration order.
 * Failure to mount /proc, /sys, or /dev is FATAL (drop to emergency shell).
 * Failure to mount additional entries is logged WARN and continues.
 */

#include <stdbool.h>

/* Maximum filesystems in fstab.toml */
#define MOUNT_MAX_ENTRIES 16

/* Mount result for a single entry */
typedef enum {
    MOUNT_OK         =  0,
    MOUNT_ERR_FATAL  = -1,  /* Critical filesystem failed — drop to shell */
    MOUNT_ERR_WARN   = -2,  /* Non-critical filesystem failed — continue   */
} mount_result_t;

/*
 * mount_early() — Mount /proc, /sys, /dev before fstab is readable.
 *
 * Called at very early Stage 2, before luna_log_init() has a writable log
 * path (since /var is on the real root). Uses hardcoded paths.
 * Returns MOUNT_OK on success, MOUNT_ERR_FATAL on any failure.
 */
mount_result_t mount_early(void);

/*
 * mount_fstab() — Parse /etc/luna/fstab.toml and mount all entries.
 *
 * Mounts in [[mount]] declaration order.
 * Fatal if any of: /proc, /sys, /dev fail to mount.
 * Returns MOUNT_OK if all mounts succeeded.
 * Returns MOUNT_ERR_WARN if non-critical mounts failed (boot continues).
 * Returns MOUNT_ERR_FATAL if a critical mount failed (drop to shell).
 */
mount_result_t mount_fstab(const char *fstab_path);

/*
 * mount_unmount_all() — Unmount all mounted filesystems in reverse order.
 *
 * Called during shutdown sequencing. Non-fatal if individual unmounts fail.
 */
void mount_unmount_all(void);

#endif /* MAHINA_MOUNT_H */
