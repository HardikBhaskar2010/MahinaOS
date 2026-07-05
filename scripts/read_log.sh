#!/bin/bash
losetup -D
LOOP=$(losetup -fP --show build/mahina-0.1.0.img)
MNT=$(mktemp -d)
mount "${LOOP}p2" "$MNT"
mkdir -p build
cp "$MNT/@/var/log/luna-init/boot.log" build/boot.log
umount "$MNT"
losetup -d "$LOOP"
rmdir "$MNT"
