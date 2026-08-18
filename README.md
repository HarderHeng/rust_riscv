# RISC-V Bare-Metal Rust Kernel

用 Rust 从零编写、无操作系统依赖的 RISC-V 32 位裸机内核，目前运行在 QEMU 模拟的 `virt` 虚拟机上。适合学习底层系统编程、RISC-V 架构和嵌入式 Rust 的实践项目。

`#![no_std]` + `#![no_main]`，不依赖任何 C 运行时——从引导汇编、串口驱动到中断处理和交互式 Shell，全部自己实现。

## 技术栈概览

| 项目 | 说明 |
|---|---|
| 语言 | Rust（`riscv32imac-unknown-none-elf` 目标，`no_std` / `no_main`） |
| 架构 | RISC-V 32 位（RV32IMAC：整数 / 乘除 / 原子 / 压缩指令） |
| 运行环境 | QEMU `virt` 虚拟机（`qemu-system-riscv32`） |
| 硬件外设 | 16550A UART 串口、PLIC / CLINT 中断控制器 |
| 内存布局 | 自定义 `linker.ld` 链接脚本（代码 / 数据 / BSS / 64 KiB 栈） |

## 它做了什么

启动引导（汇编 `_start` 设置栈、清零 BSS）→ UART 串口驱动输出 → 机器模式中断系统（UART 接收、PLIC 外部中断、可注册回调）→ 进入一个可交互的 Shell（`help` / `echo` / `meminfo` / `reboot` 等命令）。

## 快速开始

前置依赖：

- Rust 工具链（`rustup`）与 RISC-V 目标：`rustup target add riscv32imac-unknown-none-elf`
- QEMU RISC-V 模拟器：`qemu-system-riscv32`

构建并运行：

```bash
cargo build              # 编译
cargo run                # 在 QEMU 中运行
cargo run --release      # 优化构建后运行
```

预期输出：

```
=================================
  Bare-Metal RISC-V Kernel
=================================
[KERNEL] Trap handler initialized
[KERNEL] Console RX interrupt enabled
[KERNEL] Interrupts enabled

riscv32> help
```

QEMU 中按 `Ctrl-A` 然后 `X` 退出。

## 项目结构

```text
├── crates/
│   ├── hal/               # 硬件抽象层：Platform / SerialPort / InterruptController trait
│   ├── riscv-common/      # 共享启动代码 + CSR 工具（自动区分 rv32 / rv64）
│   ├── kernel/            # 平台无关内核：Shell + 中断分发
│   └── boards/
│       ├── qemu-virt-rv32/   # ✅ QEMU 虚拟机板级支持（完整可用）
│       ├── bl808-e902/       # ⚠️ 真实芯片占位（RV32EMC 低功耗核）
│       ├── bl808-e907/       # ⚠️ 真实芯片占位（RV32IMACF 应用核）
│       └── bl808-c906/       # ⚠️ 真实芯片占位（RV64IMAC，目标 Linux）
├── docs/                  # 项目文档（规格 / 计划 / 架构，见下方导航）
├── linker.ld              # 链接脚本：内存布局
└── Cargo.toml             # workspace 配置
```

内核代码不直接碰硬件，而是面向 HAL 的 `Platform` trait 编程；每块板子提供自己的实现。QEMU 板是当前完整可用的参考实现，三个 BL808 真实芯片板为占位骨架（等待硬件数据手册补齐内存映射与寄存器规格）。

## 文档导航

| 文档 | 读者 | 内容 |
|---|---|---|
| [README.md](README.md) | 所有访客 | 这是啥、怎么跑（本文档） |
| [AGENTS.md](AGENTS.md) | AI Agent + 开发者 | 行为约束、代码分层、常用命令、文档索引 |
| [docs/spec/](docs/spec/) | 实现者 | 资源与接口规格：内存映射、Shell 命令与扩展指南 |
| [docs/plan/](docs/plan/) | 维护者 | 计划与过程：评审报告、已修复问题、路线图 |
| [docs/architecture/](docs/architecture/) | 架构师 / 深度参与者 | 设计原理与机制细节：HAL 分层、Shell 架构、中断模型 |

完整索引见 [docs/README.md](docs/README.md)。

## 调试

```bash
cargo run -- gdb                    # QEMU 挂起等待 GDB（端口 1234）
riscv32-unknown-elf-gdb target/riscv32imac-unknown-none-elf/debug/qemu-virt-rv32
# (gdb) target remote :1234
```

查看生成信息：

```bash
less kernel.map                     # 内存映射
rust-objdump -d <elf>               # 反汇编
rust-size <elf>                     # 各段大小
```

## 常见问题

- **内核没输出**：确认 UART 已在 `kernel_main` 开头初始化（`PLATFORM.console().init()`）。
- **QEMU 立即退出**：`kernel_main` 不能返回，必须以死循环（Shell 的 `run()` 或 `wfi`）结尾。
- **链接错误 / undefined symbol**：确认根目录 `linker.ld` 存在，且 `build.rs` 声明了对它的依赖。

## 许可

MIT License，见 [LICENSE](LICENSE)。

## 参考资源

- [RISC-V 规范](https://riscv.org/technical/specifications/)
- [QEMU RISC-V 文档](https://www.qemu.org/docs/master/system/target-riscv.html)
- [Rust Embedded Book](https://rust-embedded.github.io/book/)
- [16550 UART 数据手册](http://caro.su/msx/ocm_de1/16550.pdf)
