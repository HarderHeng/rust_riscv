# rcore-bl808

把 [rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3) 接到 **Sipeed M1s Dock / BL808 的 D0（C906）**。

本仓库只做板级，不 fork 内核。E907 没有 MMU，操作系统只跑在 C906 上。M0 负责拉核。

当前路径：sbi0（C906 上 M 态 SBI + S 态 `ecall`）。stage0 仍是 S 态直接写 FIFO 的对照。rCore 还没开始。

改代码前先看 [AGENTS.md](AGENTS.md) 和 [docs/architecture.md](docs/architecture.md)。

## 硬件

| | |
|--|--|
| 板子 | Sipeed M1s Dock |
| M0 / E907 | UART0 GPIO14/15，拉起 D0 |
| D0 / C906 | UART3 GPIO16/17，以后跑 OS |
| LED | GPIO8 |
| 控制台 | `/dev/ttyACM0`，`2000000 8N1` |
| 烧录口 | `/dev/ttyACM1`（不要当控制台） |

## 编译

`rustc` ≥ 1.85，targets `riscv32imac-unknown-none-elf`、`riscv64imac-unknown-none-elf`，`llvm-tools-preview`。HAL 和 `blri` 走 Cargo git 依赖，不用在旁边再 clone。

```bash
./scripts/build.sh
```

- `target/riscv32imac-unknown-none-elf/release/rust-helloworld-m0.bin` → Flash `0x0`
- `target/riscv64imac-unknown-none-elf/release/sbi0.bin` → Flash `0x100000`（S 态路径）
- `target/riscv64imac-unknown-none-elf/release/stage0.bin` → Flash `0x100000`（S 态直接写 FIFO 对照）
- `target/riscv64imac-unknown-none-elf/release/rust-helloworld-d0.bin` → Flash `0x100000`（双核 M 态对照）

## 烧录

UART Type-C 上电后：BOOT+RST，先松 RST，再松 BOOT。

工作区里的烧录脚本（本仓库不绑定它的路径）：

```bash
flash_m1s.sh -y \
  --m0 target/riscv32imac-unknown-none-elf/release/rust-helloworld-m0.bin \
  --d0 target/riscv64imac-unknown-none-elf/release/sbi0.bin
```

期望 ACM0 输出（第 4 章 Sv39 + PSRAM 待上板）：

```text
rust helloworld from D0/C906, uart up
rust helloworld from D0/C906, ipc synced
rust helloworld from D0/C906, icache on
[M] psram ok
[M] sbi0 entering S-mode
[S] hello via sbi
[S] satp on
[S] psram ok
[M] shutdown
```

`stage0.bin` 是 S 态直接写 FIFO 的对照；`rust-helloworld-d0.bin` 是两核都在 M 态、大约每 5 秒翻一次 LED 的对照。

```bash
picocom -b 2000000 /dev/ttyACM0
```

## 文档

- [AGENTS.md](AGENTS.md) — 给 Agent 的规则
- [docs/architecture.md](docs/architecture.md) — 本仓库代码架构
- [docs/memory-map.md](docs/memory-map.md) — 地址
- [docs/chapter-status.md](docs/chapter-status.md) — 对照 Tutorial

## 许可

MIT。接入 rCore-Tutorial 时按那边的许可证。
