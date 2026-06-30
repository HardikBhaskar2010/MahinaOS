#!/usr/bin/env bash
set -e
IMG="$(pwd)/build/mahina-0.1.0.img"
LOOP=$(losetup --find --partscan --show "$IMG")
sleep 1
MNT=$(mktemp -d)
mount "${LOOP}p1" "$MNT"
echo "=== ESP contents ==="
find "$MNT" -type f -exec ls -l {} + | sort
umount "$MNT"
rmdir "$MNT"
losetup -d "$LOOP"
