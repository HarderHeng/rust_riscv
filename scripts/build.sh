#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BLRI_DIR="${BLRI_DIR:-$ROOT/../bouffalo-hal}"
M0_TARGET=riscv32imac-unknown-none-elf
D0_TARGET=riscv64imac-unknown-none-elf
M0_ELF="$ROOT/target/$M0_TARGET/release/rust-helloworld-m0"
D0_ELF="$ROOT/target/$D0_TARGET/release/rust-helloworld-d0"
M0_BIN="$M0_ELF.bin"
D0_BIN="$D0_ELF.bin"

cd "$ROOT"

find_blri() {
    if [[ -x "$BLRI_DIR/target/x86_64-unknown-linux-gnu/release/blri" ]]; then
        echo "$BLRI_DIR/target/x86_64-unknown-linux-gnu/release/blri"
    elif [[ -x "$BLRI_DIR/target/release/blri" ]]; then
        echo "$BLRI_DIR/target/release/blri"
    else
        echo ""
    fi
}

BLRI="$(find_blri)"
if [[ -z "$BLRI" ]]; then
    echo "building blri to patch boot header hash..."
    cargo build --release -p blri --manifest-path "$BLRI_DIR/Cargo.toml"
    BLRI="$(find_blri)"
fi
[[ -n "$BLRI" ]] || { echo "找不到 blri" >&2; exit 1; }

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
rust-objcopy --binary-architecture=riscv32 --strip-all -O binary "$M0_ELF" "$M0_BIN"
rust-objcopy --binary-architecture=riscv64 --strip-all -O binary "$D0_ELF" "$D0_BIN"

# objcopy may append padding past img_len. blri hashes to EOF, BootROM hashes
# only img_len bytes — that mismatch is why previous M0 images never ran.
truncate_to_img_len "$M0_BIN"
truncate_to_img_len "$D0_BIN"
"$BLRI" patch "$M0_BIN"
"$BLRI" patch "$D0_BIN"
verify_hash "$M0_BIN"
verify_hash "$D0_BIN"

echo
echo "M0 BIN: $M0_BIN"
echo "D0 BIN: $D0_BIN"
ls -l "$M0_BIN" "$D0_BIN"
echo
echo "烧录（先进入下载模式：BOOT+RST，先松 RST，再松 BOOT）："
echo "  $ROOT/../flash_m1s.sh -y --m0 $M0_BIN --d0 $D0_BIN"
