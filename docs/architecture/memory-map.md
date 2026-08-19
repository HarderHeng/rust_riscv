# 内存映射规格

本文档是内存布局与 MMIO 地址的**权威来源**。修改硬件地址或 `linker.ld` 时必须同步更新。

数据来源：QEMU `hw/riscv/virt.c` 的 `virt_memmap[]` 与 `include/hw/riscv/virt.h`（`-machine virt` 默认配置）。

## 1. 系统内存映射（QEMU virt，本板为 RV32）

| 地址范围 | 设备 | 大小 | 说明 |
|---|---|---|---|
| `0x0000_0000 - 0x0000_00FF` | Debug ROM | 256 B | 调试用 |
| `0x0000_1000 - 0x0000_FFFF` | MROM（掩膜 ROM / Boot ROM） | 60 KiB | 复位向量，`-bios none` 时由 QEMU 直接用 `-kernel` 启动 |
| `0x0010_0000 - 0x0010_0FFF` | **VIRT_TEST**（syscon） | 4 KiB | 写 `0x7777`=重启、`0x5555`=关机、`0x3333`=fail |
| `0x0010_1000 - 0x0010_1FFF` | RTC（goldfish-rtc） | 4 KiB | 实时时钟，IRQ 11 |
| `0x0200_0000 - 0x0200_FFFF` | CLINT（核心本地中断） | 64 KiB | 软件中断 + 定时器 |
| `0x02F0_0000 - 0x02F0_3FFF` | ACLINT SSWI | 16 KiB | 仅 `-machine virt,aclint=on` 时存在 |
| `0x0300_0000 - 0x0300_FFFF` | PCIe PIO | 64 KiB | PCI I/O 端口窗口 |
| `0x0301_0000 - 0x0301_0FFF` | RISC-V IOMMU | 4 KiB | 仅 `-machine virt,iommu-sys=on` |
| `0x0400_0000 - 0x05FF_FFFF` | Platform Bus | 32 MiB | 动态 sysbus 设备（如 TPM） |
| `0x0C00_0000 - 0x0C5F_FFFF` | **PLIC**（平台级中断） | 6 MiB | 大小 = `0x200000 + 2×CPUS_MAX×0x1000`（当前覆盖到 0x0C_5F_FFFF） |
| `0x0C00_0000`（M）/ `0x0D00_0000`（S） | APLIC | — | 仅 `-machine virt,aplic=on` / `aplic-imsic` 时取代 PLIC |
| `0x1000_0000 - 0x1000_00FF` | **UART0**（16550A） | 256 B | 串口，时钟 3.6864 MHz，IRQ 10 |
| `0x1000_1000 - 0x1000_8FFF` | VirtIO MMIO | 32 KiB | 8 个槽位，每个 0x1000，IRQ 1–8 |
| `0x1010_0000 - 0x1010_0017` | FW_CFG（firmware config） | 24 B | 初始固件硬件探针用 |
| `0x2000_0000 - 0x23FF_FFFF` | Flash（pflash0/1） | 64 MiB | 两块 32 MiB CFI flash |
| `0x2400_0000` / `0x2800_0000` | IMSIC（M/S） | — | 仅 `-machine virt,aplic-imsic` 时存在 |
| `0x3000_0000 - 0x3FFF_FFFF` | **PCIe ECAM** | 256 MiB | PCI 配置空间（总线 0–255） |
| `0x4000_0000 - 0x7FFF_FFFF` | **PCIe MMIO**（4G 以下） | 1 GiB | 32 位 BAR 窗口 |
| `0x3000_0000_00 - 0x3FFF_FFFF_FF` | PCIe MMIO high（RV32） | 4 GiB | 64 位 BAR 窗口，RV32 固定在此基址 |
| `0x8000_0000 - 0x87FF_FFFF` | **DRAM**（内核加载处） | 128 MiB | 默认内存，QEMU `-kernel` 加载到此 |

> **MMU 说明**：本内核运行在 M 模式且**未开启分页（MMU 关闭）**，所有地址均为物理地址，直接读写。
> 上表中的 PCIe ECAM / MMIO / High MMIO 是设备的内存窗口（供设备 BAR 空间访问），不是 CPU 页表。

## 2. 链接脚本布局（RAM 内，`linker.ld`）

内核从 `0x8000_0000` 开始，RAM 内各段：

| 段 | 说明 |
|---|---|
| `.text` | 代码段，起始即 `_start` 入口 |
| `.rodata` | 只读数据（字符串、常量） |
| `.data` | 已初始化读写数据 |
| `.bss` | 零初始化数据（启动时由 `_start` 清零） |
| `.stack` | 内核栈，64 KiB |
| heap | 栈之后到 RAM 末尾的剩余内存（当前未分配） |

## 3. 外设基址与关键寄存器

### 3.1 UART0（16550A，`ns16550a`）

- 基址：`0x1000_0000`，大小 0x100，IRQ `10`
- 时钟：**3.6864 MHz**（fdt `clock-frequency = 3686400`）
- 波特率除数：`divisor = clock / (16 × baud)`（如 38400 baud → divisor 6）
- 寄存器偏移（byte 寻址）：

| 偏移 | DLAB=0 读 | DLAB=0 写 | DLAB=1 写 |
|---|---|---|---|
| `0x00` | RBR（接收缓冲） | THR（发送保持） | DLL（除数低 8 位） |
| `0x01` | IER（读） | IER | DLM（除数高 8 位） |
| `0x02` | IIR | FCR | FCR |
| `0x03` | LCR | LCR | LCR |
| `0x04` | MCR | MCR | MCR |
| `0x05` | LSR | — | LSR |
| `0x06` | MSR | — | MSR |
| `0x07` | SCR | SCR | SCR |

- 关键位：`LSR[5]`=THRE（发送就绪，写前必须轮询），`LSR[0]`=DR（有数据可读），`LCR[7]`=DLAB，`IER[0]`=RX 数据可用中断。
- 寄存器访问必须使用 volatile 读写。

### 3.2 CLINT

- 基址：`0x0200_0000`，每 hart 一组，以下偏移均为 Hart 0：

| 偏移 | 寄存器 | 宽度 | 说明 |
|---|---|---|---|
| `0x0000` | MSIP[0] | 32 | 软件中断挂起（每 hart +4） |
| `0x4000` | MTIMECMP[0] | 64 | 定时器比较值（每 hart +8；先写高 32 位，再写低 32 位） |
| `0xBFF8` | MTIME | 64 | 当前时间（全局）；频率 10 MHz |

- 触发条件：`MTIME >= MTIMECMP` → `mip.MTIP=1`。
- `riscv,aclint-mswi/mtimer` 兼容，基址偏移与上表一致。

### 3.3 PLIC

- 基址：`0x0C00_0000`，优先级位宽 3（0–7），中断源共 **96** 个（IRQ 1–95 可用，IRQ 0 保留）
- 关键偏移：

| 偏移 | 寄存器 | 说明 |
|---|---|---|
| `0x0000 + 4×irq` | Priority[irq] | IRQ 优先级（0=禁用，1–7 有效；irq≥1） |
| `0x1000` | Pending | 挂起位图（bit N = IRQ N） |
| `0x2000 + 0x80×ctx` | Enable[ctx] | 使能位图（每 context 128 字节） |
| `0x200000 + 0x1000×ctx` | Threshold[ctx] | 优先级阈值（> 阈值才投递） |
| `0x200004 + 0x1000×ctx` | Claim / Complete[ctx] | 读=声明（返回 IRQ），写=完成 |

- Context 编号：Hart N M 模式 = `2N`，S 模式 = `2N+1`。
- 本驱动使用 Context 0（Hart 0 M 模式）：Enable 位于 `0x2000`，Claim/Complete 位于 `0x200004`。

### 3.4 RTC（goldfish-rtc）

- 基址：`0x0010_1000`，IRQ `11`
- 寄存器：`0x00` TIME_LOW、`0x04` TIME_HIGH、`0x08` ALARM_LOW、`0x0C` ALARM_HIGH、`0x10` IRQ_ENABLED、`0x18` ALARM_STATUS、`0x1C` SET_ALARM_LOW、`0x20` SET_ALARM_HIGH

### 3.5 VIRT_TEST（syscon）

- 基址：`0x0010_0000`
- 写入值：`0x7777`=reset（`platform::reboot` 使用）、`0x5555`=poweroff（pass）、`0x3333`=fail

## 4. QEMU virt 外部中断映射

| IRQ | 设备 |
|---|---|
| 0 | 保留（无效） |
| 1–8 | VirtIO（块、网络、控制台、RNG 等 8 个设备） |
| 10 | **UART0** |
| 11 | RTC |
| 32–35 | PCIe（4 个 INTx，`PCIE_IRQ = 0x20`） |
| 36–39 | RISC-V IOMMU（`iommu-sys=on`） |
| 64–95 | Platform Bus（共 32 个） |

## 5. 链接符号（由 `linker.ld` 定义，供启动代码使用）

- `_sbss` / `_ebss`：BSS 段边界
- `_sdata` / `_edata` / `_sidata`：数据段边界
- `_heap_start` / `_heap_end`：堆区间
- `_stack_top`：栈顶

## 6. 更新流程

1. 修改对应板的 `linker.ld` 或驱动基址常量
2. 同步更新本文档
3. 若涉及 HAL trait 返回的 `MemoryLayout`（`crates/hal/src/memory.rs`），检查板级实现（`platform::memory_layout`）

## 7. 参考

- QEMU 源码：`hw/riscv/virt.c`（`virt_memmap[]`）、`include/hw/riscv/virt.h`（IRQ 与 PLIC/CLINT 常量）
- RISC-V 特权架构规范 v1.11（CLINT/PLIC/CSR）
- SiFive CLINT / PLIC 规范
