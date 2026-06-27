#!/usr/bin/env bash
# build-kernel.sh — Compile a custom monolithic Linux kernel for Mahina OS
set -e

KERNEL_VERSION="6.9.7"
BUILD_DIR="/tmp/mahina-kernel-build"
MAHINA_VMLINUZ="$(pwd)/build/vmlinuz-mahina"

echo "  KERNEL  Downloading Linux ${KERNEL_VERSION}..."
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

if [ ! -f "linux-${KERNEL_VERSION}.tar.xz" ]; then
    wget -q "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-${KERNEL_VERSION}.tar.xz"
fi

if [ ! -d "linux-${KERNEL_VERSION}" ]; then
    echo "  KERNEL  Extracting source..."
    tar -xf "linux-${KERNEL_VERSION}.tar.xz"
fi

cd "linux-${KERNEL_VERSION}"

echo "  KERNEL  Configuring monolithic kernel..."
make defconfig >/dev/null
make kvm_guest.config >/dev/null

# Ensure required features are BUILT-IN (not modules)
scripts/config --enable CONFIG_BTRFS_FS
scripts/config --enable CONFIG_BTRFS_FS_POSIX_ACL
scripts/config --enable CONFIG_BTRFS_FS_CHECK_INTEGRITY
scripts/config --enable CONFIG_LIBCRC32C
scripts/config --enable CONFIG_VIRTIO_BLK
scripts/config --enable CONFIG_VIRTIO_NET
scripts/config --enable CONFIG_VIRTIO_PCI
scripts/config --enable CONFIG_SCSI_VIRTIO
scripts/config --enable CONFIG_SCSI
scripts/config --enable CONFIG_BLK_DEV_SD
scripts/config --enable CONFIG_DEVTMPFS
scripts/config --enable CONFIG_DEVTMPFS_MOUNT
scripts/config --enable CONFIG_CRYPTO_CRC32C
scripts/config --disable CONFIG_WERROR

echo "  KERNEL  Updating configuration..."
make CC=gcc-13 HOSTCC=gcc-13 olddefconfig >/dev/null

echo "  KERNEL  Building kernel (this will take 1-3 minutes)..."
make -j$(nproc) CC=gcc-13 HOSTCC=gcc-13 bzImage >/dev/null

echo "  KERNEL  Copying bzImage to $MAHINA_VMLINUZ"
cp arch/x86/boot/bzImage "$MAHINA_VMLINUZ"
echo "  KERNEL  Build complete!"
