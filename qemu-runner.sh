#!/usr/bin/env bash
# cargo run -- gdb   →  launch QEMU suspended, waiting for GDB on :1234
# cargo run          →  launch QEMU normally
set -e

KERNEL="$1"
shift

if [[ "${1:-}" == "gdb" ]]; then
    exec qemu-system-riscv32 \
        -machine virt \
        -nographic \
        -bios none \
        -kernel "$KERNEL" \
        -s -S
else
    exec qemu-system-riscv32 \
        -machine virt \
        -nographic \
        -bios none \
        -kernel "$KERNEL" \
        "$@"
fi
