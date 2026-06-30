#!/bin/bash
losetup -D
LOOP=$(losetup -fP --show build/mahina-0.1.0.img)
MNT=$(mktemp -d)
mount "${LOOP}p2" "$MNT"
cp "$MNT/@/var/log/luna-init/boot.log" /mnt/c/Users/sneha/Music/MahinaOS/boot.log
umount "$MNT"
losetup -d "$LOOP"
rmdir "$MNT"
