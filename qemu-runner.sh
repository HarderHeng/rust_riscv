#!/usr/bin/env bash
# cargo run -- gdb      → launch QEMU suspended, waiting for GDB on :1234
# cargo run -- virtio   → launch with virtio-console connected to stdio
# cargo run             → launch QEMU normally
set -e

if [[ $# -eq 0 ]]; then
    echo "usage: $0 <kernel-elf> [gdb|virtio]" >&2
    echo "  This script is invoked by cargo as the QEMU runner." >&2
    echo "  Run 'cargo run -p qemu-virt-rv32 --release' instead." >&2
    exit 1
fi

KERNEL="$1"
shift

if [[ "${1:-}" == "virtio" ]]; then
    shift
    exec qemu-system-riscv32 \
        -machine virt \
        -global virtio-mmio.force-legacy=false \
        -nographic \
        -monitor none \
        -serial none \
        -bios none \
        -kernel "$KERNEL" \
        -chardev stdio,id=virtio0,mux=on,signal=off \
        -device virtio-serial-device \
        -device virtconsole,chardev=virtio0 \
        "$@"
elif [[ "${1:-}" == "gdb" ]]; then
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
