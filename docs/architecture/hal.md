# HAL 分层设计

本文档解读 `crates/hal` 的硬件抽象层设计：为什么拆分层、trait 如何协作、板级如何接入。

## 1. 设计动机

裸机内核要移植到多个平台（QEMU 虚拟机、BL808 真实芯片的多个核），如果内核代码直接操作 MMIO 寄存器，每加一块板子就要改内核。HAL 通过 trait 定义硬件能力的**契约**，让内核只依赖抽象接口，具体寄存器操作下沉到板级实现。

依赖方向（单向，禁止反向）：

```text
boards/<board>（bin：装配、PLATFORM 实例、panic handler）
      │
      ├──► kernel（平台无关内核：Shell、中断分发）
      │         │
      │         └──► hal（trait 定义）
      │                   ▲
      └──► hal_impl（板级具体实现，实现 hal 的 trait）
```

- `kernel` 只知道 `hal` 的 trait，不知道任何寄存器地址。
- `boards` 既提供 `kernel_main` 的调用环境，也提供 `hal` 的具体实现。
- `riscv-common` 提供架构级启动代码（`_start`、BSS 清零、CSR 读写），被板级复用。

## 2. 核心 trait

### 2.1 `Platform`（`crates/hal/src/platform.rs`）

平台的总入口，用关联类型把串口和中断控制器绑定到具体类型：

```rust
pub trait Platform: Send + Sync {
    type Serial: SerialPort;
    type Interrupt: InterruptController;

    fn console(&self) -> &Self::Serial;          // 主串口（内核输出/调试）
    fn interrupt_controller(&self) -> &Self::Interrupt;
    fn console_irq(&self) -> u32;                // 串口中断源编号（QEMU 为 10）
    fn name(&self) -> &'static str;
    fn arch(&self) -> &'static str;
    fn reboot(&self) -> !;
    fn memory_layout(&self) -> MemoryLayout;
}
```

要点：
- `Send + Sync` 允许静态实例（`static PLATFORM`）在中断上下文安全共享。
- 方法全部 `&self`，配合 `Send + Sync` 支持中断驱动的并发访问。
- `MemoryLayout`（`crates/hal/src/memory.rs`）描述堆 / 栈 / text / data / bss 各区间，通常从链接脚本符号填充。

### 2.2 `SerialPort`（`crates/hal/src/serial.rs`）

串口抽象：`init`、阻塞写 `putc`、非阻塞读 `try_getc`、字符串写 `puts`（默认逐字节）、RX 中断开关 `enable/disable_rx_interrupt`。

### 2.3 `InterruptController`（`crates/hal/src/interrupt.rs`）

面向 PLIC 这类优先级中断控制器：设置优先级、使能 / 禁用 IRQ、设置阈值、`claim`（原子取回挂起源）与 `complete`（处理完毕，必须成对调用）。

## 3. 板级接入方式（以 QEMU virt 为例）

`crates/boards/qemu-virt-rv32`：

- `platform.rs`：定义 `QemuVirtPlatform`，实现 `Platform`（`console()` 返回 16550A 驱动、`interrupt_controller()` 返回 PLIC 驱动）。
- `hal_impl/uart_16550a.rs`、`hal_impl/plic.rs`：实现 `SerialPort`、`InterruptController`，**所有 MMIO 访问用 volatile 读写**，`unsafe` 只出现在寄存器边界。
- `main.rs`：`static PLATFORM: QemuVirtPlatform` → 注册 PLIC claim/complete 到内核 trap 系统 → 调 `kernel::kernel_main(&PLATFORM, commands, prompt)`。
- `startup.rs`：复用 `riscv-common` 的启动代码。

## 4. 关键约定

1. **安全边界**：trait 的公开方法都是 safe Rust；`unsafe` 封装在具体实现的寄存器访问处。
2. **不可破坏分层**：内核不得直接访问板级 MMIO；新硬件驱动写在 `hal_impl/` 下实现 trait，而不是裸寄存器操作。
3. **`static` 兼容**：所有 trait `Send + Sync`，实现通常是 `static` 实例。
4. **文档要求**：`crates/hal`、`crates/kernel` 均开启 `#![deny(missing_docs)]`，公开项必须写文档注释。

## 5. 扩展新平台

1. 新建 `crates/boards/<name>`（bin crate），依赖 `kernel`、`hal`、`riscv-common`。
2. 实现 `Platform`，在 `hal_impl/` 写串口 / 中断控制器实现。
3. 提供启动代码（可复用 `riscv-common`）与 `linker.ld`。
4. 在 `main.rs` 装配：`PLATFORM` 实例 → `kernel_main`。
5. 同步更新 [memory-map.md](memory-map.md) 与本文档。