#!/usr/bin/env bash
# QEMU runner script for RISC-V 32-bit virt machine

set -e

BINARY="$1"

# Check if GDB mode is requested
if [ "$2" = "gdb" ]; then
    echo "Starting QEMU in GDB mode (listening on port 1234)..."
    echo "Connect with: riscv32-unknown-elf-gdb $BINARY"
    echo "              (gdb) target remote :1234"
    echo "              (gdb) continue"
    exec qemu-system-riscv32 \
        -machine virt \
        -bios none \
        -kernel "$BINARY" \
        -serial mon:stdio \
        -nographic \
        -s -S
else
    # Normal execution mode
    exec qemu-system-riscv32 \
        -machine virt \
        -bios none \
        -kernel "$BINARY" \
        -serial mon:stdio \
        -nographic
fi
