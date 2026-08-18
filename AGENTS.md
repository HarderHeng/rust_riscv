# AGENTS.md

本文件为 AI Agent 与开发者（实际写代码的人）提供行为约束与精准导航。开始任何代码改动前，先读完本文档。

## 项目定位

RISC-V 32 位裸机内核（`no_std` / `no_main`），运行于 QEMU `virt` 虚拟机。当前是**多平台 HAL workspace 架构**：内核逻辑与硬件实现分离，QEMU 板完整可用，BL808 真实芯片板为占位骨架。

代码结构以 `crates/` 下的 workspace 为准；仓库根目录的 `src/` 与链接脚本是**已迁移的旧版遗留**（部分引用可能过时，注意甄别）。

## 红线（不可违反）

1. **不得引入 `std`**：所有 crate 必须 `#![no_std]`；用 `core::`，不用 `std::`。
2. **硬件寄存器访问必须 volatile**：一律 `read_volatile` / `write_volatile`，禁止编译器优化掉 MMIO 读写。
3. **`unsafe` 只在 MMIO 边界**：驱动公开 API 必须是 safe Rust，`unsafe` 封装在寄存器访问处。
4. **`kernel_main` 不得返回**：必须以死循环（Shell `run()` 或 `wfi`）结束，否则 QEMU 立即退出。
5. **Panic 行为为 abort**：dev/release 均 `panic = "abort"`，无 unwinding。
6. **不得破坏分层**：`kernel` 只依赖 HAL trait，禁止直接访问板级 MMIO；板级实现必须在 `boards/*/hal_impl/` 内。
7. **UART 输出统一 `\r\n`**：保持终端兼容。

## 架构分层

```text
boards/<board>           板级入口（bin）：PLATFORM 实例、panic handler、启动装配
    └─ hal_impl/         该板对 HAL trait 的具体实现（UART、PLIC、时钟）
kernel（crates/kernel）  平台无关内核：Shell、中断分发（trap 回调注册）
hal（crates/hal）        trait 定义：Platform / SerialPort / InterruptController
riscv-common             启动代码（_start、BSS 清零）+ CSR 工具（rv32/rv64 自动选择）
```

依赖方向：`kernel → hal ← boards`；`riscv-common` 被板级使用。**禁止反向依赖**（hal 不依赖 kernel，kernel 不依赖 boards）。

## 常用命令

```bash
cargo build --release                     # 构建（默认构建全部）
cargo build -p qemu-virt-rv32             # 只构建 QEMU 板
cargo run -p qemu-virt-rv32 --release     # 在 QEMU 中运行
cargo run -p qemu-virt-rv32 -- gdb        # 启动 QEMU 挂起，等 GDB 端口 1234
cargo fmt --check                         # 格式检查（改动后必须通过）
```

BL808 板当前为占位，构建可能因 `todo!()` 失败；QEMU 板是最新可用实现。

## 文档索引（按阅读顺序）

| 文档 | 何时读 |
|---|---|
| [README.md](README.md) | 首次接触项目（访客级认知） |
| [docs/architecture/memory-map.md](docs/architecture/memory-map.md) | 涉及地址、内存布局、外设位置 |
| [docs/architecture/shell-reference.md](docs/architecture/shell-reference.md) | 改 Shell 用法 / 行为规格 |
| [docs/architecture/shell-extension.md](docs/architecture/shell-extension.md) | 新增 Shell 命令 |
| [docs/architecture/hal.md](docs/architecture/hal.md) | 改 HAL trait 或新增板级实现 |
| [docs/architecture/shell.md](docs/architecture/shell.md) | 改 Shell 内部机制 / 设计 |
| [docs/architecture/interrupt-model.md](docs/architecture/interrupt-model.md) | 改中断 / trap / PLIC / CLINT |
| [docs/architecture/interrupt-comparison.md](docs/architecture/interrupt-comparison.md) | 对比 QEMU 与 BL808 中断差异 |
| [docs/architecture/fixes-applied.md](docs/architecture/fixes-applied.md) | 查已知修复记录，避免重复踩坑 |
| [docs/README.md](docs/README.md) | 文档全貌与工作流（spec / plan 约定） |

## 关键约定

- 新增硬件驱动：在对应的 `boards/*/hal_impl/` 下实现 HAL trait，不要直接写裸 MMIO。
- 新增 Shell 命令：实现 `CommandHandler`，注册到 `COMMANDS` 数组并**保持字母序**（二分查找依赖排序）。
- 新增内存段 / 修改布局：改对应板的 `linker.ld`，并同步 `docs/architecture/memory-map.md`。
- 所有 crate 开启 `#![deny(missing_docs)]`：公开项必须有文档注释。
- 本文件与文档结构变更后，同步更新 [docs/README.md](docs/README.md) 索引。