#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Keep in sync with workspace.dependencies in Cargo.toml.
HAL_GIT="${HAL_GIT:-https://github.com/rustsbi/bouffalo-hal}"
HAL_REV="${HAL_REV:-ea477a96b6c43ec906b9e29abd56c7c5e2338ca2}"
HOST_ROOT="$ROOT/target/host-tools"
M0_TARGET=riscv32imac-unknown-none-elf
D0_TARGET=riscv64imac-unknown-none-elf
M0_ELF="$ROOT/target/$M0_TARGET/release/rust-helloworld-m0"
D0_ELF="$ROOT/target/$D0_TARGET/release/rust-helloworld-d0"
STAGE0_ELF="$ROOT/target/$D0_TARGET/release/stage0"
SBI0_ELF="$ROOT/target/$D0_TARGET/release/sbi0"
M0_BIN="$M0_ELF.bin"
D0_BIN="$D0_ELF.bin"
STAGE0_BIN="$STAGE0_ELF.bin"
SBI0_BIN="$SBI0_ELF.bin"

cd "$ROOT"

ensure_blri() {
    if [[ -n "${BLRI:-}" && -x "$BLRI" ]]; then
        return
    fi
    if command -v blri >/dev/null 2>&1; then
        BLRI="$(command -v blri)"
        return
    fi
    BLRI="$HOST_ROOT/bin/blri"
    if [[ ! -x "$BLRI" ]]; then
        echo "installing blri from $HAL_GIT @$HAL_REV ..."
        cargo install blri --git "$HAL_GIT" --rev "$HAL_REV" --root "$HOST_ROOT"
    fi
    [[ -x "$BLRI" ]] || { echo "找不到 blri" >&2; exit 1; }
}

ensure_blri

if ! rustup target list --installed | grep -qx "$M0_TARGET"; then
    rustup target add "$M0_TARGET"
fi
if ! rustup target list --installed | grep -qx "$D0_TARGET"; then
    rustup target add "$D0_TARGET"
fi

truncate_to_img_len() {
    python3 - "$1" <<'PY'
import struct, sys
from pathlib import Path
p = Path(sys.argv[1])
data = bytearray(p.read_bytes())
group = struct.unpack_from("<I", data, 0x84)[0]
img_len = struct.unpack_from("<I", data, 0x8C)[0]
need = group + img_len
if len(data) < need:
    raise SystemExit(f"{p}: file {len(data)} < header+payload {need}")
if len(data) != need:
    print(f"truncate {p.name}: {len(data)} -> {need} (drop {len(data) - need} tail bytes)")
    p.write_bytes(data[:need])
PY
}

verify_hash() {
    python3 - "$1" <<'PY'
import hashlib, struct, sys
from pathlib import Path
data = Path(sys.argv[1]).read_bytes()
group = struct.unpack_from("<I", data, 0x84)[0]
img_len = struct.unpack_from("<I", data, 0x8C)[0]
header_hash = data[0x90:0xB0]
payload_hash = hashlib.sha256(data[group:group + img_len]).digest()
if header_hash != payload_hash:
    raise SystemExit(
        f"{sys.argv[1]} hash mismatch: header={header_hash.hex()} payload={payload_hash.hex()}"
    )
print(f"hash ok  {Path(sys.argv[1]).name}  group=0x{group:x} img_len={img_len} file={len(data)}")
PY
}

cargo build -p rust-helloworld-m0 --release --target "$M0_TARGET"
cargo build -p rust-helloworld-d0 --release --target "$D0_TARGET"
cargo build -p stage0 --release --target "$D0_TARGET"
cargo build -p sbi0 --release --target "$D0_TARGET"
rust-objcopy --binary-architecture=riscv32 --strip-all -O binary "$M0_ELF" "$M0_BIN"
rust-objcopy --binary-architecture=riscv64 --strip-all -O binary "$D0_ELF" "$D0_BIN"
rust-objcopy --binary-architecture=riscv64 --strip-all -O binary "$STAGE0_ELF" "$STAGE0_BIN"
rust-objcopy --binary-architecture=riscv64 --strip-all -O binary "$SBI0_ELF" "$SBI0_BIN"

# objcopy may append padding past img_len. blri hashes to EOF, BootROM hashes
# only img_len bytes — that mismatch is why previous M0 images never ran.
truncate_to_img_len "$M0_BIN"
truncate_to_img_len "$D0_BIN"
truncate_to_img_len "$STAGE0_BIN"
truncate_to_img_len "$SBI0_BIN"
"$BLRI" patch "$M0_BIN"
"$BLRI" patch "$D0_BIN"
"$BLRI" patch "$STAGE0_BIN"
"$BLRI" patch "$SBI0_BIN"
verify_hash "$M0_BIN"
verify_hash "$D0_BIN"
verify_hash "$STAGE0_BIN"
verify_hash "$SBI0_BIN"

echo
echo "M0 BIN:     $M0_BIN"
echo "D0 BIN:     $D0_BIN"
echo "STAGE0 BIN: $STAGE0_BIN"
echo "SBI0 BIN:   $SBI0_BIN"
ls -l "$M0_BIN" "$D0_BIN" "$STAGE0_BIN" "$SBI0_BIN"
echo
echo "烧录 sbi0（BOOT+RST，先松 RST，再松 BOOT）："
echo "  flash_m1s.sh -y --m0 $M0_BIN --d0 $SBI0_BIN"
