#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ELF="$ROOT/target/riscv64imac-unknown-none-elf/release/sbi0"
BIN="$ELF.bin"
[[ -f "$ELF" ]] || { echo "missing $ELF" >&2; exit 1; }
[[ -f "$BIN" ]] || { echo "missing $BIN" >&2; exit 1; }
head -c 4 "$BIN" | grep BFNP >/dev/null || { echo "BIN missing BFNP header" >&2; exit 1; }
NM="${NM:-$(command -v llvm-nm || true)}"
OBJDUMP="${OBJDUMP:-$(command -v llvm-objdump || true)}"
if [[ -z "$NM" ]]; then NM="$(command -v llvm-nm-18 || true)"; fi
if [[ -z "$OBJDUMP" ]]; then OBJDUMP="$(command -v llvm-objdump-18 || true)"; fi
[[ -n "$NM" && -n "$OBJDUMP" ]] || { echo "need llvm-nm / llvm-objdump" >&2; exit 1; }
"$NM" "$ELF" | grep ' s_main$' >/dev/null || { echo "ELF missing s_main" >&2; exit 1; }
"$NM" "$ELF" | grep ' trap_m$' >/dev/null || { echo "ELF missing trap_m" >&2; exit 1; }
"$NM" "$ELF" | grep ' trap_handle$' >/dev/null || { echo "ELF missing trap_handle" >&2; exit 1; }
"$NM" "$ELF" | grep ' trap_s$' >/dev/null || { echo "ELF missing trap_s" >&2; exit 1; }
"$OBJDUMP" -d "$ELF" | grep -E '[[:space:]]mret' >/dev/null || { echo "ELF missing mret" >&2; exit 1; }
"$OBJDUMP" -d "$ELF" | grep -E '[[:space:]]ecall' >/dev/null || { echo "ELF missing ecall" >&2; exit 1; }
echo "check-sbi0 ok"
