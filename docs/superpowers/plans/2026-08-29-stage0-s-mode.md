# stage0 S-mode Print Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** D0 `mret` into S-mode and print `[S] hello from supervisor on C906` on UART3, using the already-verified `helloworld-m0` loader.

**Architecture:** New `stage0` crate copies helloworld-d0’s M-mode UART/IPC/I-cache path, then opens PMP, clears `satp`, sets `mstatus.MPP=S`, and `mret`s to `s_main`. S-mode writes UART3 FIFO at `0x30002000+0x88` only — no HAL. `helloworld-m0` / `helloworld-d0` stay unchanged.

**Tech Stack:** Rust `no_std`, `bouffalo-rt` `bl808-dsp`, `bouffalo-hal` git rev `ea477a96`, `scripts/build.sh` + `blri`, M1s Dock ACM0 at 2000000 8N1.

**Spec:** `docs/superpowers/specs/2026-08-29-stage0-s-mode-design.md`

---

## File map

| File | Role |
|------|------|
| `Cargo.toml` | Add workspace member `stage0` |
| `stage0/Cargo.toml` | Crate manifest, same deps as helloworld-d0 |
| `stage0/build.rs` | Link `bouffalo-rt.ld` |
| `stage0/src/main.rs` | M-mode init + `mret` + S-mode FIFO print |
| `scripts/build.sh` | Build/patch `stage0.bin`; flash hint uses m0+stage0 |
| `scripts/check-stage0.sh` | Host checks: `BFNP`, `s_main`, `mret` |
| `README.md` | Flash stage0 + expected lines |
| `docs/architecture.md` | stage0 flow |
| `docs/chapter-status.md` | Ch1 status |
| `docs/memory-map.md` | Flash table |
| `AGENTS.md` | S-mode / PMP rules |

Do not edit `helloworld-m0/` or `helloworld-d0/`.

There is no host RISC-V S-mode simulator in this repo. Host “tests” are: crate builds, image hash, ELF contains `s_main` and `mret`. Pass/fail of S-mode is ACM0 on hardware.

---

### Task 1: Add the `stage0` crate so it compiles

**Files:**
- Modify: `Cargo.toml`
- Create: `stage0/Cargo.toml`
- Create: `stage0/build.rs`
- Create: `stage0/src/main.rs` (M-mode UART up + hang; no `mret` yet)

- [ ] **Step 1: Add the workspace member**

In `Cargo.toml`, set:

```toml
[workspace]
members = ["helloworld-m0", "helloworld-d0", "stage0"]
resolver = "3"

[workspace.dependencies]
bouffalo-hal = { git = "https://github.com/rustsbi/bouffalo-hal", rev = "ea477a96b6c43ec906b9e29abd56c7c5e2338ca2" }
bouffalo-rt = { git = "https://github.com/rustsbi/bouffalo-hal", rev = "ea477a96b6c43ec906b9e29abd56c7c5e2338ca2", default-features = false }
```

- [ ] **Step 2: Write `stage0/Cargo.toml`**

```toml
[package]
name = "stage0"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
bouffalo-hal = { workspace = true, features = ["bl808"] }
panic-halt = "1.0.0"
riscv = "0.13.0"
embedded-time = "0.12.1"
bouffalo-rt = { workspace = true, features = ["bl808-dsp"] }

[[bin]]
name = "stage0"
test = false
bench = false
```

- [ ] **Step 3: Write `stage0/build.rs`**

```rust
fn main() {
    println!("cargo:rustc-link-arg=-Tbouffalo-rt.ld");
}
```

- [ ] **Step 4: Write a compiling M-mode stub `stage0/src/main.rs`**

Copy helpers from `helloworld-d0/src/main.rs`. `main` must print `uart up`, wait IPC, print `ipc synced`, enable I-cache, print `icache on`, then `loop { spin_loop() }`. Do **not** call `mret` yet. Do **not** include the 5s LED loop. Use these exact strings:

- `rust helloworld from D0/C906, uart up`
- `rust helloworld from D0/C906, ipc synced`
- `rust helloworld from D0/C906, icache on`

Full file:

```rust
#![no_std]
#![no_main]

use bouffalo_hal::{prelude::*, uart::Config};
use bouffalo_rt::{Clocks, Peripherals, entry};
use core::ptr::{read_volatile, write_volatile};
use embedded_time::rate::*;
use panic_halt as _;

const XTAL_HZ: u32 = 40_000_000;
const MM_GLB_BASE: usize = 0x3000_7000;
const MM_CLK_CTRL_CPU: usize = MM_GLB_BASE;
const MM_CLK_CTRL_PERI: usize = MM_GLB_BASE + 0x10;
const DSP_UART_CLK_XCLK: u32 = 2;
const IPC_SYNC_ADDR1: usize = 0x4000_0000;
const IPC_SYNC_ADDR2: usize = 0x4000_0004;
const IPC_SYNC_FLAG: u32 = 0x1234_5678;

struct Uart3Xclk;

impl bouffalo_hal::uart::Clock for Uart3Xclk {
    fn uart_clock<const I: usize>(self) -> Hertz {
        Hertz(XTAL_HZ)
    }
}

#[entry]
fn main(p: Peripherals, _c: Clocks) -> ! {
    enable_uart3_clock();
    let tx = p.gpio.io16.into_mm_uart();
    let rx = p.gpio.io17.into_mm_uart();
    let config = Config::default().set_baudrate(2_000_000.Bd());
    let mut serial = p.uart3.freerun(config, (tx, rx), Uart3Xclk).unwrap();

    writeln!(serial, "rust helloworld from D0/C906, uart up").ok();
    serial.flush().ok();

    wait_for_m0();
    writeln!(serial, "rust helloworld from D0/C906, ipc synced").ok();
    serial.flush().ok();

    enable_icache();
    writeln!(serial, "rust helloworld from D0/C906, icache on").ok();
    serial.flush().ok();

    loop {
        core::hint::spin_loop();
    }
}

fn enable_uart3_clock() {
    unsafe {
        let cpu = read_volatile(MM_CLK_CTRL_CPU as *const u32);
        write_volatile(
            MM_CLK_CTRL_CPU as *mut u32,
            (cpu & !(0x3 << 4)) | (DSP_UART_CLK_XCLK << 4),
        );
        let peri = read_volatile(MM_CLK_CTRL_PERI as *const u32);
        write_volatile(MM_CLK_CTRL_PERI as *mut u32, peri & !(1 << 16));
        let peri = read_volatile(MM_CLK_CTRL_PERI as *const u32);
        write_volatile(
            MM_CLK_CTRL_PERI as *mut u32,
            (peri & !(0x7 << 17)) | (1 << 16),
        );
    }
}

fn wait_for_m0() {
    unsafe {
        while read_volatile(IPC_SYNC_ADDR1 as *const u32) != IPC_SYNC_FLAG
            || read_volatile(IPC_SYNC_ADDR2 as *const u32) != IPC_SYNC_FLAG
        {
            dcache_invalidate(IPC_SYNC_ADDR1);
            core::hint::spin_loop();
        }
        write_volatile(IPC_SYNC_ADDR1 as *mut u32, 0);
        write_volatile(IPC_SYNC_ADDR2 as *mut u32, 0);
    }
}

fn enable_icache() {
    unsafe {
        core::arch::asm!("fence", options(nostack));
        core::arch::asm!("fence.i", options(nostack));
        core::arch::asm!(".word 0x0100000b", options(nostack));
        let mut mhcr: usize;
        core::arch::asm!("csrr {mhcr}, 0x7C1", mhcr = out(reg) mhcr, options(nostack));
        mhcr |= 1;
        core::arch::asm!("csrw 0x7C1, {mhcr}", mhcr = in(reg) mhcr, options(nostack));
        core::arch::asm!("fence", options(nostack));
        core::arch::asm!("fence.i", options(nostack));
    }
}

fn dcache_invalidate(addr: usize) {
    unsafe {
        core::arch::asm!(
            ".insn i 0x0b, 0x0, x0, {addr}, 0x2a",
            addr = in(reg) addr,
            options(nostack)
        );
        core::arch::asm!("fence", options(nostack));
    }
}
```

- [ ] **Step 5: Build the stub**

Run:

```bash
cargo build -p stage0 --release --target riscv64imac-unknown-none-elf
```

Expected: `Finished release` and `target/riscv64imac-unknown-none-elf/release/stage0` exists. Do not treat warnings from `bouffalo-hal` as failure.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock stage0/Cargo.toml stage0/build.rs stage0/src/main.rs
git commit -m "$(cat <<'EOF'
Add a stage0 crate that reaches the verified D0 M-mode path.

EOF
)"
```

---

### Task 2: Enter S-mode and print via UART3 FIFO

**Files:**
- Modify: `stage0/src/main.rs`

- [ ] **Step 1: Confirm the stub has no `mret`**

Run:

```bash
llvm-objdump -d target/riscv64imac-unknown-none-elf/release/stage0 | grep -E 'mret' || true
```

Expected: no `mret` (or empty). This is the “fail” before the feature exists.

- [ ] **Step 2: Replace `main`’s hang with PMP + `mret`, add S-mode FIFO print**

After the `icache on` flush, add:

```rust
    writeln!(serial, "[M] stage0 entering S-mode").ok();
    serial.flush().ok();
    pmp_allow_all();
    enter_s_mode(s_main);
```

Add these functions to the same file (do not use HAL in them):

```rust
const UART3_BASE: usize = 0x3000_2000;
const UART_FIFO_CONFIG_1: usize = UART3_BASE + 0x84;
const UART_FIFO_WDATA: usize = UART3_BASE + 0x88;

fn pmp_allow_all() {
    unsafe {
        let addr: usize = usize::MAX;
        let cfg: usize = 0x1F;
        core::arch::asm!(
            "csrw pmpaddr0, {addr}",
            "csrw pmpcfg0, {cfg}",
            addr = in(reg) addr,
            cfg = in(reg) cfg,
            options(nostack)
        );
    }
}

fn enter_s_mode(dest: extern "C" fn() -> !) -> ! {
    let dest = dest as usize;
    unsafe {
        core::arch::asm!(
            "csrw satp, zero",
            "csrr {mstatus}, mstatus",
            "li {tmp}, {mpp_mask}",
            "and {mstatus}, {mstatus}, {tmp}",
            "li {tmp}, {mpp_s}",
            "or {mstatus}, {mstatus}, {tmp}",
            "csrw mstatus, {mstatus}",
            "csrw mepc, {dest}",
            "mret",
            dest = in(reg) dest,
            mstatus = out(reg) _,
            tmp = out(reg) _,
            mpp_mask = const !(0b11 << 11),
            mpp_s = const 0b01 << 11,
            options(noreturn, nostack)
        );
    }
}

fn uart3_putb(b: u8) {
    unsafe {
        while (read_volatile(UART_FIFO_CONFIG_1 as *const u32) & 0x3F) >= 32 {
            core::hint::spin_loop();
        }
        write_volatile(UART_FIFO_WDATA as *mut u32, b as u32);
    }
}

fn uart3_puts(s: &str) {
    for b in s.as_bytes() {
        uart3_putb(*b);
    }
}

#[unsafe(no_mangle)]
extern "C" fn s_main() -> ! {
    uart3_puts("[S] hello from supervisor on C906\r\n");
    loop {
        core::hint::spin_loop();
    }
}
```

`enter_s_mode` must be `-> !`. Do not call `uart3_puts` from M-mode.

- [ ] **Step 3: Rebuild and check `mret` / `s_main`**

```bash
cargo build -p stage0 --release --target riscv64imac-unknown-none-elf
llvm-nm target/riscv64imac-unknown-none-elf/release/stage0 | grep s_main
llvm-objdump -d target/riscv64imac-unknown-none-elf/release/stage0 | grep -E '[[:space:]]mret'
```

Expected: `s_main` in `nm` output; at least one `mret` in objdump.

- [ ] **Step 4: Commit**

```bash
git add stage0/src/main.rs
git commit -m "$(cat <<'EOF'
Enter S-mode on D0 and print through the UART3 FIFO.

EOF
)"
```

---

### Task 3: Package `stage0.bin` like the helloworld images

**Files:**
- Modify: `scripts/build.sh`
- Create: `scripts/check-stage0.sh`

- [ ] **Step 1: Extend `scripts/build.sh`**

After the existing M0/D0 build/objcopy/truncate/patch/verify block, also build and package `stage0`. Keep producing helloworld-d0.bin. Change the flash hint to m0+stage0.

Add near the top:

```bash
STAGE0_ELF="$ROOT/target/$D0_TARGET/release/stage0"
STAGE0_BIN="$STAGE0_ELF.bin"
```

After the helloworld-d0 `cargo build` line, add:

```bash
cargo build -p stage0 --release --target "$D0_TARGET"
```

After d0 objcopy:

```bash
rust-objcopy --binary-architecture=riscv64 --strip-all -O binary "$STAGE0_ELF" "$STAGE0_BIN"
```

Truncate, `blri patch`, and `verify_hash` `STAGE0_BIN` the same way as the others.

End of script:

```bash
echo "M0 BIN:     $M0_BIN"
echo "D0 BIN:     $D0_BIN"
echo "STAGE0 BIN: $STAGE0_BIN"
ls -l "$M0_BIN" "$D0_BIN" "$STAGE0_BIN"
echo
echo "烧录 stage0（BOOT+RST，先松 RST，再松 BOOT）："
echo "  flash_m1s.sh -y --m0 $M0_BIN --d0 $STAGE0_BIN"
```

- [ ] **Step 2: Write `scripts/check-stage0.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ELF="$ROOT/target/riscv64imac-unknown-none-elf/release/stage0"
BIN="$ELF.bin"
[[ -f "$ELF" ]] || { echo "missing $ELF" >&2; exit 1; }
[[ -f "$BIN" ]] || { echo "missing $BIN" >&2; exit 1; }
head -c 4 "$BIN" | grep -q BFNP || { echo "BIN missing BFNP header" >&2; exit 1; }
llvm-nm "$ELF" | grep -q ' s_main$' || { echo "ELF missing s_main" >&2; exit 1; }
llvm-objdump -d "$ELF" | grep -qE '[[:space:]]mret' || { echo "ELF missing mret" >&2; exit 1; }
echo "check-stage0 ok"
```

`chmod +x scripts/check-stage0.sh`

- [ ] **Step 3: Run the full pack + check**

```bash
./scripts/build.sh
./scripts/check-stage0.sh
```

Expected: three hash-ok lines; `check-stage0 ok`; `stage0.bin` starts with `BFNP`.

- [ ] **Step 4: Commit**

```bash
git add scripts/build.sh scripts/check-stage0.sh
git commit -m "$(cat <<'EOF'
Package stage0.bin with the same BootROM hash pipeline.

EOF
)"
```

---

### Task 4: Update docs to match the new crate

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture.md`
- Modify: `docs/chapter-status.md`
- Modify: `docs/memory-map.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: README — compile/flash/expected output**

In the compile list add:

- `target/riscv64imac-unknown-none-elf/release/stage0.bin` → Flash `0x100000`（S 态路径）

Flash example:

```bash
flash_m1s.sh -y \
  --m0 target/riscv32imac-unknown-none-elf/release/rust-helloworld-m0.bin \
  --d0 target/riscv64imac-unknown-none-elf/release/stage0.bin
```

Expected serial block from the spec (uart up … `[S] hello from supervisor on C906`). Keep a one-liner that helloworld-d0 is still the 5s dual-core对照.

- [ ] **Step 2: architecture.md**

In the directory tree add `stage0/`. After the D0 helloworld section, add a “stage0” section: same M-state prefix, then PMP `pmpaddr0=-1` / `pmpcfg0=0x1F`, `satp=0`, `mret` to `s_main`, FIFO `0x30002000+0x88`. Update the “现在/以后” diagram to:

```text
现在：  BootROM → helloworld-m0 → stage0（M 态初始化，mret 进 S 态打印）
对照：  BootROM → helloworld-m0 → helloworld-d0（两核都在 M 态循环）
以后：  BootROM → M0 拉核 → D0 M 态（SBI）→ S 态 rCore
```

- [ ] **Step 3: chapter-status.md**

Change chapter 1 cell to: **stage0 已做到 `mret` + UART3 FIFO 打印 `[S]`**（实现后；若尚未上板验证，写“代码已合入，待上板”）。

- [ ] **Step 4: memory-map.md**

Flash table: add a row that `0x100000` is `stage0.bin` on the S-mode path, and `rust-helloworld-d0.bin` only when flashing the对照.

- [ ] **Step 5: AGENTS.md**

Add under 写固件时:

- S 态不要调用 HAL UART；只写 `0x30002000+0x88`。
- 这一期 PMP 全开（`pmpaddr0=-1`，`pmpcfg0=0x1F`）。不要在没验证 `[S]` 之前改回细粒度。
- 不要改 helloworld 两个 crate 来“顺便”做 stage0。

- [ ] **Step 6: Commit**

```bash
git add README.md docs/architecture.md docs/chapter-status.md docs/memory-map.md AGENTS.md
git commit -m "$(cat <<'EOF'
Document stage0 as the S-mode bring-up path.

EOF
)"
```

---

### Task 5: Flash and read ACM0

**Files:** none in git unless the run fails and you fix code.

- [ ] **Step 1: Start ACM0 listener (2000000 8N1, 20s) then flash**

Do **not** open ACM1 as a console. Flash:

```bash
flash_m1s.sh -y \
  --m0 target/riscv32imac-unknown-none-elf/release/rust-helloworld-m0.bin \
  --d0 target/riscv64imac-unknown-none-elf/release/stage0.bin
```

Board must be in download mode (BOOT+RST, release RST first).

- [ ] **Step 2: Compare the log to the spec**

Must contain, in order:

```text
rust helloworld from D0/C906, uart up
rust helloworld from D0/C906, ipc synced
rust helloworld from D0/C906, icache on
[M] stage0 entering S-mode
[S] hello from supervisor on C906
```

If `[M]` exists and `[S]` does not, debug PMP / `mret` / FIFO (spec failure table). Do not “fix” by printing `[S]` from M-mode.

- [ ] **Step 3: After a passing board run, set chapter-status ch1 to 板上已打出 `[S]` if it still said 待上板, and commit only if docs changed**

```bash
git add docs/chapter-status.md
git commit -m "$(cat <<'EOF'
Record that stage0 printed from S-mode on the M1s Dock.

EOF
)"
```

Skip this commit if the file already says the board result.

---

## Spec coverage

| Spec item | Task |
|-----------|------|
| New `stage0` crate, helloworld unchanged | 1 |
| M-mode UART/IPC/I-cache same as helloworld-d0 | 1 |
| Exact M-mode log lines + `[M]` | 1–2 |
| PMP `pmpaddr0=-1`, `pmpcfg0=0x1F` | 2 |
| `satp=0`, MPP=S, `mret` to `s_main` | 2 |
| S-mode FIFO `0x30002000+0x88`, exact `[S]` string | 2 |
| No HAL in S-mode, no 5s LED loop | 2 |
| `stage0.bin` truncate + blri + hash | 3 |
| build.sh flash hint m0+stage0 | 3 |
| Docs / AGENTS | 4 |
| ACM0 hardware order | 5 |
| No rCore submodule, no SBI, no satp paging | all (not added) |
