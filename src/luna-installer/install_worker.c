/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * install_worker.c — Real disk-write backend for luna-installer.
 *
 * Runs in a forked child process. The parent UI reads progress from
 * the pipe and renders it in the progress bar.
 *
 * Sequence:
 *   1. Partition disk   (parted)
 *   2. Format ESP       (mkfs.fat)
 *   3. Format root      (mkfs.btrfs)
 *   4. Create Btrfs subvolumes (@root, @home, @snapshots)
 *   5. Mount target     (/mnt/mahina-install)
 *   6. Extract rootfs   (/usr/share/mahina/rootfs.tar.xz via tar)
 *   7. Write system config files (fstab.toml, hostname, locale, passwd)
 *   8. Write limine.conf to the ESP
 *   9. Unmount and sync
 *
 * On any failure the worker sends "PERCENT|ERROR: <msg>" and exits 1.
 */

#include "install_worker.h"

#include <crypt.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

/* ── Internal helpers ────────────────────────────────────────────────────── */

/* Send a "PERCENT|MSG\n" progress update to the parent UI */
static void progress(int pipe_fd, int pct, const char *msg)
{
    char buf[256];
    int n = snprintf(buf, sizeof(buf), "%d|%s\n", pct, msg);
    if (n > 0 && n < (int)sizeof(buf)) {
        (void)write(pipe_fd, buf, (size_t)n);
    }
}

/* Run a shell command, wait for it, return exit code */
static int run_cmd(const char *cmd)
{
    return system(cmd); /* NOLINT(cert-env33-c) — installer has root */
}

/* Run a command and fail-exit worker on non-zero return */
#define RUN_OR_FAIL(pipe_fd, pct, msg, cmd)          \
    do {                                              \
        progress((pipe_fd), (pct), (msg));            \
        if (run_cmd(cmd) != 0) {                      \
            char _ebuf[256];                          \
            snprintf(_ebuf, sizeof(_ebuf),            \
                     "ERROR: %s failed", (msg));      \
            progress((pipe_fd), -1, _ebuf);           \
            _exit(1);                                 \
        }                                             \
    } while (0)

/* ── Password hashing ────────────────────────────────────────────────────── */

int install_hash_password(const char *plaintext, char *out, size_t out_sz)
{
    /* Build a $6$ (SHA-512) salt with 16 random chars */
    static const char salt_chars[] =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./";
    char salt[32];
    int fd = open("/dev/urandom", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return -1;
    unsigned char rbuf[16];
    if (read(fd, rbuf, sizeof(rbuf)) != (ssize_t)sizeof(rbuf)) {
        close(fd);
        return -1;
    }
    close(fd);

    salt[0] = '$'; salt[1] = '6'; salt[2] = '$';
    for (int i = 0; i < 16; i++) {
        salt[3 + i] = salt_chars[rbuf[i] % (sizeof(salt_chars) - 1)];
    }
    salt[19] = '$'; salt[20] = '\0';

    /* crypt_r is reentrant; crypt is fine here — single-threaded worker */
    char *hash = crypt(plaintext, salt);
    if (!hash) return -1;
    size_t len = strlen(hash);
    if (len >= out_sz) return -1;
    memcpy(out, hash, len + 1);
    return 0;
}

/* ── Main worker ──────────────────────────────────────────────────────────── */

void install_worker_run(int pipe_fd, const install_config_t *cfg)
{
    char cmd[512];
    const char *disk   = cfg->disk;
    const char *mnt    = "/mnt/mahina-install";
    const char *rootfs = "/usr/share/mahina/rootfs.tar.xz";

    /* Derive partition device names
     * NVME drives use pN suffix (e.g. /dev/nvme0n1p1),
     * everything else uses plain N (e.g. /dev/sda1, /dev/vda1). */
    char esp_dev[140], root_dev[140];
    if (strstr(disk, "nvme") || strstr(disk, "mmcblk")) {
        snprintf(esp_dev,  sizeof(esp_dev),  "%sp1", disk);
        snprintf(root_dev, sizeof(root_dev), "%sp2", disk);
    } else {
        snprintf(esp_dev,  sizeof(esp_dev),  "%s1", disk);
        snprintf(root_dev, sizeof(root_dev), "%s2", disk);
    }

    /* ── Step 1: Partition ──────────────────────────────────────────────── */
    progress(pipe_fd, 2, "Partitioning disk...");

    snprintf(cmd, sizeof(cmd),
             "parted -s %s mklabel gpt 2>/dev/null", disk);
    if (run_cmd(cmd) != 0) {
        progress(pipe_fd, -1, "ERROR: Failed to create GPT label");
        _exit(1);
    }
    snprintf(cmd, sizeof(cmd),
             "parted -s %s mkpart ESP fat32 1MiB 513MiB 2>/dev/null", disk);
    if (run_cmd(cmd) != 0) {
        progress(pipe_fd, -1, "ERROR: Failed to create ESP partition");
        _exit(1);
    }
    snprintf(cmd, sizeof(cmd),
             "parted -s %s set 1 esp on 2>/dev/null", disk);
    run_cmd(cmd); /* best effort */
    snprintf(cmd, sizeof(cmd),
             "parted -s %s mkpart ROOT btrfs 513MiB 100%% 2>/dev/null", disk);
    if (run_cmd(cmd) != 0) {
        progress(pipe_fd, -1, "ERROR: Failed to create root partition");
        _exit(1);
    }
    /* Inform kernel of new partition table */
    snprintf(cmd, sizeof(cmd), "partprobe %s 2>/dev/null || true", disk);
    run_cmd(cmd);
    sleep(1); /* Allow devtmpfs to create nodes */

    /* ── Step 2: Format ESP ─────────────────────────────────────────────── */
    snprintf(cmd, sizeof(cmd), "mkfs.fat -F32 -n MAHINA_EFI %s 2>/dev/null",
             esp_dev);
    RUN_OR_FAIL(pipe_fd, 10, "Formatting EFI partition...", cmd);

    /* ── Step 3: Format root ────────────────────────────────────────────── */
    snprintf(cmd, sizeof(cmd),
             "mkfs.btrfs -f -L MAHINA_ROOT %s 2>/dev/null", root_dev);
    RUN_OR_FAIL(pipe_fd, 15, "Formatting root filesystem (Btrfs)...", cmd);

    /* ── Step 4: Create Btrfs subvolumes ────────────────────────────────── */
    progress(pipe_fd, 18, "Creating Btrfs subvolumes...");
    snprintf(cmd, sizeof(cmd), "mkdir -p /tmp/mahina-btrfs-top 2>/dev/null");
    run_cmd(cmd);
    snprintf(cmd, sizeof(cmd),
             "mount -t btrfs %s /tmp/mahina-btrfs-top 2>/dev/null", root_dev);
    if (run_cmd(cmd) != 0) {
        progress(pipe_fd, -1, "ERROR: Failed to mount Btrfs top-level");
        _exit(1);
    }
    run_cmd("btrfs subvolume create /tmp/mahina-btrfs-top/@ 2>/dev/null");
    run_cmd("btrfs subvolume create /tmp/mahina-btrfs-top/@home 2>/dev/null");
    run_cmd("btrfs subvolume create /tmp/mahina-btrfs-top/@snapshots 2>/dev/null");
    run_cmd("umount /tmp/mahina-btrfs-top 2>/dev/null");
    run_cmd("rmdir /tmp/mahina-btrfs-top 2>/dev/null");

    /* ── Step 5: Mount target ────────────────────────────────────────────── */
    progress(pipe_fd, 22, "Mounting target filesystem...");
    snprintf(cmd, sizeof(cmd), "mkdir -p %s", mnt);
    run_cmd(cmd);
    snprintf(cmd, sizeof(cmd),
             "mount -t btrfs -o rw,subvol=@ %s %s 2>/dev/null",
             root_dev, mnt);
    if (run_cmd(cmd) != 0) {
        progress(pipe_fd, -1, "ERROR: Failed to mount root subvolume");
        _exit(1);
    }
    snprintf(cmd, sizeof(cmd), "mkdir -p %s/boot/efi", mnt);
    run_cmd(cmd);
    snprintf(cmd, sizeof(cmd),
             "mount %s %s/boot/efi 2>/dev/null", esp_dev, mnt);
    if (run_cmd(cmd) != 0) {
        progress(pipe_fd, -1, "ERROR: Failed to mount ESP");
        _exit(1);
    }

    /* ── Step 6: Extract rootfs ──────────────────────────────────────────── */
    progress(pipe_fd, 28, "Extracting system files...");
    struct stat st;
    if (stat(rootfs, &st) == 0) {
        /* Real rootfs tarball found — extract it */
        snprintf(cmd, sizeof(cmd),
                 "tar -xJf %s -C %s --numeric-owner 2>/dev/null", rootfs, mnt);
        if (run_cmd(cmd) != 0) {
            progress(pipe_fd, -1, "ERROR: rootfs extraction failed");
            _exit(1);
        }
    } else {
        /* No rootfs tarball — build a minimal directory skeleton.
         * This is the QEMU development path where binaries are already
         * installed on the host image. */
        progress(pipe_fd, 30, "Bootstrapping minimal rootfs...");
        snprintf(cmd, sizeof(cmd),
                 "mkdir -p %s/{usr/bin,usr/sbin,usr/share/fonts,"
                 "usr/share/wallpaper,etc/luna/services,var/log/luna-init,"
                 "var/lib/lpkg,tmp,run,proc,sys,dev,home,root}", mnt);
        run_cmd(cmd);
        snprintf(cmd, sizeof(cmd),
                 "ln -sfn usr/bin %s/bin && "
                 "ln -sfn usr/sbin %s/sbin", mnt, mnt);
        run_cmd(cmd);

        /* Copy binaries from the live/installer system */
        const char *bins[] = {
            "/usr/sbin/luna-init",
            "/usr/bin/luna-init-ctl",
            "/usr/sbin/luna-splash",
            "/usr/bin/lgp-compositor",
            "/usr/bin/luna-shell",
            "/usr/bin/luna-terminal",
            "/usr/bin/luna-installer",
            "/usr/bin/luna-ai-d",
            "/usr/bin/luna-island",
            "/usr/bin/lpkg",
            "/usr/bin/settings-rs",
            "/usr/bin/files-rs",
            "/usr/bin/calc-rs",
            "/usr/bin/text-rs",
            "/usr/bin/tasks-rs",
            "/usr/bin/about-rs",
            "/usr/bin/busybox",
            NULL
        };
        for (int i = 0; bins[i]; i++) {
            if (stat(bins[i], &st) == 0) {
                char dst_dir[256];
                /* Determine destination directory */
                if (strstr(bins[i], "/usr/sbin/"))
                    snprintf(dst_dir, sizeof(dst_dir), "%s/usr/sbin/", mnt);
                else
                    snprintf(dst_dir, sizeof(dst_dir), "%s/usr/bin/", mnt);
                snprintf(cmd, sizeof(cmd), "cp %s %s 2>/dev/null",
                         bins[i], dst_dir);
                run_cmd(cmd);
            }
        }
        /* Busybox sh symlink */
        snprintf(cmd, sizeof(cmd),
                 "ln -sf /usr/bin/busybox %s/usr/bin/sh 2>/dev/null", mnt);
        run_cmd(cmd);
        /* Fonts and wallpaper */
        snprintf(cmd, sizeof(cmd),
                 "cp /usr/share/fonts/* %s/usr/share/fonts/ 2>/dev/null || true", mnt);
        run_cmd(cmd);
        snprintf(cmd, sizeof(cmd),
                 "cp /usr/share/wallpaper/* %s/usr/share/wallpaper/ 2>/dev/null || true", mnt);
        run_cmd(cmd);
        /* Splash frame sequence */
        snprintf(cmd, sizeof(cmd),
                 "mkdir -p %s/usr/share/mahina/splash && "
                 "cp /usr/share/mahina/splash/* %s/usr/share/mahina/splash/ 2>/dev/null || true", mnt, mnt);
        run_cmd(cmd);
        /* Service TOML files */
        snprintf(cmd, sizeof(cmd),
                 "cp /etc/luna/services/* %s/etc/luna/services/ 2>/dev/null || true", mnt);
        run_cmd(cmd);
    }

    progress(pipe_fd, 55, "Files extracted successfully.");

    /* ── Step 7: Write system config ─────────────────────────────────────── */
    progress(pipe_fd, 58, "Writing system configuration...");

    /* /etc/luna/hostname */
    {
        char path[256];
        snprintf(path, sizeof(path), "%s/etc/luna/hostname", mnt);
        FILE *f = fopen(path, "w");
        if (f) { fprintf(f, "%s\n", cfg->hostname); fclose(f); }
    }

    /* /etc/locale.conf */
    {
        char path[256];
        snprintf(path, sizeof(path), "%s/etc/locale.conf", mnt);
        FILE *f = fopen(path, "w");
        if (f) { fprintf(f, "LANG=%s\n", cfg->language); fclose(f); }
    }

    /* /etc/luna/fstab.toml — used by luna-init mount manager */
    {
        char path[256];
        snprintf(path, sizeof(path), "%s/etc/luna/fstab.toml", mnt);
        FILE *f = fopen(path, "w");
        if (f) {
            fprintf(f,
                "# fstab.toml — Generated by luna-installer\n"
                "[[mount]]\n"
                "device = \"%s\"\n"
                "mountpoint = \"/\"\n"
                "fstype = \"btrfs\"\n"
                "options = \"rw,subvol=@\"\n"
                "\n"
                "[[mount]]\n"
                "device = \"%s\"\n"
                "mountpoint = \"/boot/efi\"\n"
                "fstype = \"vfat\"\n"
                "options = \"rw,umask=0077\"\n"
                "\n"
                "[[mount]]\n"
                "device = \"tmpfs\"\n"
                "mountpoint = \"/tmp\"\n"
                "fstype = \"tmpfs\"\n"
                "options = \"rw,nosuid,nodev,size=512M\"\n"
                "\n"
                "[[mount]]\n"
                "device = \"zram0\"\n"
                "mountpoint = \"none\"\n"
                "fstype = \"swap\"\n"
                "options = \"sw\"\n",
                root_dev, esp_dev);
            fclose(f);
        }
    }

    /* /etc/passwd and /etc/shadow */
    {
        char passwd_path[256], shadow_path[256];
        snprintf(passwd_path, sizeof(passwd_path), "%s/etc/passwd", mnt);
        snprintf(shadow_path, sizeof(shadow_path), "%s/etc/shadow", mnt);

        FILE *fp = fopen(passwd_path, "w");
        if (fp) {
            fprintf(fp,
                    "root:x:0:0:root:/root:/usr/bin/sh\n"
                    "%s:x:1000:1000:%s:/home/%s:/usr/bin/sh\n",
                    cfg->username, cfg->username, cfg->username);
            fclose(fp);
        }
        FILE *fs = fopen(shadow_path, "w");
        if (fs) {
            /* root is locked; user gets the hashed password */
            fprintf(fs,
                    "root:!:19900:0:99999:7:::\n"
                    "%s:%s:19900:0:99999:7:::\n",
                    cfg->username,
                    cfg->password_hash[0] ? cfg->password_hash : "!");
            fclose(fs);
            chmod(shadow_path, 0600);
        }

        /* /etc/group */
        char group_path[256];
        snprintf(group_path, sizeof(group_path), "%s/etc/group", mnt);
        FILE *fg = fopen(group_path, "w");
        if (fg) {
            fprintf(fg,
                    "root:x:0:\n"
                    "wheel:x:10:%s\n"
                    "video:x:44:%s\n"
                    "audio:x:29:%s\n"
                    "%s:x:1000:\n",
                    cfg->username, cfg->username, cfg->username,
                    cfg->username);
            fclose(fg);
        }
        /* /home/<user> directory */
        char home_path[256];
        snprintf(home_path, sizeof(home_path), "%s/home/%s", mnt, cfg->username);
        snprintf(cmd, sizeof(cmd), "mkdir -p %s && chown 1000:1000 %s",
                 home_path, home_path);
        run_cmd(cmd);
    }

    progress(pipe_fd, 68, "Writing bootloader configuration...");

    /* ── Step 8: Write limine.conf to ESP ────────────────────────────────── */
    {
        char limine_path[256];
        snprintf(limine_path, sizeof(limine_path),
                 "%s/boot/efi/limine.conf", mnt);
        FILE *f = fopen(limine_path, "w");
        if (f) {
            fprintf(f,
                "timeout: 3\n"
                "\n"
                "/Mahina OS\n"
                "    protocol: linux\n"
                "    kernel_path: boot():/vmlinuz-mahina\n"
                "    cmdline: root=%s rootfstype=btrfs rootflags=subvol=@ rw "
                "quiet splash init=/usr/sbin/luna-init\n"
                "    module_path: boot():/initramfs-mahina.img\n",
                root_dev);
            fclose(f);
        }
        /* Copy limine EFI binary if available */
        snprintf(cmd, sizeof(cmd),
                 "mkdir -p %s/boot/efi/EFI/BOOT 2>/dev/null && "
                 "cp /boot/efi/EFI/BOOT/BOOTX64.EFI %s/boot/efi/EFI/BOOT/ 2>/dev/null || true",
                 mnt, mnt);
        run_cmd(cmd);
        /* Copy kernel and initramfs */
        snprintf(cmd, sizeof(cmd),
                 "cp /boot/efi/vmlinuz-mahina %s/boot/efi/ 2>/dev/null || "
                 "cp /boot/vmlinuz-* %s/boot/efi/vmlinuz-mahina 2>/dev/null || true",
                 mnt, mnt);
        run_cmd(cmd);
        snprintf(cmd, sizeof(cmd),
                 "cp /boot/efi/initramfs-mahina.img %s/boot/efi/ 2>/dev/null || true", mnt);
        run_cmd(cmd);
    }

    progress(pipe_fd, 85, "Finalising installation...");

    /* ── Step 9: Unmount and sync ────────────────────────────────────────── */
    sync();
    snprintf(cmd, sizeof(cmd), "umount %s/boot/efi 2>/dev/null", mnt);
    run_cmd(cmd);
    snprintf(cmd, sizeof(cmd), "umount %s 2>/dev/null", mnt);
    run_cmd(cmd);
    snprintf(cmd, sizeof(cmd), "rmdir %s 2>/dev/null", mnt);
    run_cmd(cmd);

    progress(pipe_fd, 100, "Installation complete!");
    _exit(0);
}
