# 内存映射规格

本文档是内存布局与 MMIO 地址的**权威来源**。修改硬件地址或 `linker.ld` 时必须同步更新。

## 1. 系统内存映射（QEMU virt）

| 地址范围 | 设备 | 大小 |
|---|---|---|
| `0x0000_0000 - 0x0000_0FFF` | Debug ROM | 4 KiB |
| `0x0001_0000 - 0x0001_7FFF` | Boot ROM | 32 KiB |
| `0x0200_0000 - 0x0200_FFFF` | CLINT（核心本地中断） | 64 KiB |
| `0x0C00_0000 - 0x0FFF_FFFF` | PLIC（平台级中断） | 64 MiB |
| `0x1000_0000 - 0x1000_00FF` | UART0（16550A） | 256 B |
| `0x1000_1000 - 0x1000_8FFF` | VirtIO MMIO | 32 KiB |
| `0x2000_0000 - 0x3FFF_FFFF` | PCIe ECAM | 512 MiB |
| `0x3000_0000 - 0x3FFF_FFFF` | PCIe MMIO | 256 MiB |
| `0x8000_0000 - 0x87FF_FFFF` | DRAM（内核加载处） | 128 MiB |

> 注意：`0x0100_0000` 为 VIRT_TEST 设备（`reboot` / `poweroff` 用）。

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

## 3. 外设基址明细

### 3.1 UART0（16550A）

- 基址：`0x1000_0000`
- IRQ：`10`
- 寄存器访问必须使用 volatile 读写

### 3.2 CLINT

- 基址：`0x0200_0000`
- 关键偏移：
  - `0x0000`：MSIP（软件中断挂起，Hart 0）
  - `0x4000`：MTIMECMP（定时器比较值，Hart 0，64 位，低 32 位在前）
  - `0xBFF8`：MTIME（当前时间，64 位）
- MTIME 频率：QEMU 默认 10 MHz

### 3.3 PLIC

- 基址：`0x0C00_0000`
- 关键偏移：
  - `0x0004`：IRQ 1 优先级（IRQ 0 保留；优先级 0 = 禁用，1–7 有效）
  - `0x1000`：Pending 位图
  - `0x2000`：Enable（Context 0 = Hart 0 M-mode）
  - `0x200000`：Priority Threshold（Context 0）
  - `0x200004`：Claim / Complete（Context 0）
- Context 编号：Hart N M-mode = `2N`，S-mode = `2N+1`

### 3.4 QEMU virt 外部中断映射

| IRQ | 设备 |
|---|---|
| 0 | 保留 |
| 1–8 | VirtIO（块、网络、控制台、RNG 等） |
| 9 | PCIe（如启用） |
| 10 | **UART0** |
| 11+ | 扩展设备 |

## 4. 链接符号（由 `linker.ld` 定义，供启动代码使用）

- `_sbss` / `_ebss`：BSS 段边界
- `_sdata` / `_edata` / `_sidata`：数据段边界
- `_heap_start` / `_heap_end`：堆区间
- `_stack_top`：栈顶

## 5. 更新流程

1. 修改对应板的 `linker.ld` 或驱动基址常量
2. 同步更新本文档
3. 若涉及 H(A)L trait 返回的 `MemoryLayout`（`crates/hal/src/memory.rs`），检查板级实现