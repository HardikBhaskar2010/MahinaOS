# Credits

Mahina builds upon decades of open-source work, research, and engineering. While we are writing the core init system and graphics stack from scratch, we stand on the shoulders of giants.

We would like to extend our special thanks and immense gratitude to the following projects and their communities:

- **The Linux Kernel Community:** For providing the robust, secure, and infinitely configurable foundation upon which Mahina runs.
- **Limine Bootloader:** An elegant, modern, and advanced x86/x86_64 BIOS and UEFI bootloader that makes early boot a joy rather than a chore.
- **BusyBox:** The Swiss Army Knife of Embedded Linux, providing our early-boot shell environment during Stage 0 bring-up.
- **The LLVM/Clang Project:** Our primary compiler toolchain, providing incredible static analysis, `clang-tidy`, and the sanitizers (ASan/UBSan) that keep Mahina memory-safe.
- **QEMU:** For providing the lightning-fast virtualization environment we use for daily testing and development.
- **OVMF (Open Virtual Machine Firmware):** For enabling UEFI support in QEMU, allowing us to test modern hardware boot paths.
