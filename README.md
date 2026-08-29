# rcore-bl808

把 [rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3) 接到 **Sipeed M1s Dock / BL808 的 D0（C906）**。

本仓库只做板级，不 fork 内核。E907 没有 MMU，操作系统只跑在 C906 上。M0 负责拉核。

当前第一步：双核 HAL helloworld（M1s 上已验证）。S 态、SBI、rCore 还没开始。

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
- `target/riscv64imac-unknown-none-elf/release/rust-helloworld-d0.bin` → Flash `0x100000`

## 烧录

UART Type-C 上电后：BOOT+RST，先松 RST，再松 BOOT。

```bash
../flash_m1s.sh -y \
  --m0 target/riscv32imac-unknown-none-elf/release/rust-helloworld-m0.bin \
  --d0 target/riscv64imac-unknown-none-elf/release/rust-helloworld-d0.bin
```

```text
rust helloworld from D0/C906, uart up
rust helloworld from D0/C906, ipc synced
rust helloworld from D0/C906, icache on
hello world from D0/C906, count=1, gpio8 led=on
```

之后大约每 5 秒一行。

```bash
picocom -b 2000000 /dev/ttyACM0
```

## 文档

- [给 Agent 的规则](AGENTS.md)
- [代码架构](docs/architecture.md)
- [启动链](docs/boot-chain.md)
- [内存图](docs/memory-map.md)
- [对照 Tutorial](docs/chapter-status.md)

## 许可

MIT。接入 rCore-Tutorial 时按那边的许可证。
