# rcore-bl808

把 [rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3) 接到 **Sipeed M1s Dock / BL808 的 D0（T-Head C906）** 上。

这不是又一份 rCore 内核 fork。内核跟着上游走；本仓库只做板级：

- M0/E907 负责拉起 C906
- C906 上的 SBI / 内存布局 / UART3
- 对照 Tutorial 章节的移植状态

E907 没有 MMU，**不能**跑 rCore。操作系统只跑在 C906 上。

## 现在做到哪

| 阶段 | 状态 |
|------|------|
| 0 双核 HAL helloworld（M0 拉起 D0，两核打印，GPIO8，5 秒） | **M1s Dock 上已验证** |
| D0 M 态进 S 态并打印 | 未开始 |
| OpenSBI / RustSBI | 未开始 |
| rCore-Tutorial ch1–ch3 接到这块板 | 未开始 |
| Sv39（ch4）+ 用户态 shell | 未开始 |

第一步只证明：BootROM 能跑我们的镜像，M0 能拉起 D0，UART 和 IPC 可用。这是后面进 S 态、接 SBI 的前提。

## 硬件

- 板子：Sipeed M1s Dock（BL808）
- OS 核：D0 / C906，RV64 + Sv39
- 加载核：M0 / E907，负责 TZC、XIP offset、解复位、IPC
- M0 控制台：UART0，GPIO14 TX / GPIO15 RX，`2000000 8N1`
- D0 控制台：UART3，GPIO16 TX / GPIO17 RX，`2000000 8N1`
- 板载 LED：GPIO8
- 可用内存（规划）：PSRAM `0x50000000`，64MB

## 编译

需要 `rustc` ≥ 1.85、`riscv32imac-unknown-none-elf`、`riscv64imac-unknown-none-elf`、`llvm-tools-preview`。旁边要有 [bouffalo-hal](https://github.com/rustsbi/bouffalo-hal)（默认 `../bouffalo-hal`）。

```bash
./scripts/build.sh
```

产物：

- `target/riscv32imac-unknown-none-elf/release/rust-helloworld-m0.bin` → Flash `0x0`
- `target/riscv64imac-unknown-none-elf/release/rust-helloworld-d0.bin` → Flash `0x100000`

`build.sh` 会先把 `objcopy` 多出来的尾巴截到 `img_len`，再用 [blri](https://github.com/rustsbi/bouffalo-hal) 补 BootROM 要的 header SHA-256。不截的话 BootROM 会因 hash 拒跑。

## 烧录

板子已通过 UART Type-C 上电后：同时按住 BOOT + RST，先松 RST，再松 BOOT。

```bash
../flash_m1s.sh -y \
  --m0 target/riscv32imac-unknown-none-elf/release/rust-helloworld-m0.bin \
  --d0 target/riscv64imac-unknown-none-elf/release/rust-helloworld-d0.bin
```

脚本默认会让 BootROM 复位 CPU。在 D0 对应串口（这块板在 WSL 上是 `/dev/ttyACM0`）用 `2000000 8N1` 应看到：

```text
rust helloworld from D0/C906, uart up
rust helloworld from D0/C906, ipc synced
rust helloworld from D0/C906, icache on
hello world from D0/C906, count=1, gpio8 led=on
```

之后大约每 5 秒一行。`ipc synced` 说明 M0 已经跑起来并写了握手。`/dev/ttyACM1` 是烧录口，不要当控制台开。

```bash
picocom -b 2000000 /dev/ttyACM0
```

## 文档

- [架构](docs/architecture.md)
- [启动链](docs/boot-chain.md)
- [内存图](docs/memory-map.md)
- [对照 Tutorial 的章节状态](docs/chapter-status.md)
- [贡献](CONTRIBUTING.md)

## 和现有项目的关系

| 项目 | 关系 |
|------|------|
| [rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3) | 上游内核，本仓库不复制 |
| [rustsbi/bouffalo-hal](https://github.com/rustsbi/bouffalo-hal) | helloworld 的 M 态启动、GPIO、UART |
| 板载 Linux + OpenSBI | 内存图和 PMP 的参照，不是依赖 |
| C SDK `examples/helloworld` | 双核拉起、IPC、UART 时钟的对照 |

## 许可

MIT。rCore-Tutorial 本身有自己的许可证；接入上游代码时按那边的条款来。
