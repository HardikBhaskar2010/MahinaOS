#!/usr/bin/env bash
# run-qemu.sh — Launch Mahina OS in QEMU
# Authority: Volume II / 02_boot_flow.md (QEMU Target device = /dev/vda)

set -e

VERSION="0.1.0"
BUILD_DIR="$(pwd)/build"
IMG_FILE="${BUILD_DIR}/mahina-${VERSION}.img"

if [ ! -f "$IMG_FILE" ]; then
    echo "ERROR: Image $IMG_FILE not found."
    echo "Run build-image.sh first."
    exit 1
fi

echo "  QEMU    Starting Mahina OS (UEFI)"

# Check for OVMF firmware (UEFI)
OVMF_PATH=""
for path in \
    "/usr/share/OVMF/OVMF_CODE.fd" \
    "/usr/share/qemu/OVMF.fd" \
    "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd" \
    "/usr/share/edk2/ovmf/OVMF_CODE.fd"; do
    if [ -f "$path" ]; then
        OVMF_PATH="$path"
        break
    fi
done

if [ -z "$OVMF_PATH" ]; then
    echo "ERROR: OVMF UEFI firmware not found."
    echo "Please install ovmf / edk2-ovmf package."
    exit 1
fi

echo "  QEMU    Using OVMF: $OVMF_PATH"

qemu-system-x86_64 \
    -enable-kvm \
    -m 4G \
    -cpu host \
    -smp 4 \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_PATH" \
    -drive file="$IMG_FILE",format=raw,if=virtio \
    -netdev user,id=net0 \
    -device virtio-net-pci,netdev=net0 \
    -vga virtio \
    -display gtk \
    -serial mon:stdio
