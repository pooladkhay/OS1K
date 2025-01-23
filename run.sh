#!/bin/bash
set -xue

QEMU=qemu-system-riscv32

CC=clang
CFLAGS="-std=c11 -O2 -g3 -Wall -Wextra --target=riscv32 -ffreestanding -nostdlib"

$CC $CFLAGS -Wl,-Tkernel.ld -Wl,-Map=kernel.map -o kernel.elf kernel.c common.c

# Start QEMU
$QEMU -monitor stdio -machine virt -bios default --no-reboot -kernel kernel.elf
# $QEMU -machine virt -bios default -nographic -serial mon:stdio --no-reboot -kernel kernel.elf