#!/usr/bin/env bash
# cargo run -- gdb   →  launch QEMU suspended, waiting for GDB on :1234
# cargo run          →  launch QEMU normally
set -e

if [[ $# -eq 0 ]]; then
    echo "usage: $0 <kernel-elf> [gdb]" >&2
    echo "  This script is invoked by cargo as the QEMU runner." >&2
    echo "  Run 'cargo run -p qemu-virt-rv32 --release' instead." >&2
    exit 1
fi

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
