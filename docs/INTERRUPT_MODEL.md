# QEMU virt RISC-V 32位中断模型详解

## 概述

QEMU virt机器模拟的RISC-V 32位系统使用标准的RISC-V中断架构，包含两个主要的中断控制器：

## 1. 中断控制器架构

```
┌─────────────────────────────────────────────────────────────┐
│                    RISC-V CPU Core                          │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐     │
│  │   mstatus    │  │     mie      │  │    mip      │     │
│  │  (中断使能)   │  │ (中断使能位) │  │ (中断挂起位) │     │
│  └──────────────┘  └──────────────┘  └─────────────┘     │
│           │                 │                 │            │
│           └─────────────────┼─────────────────┘            │
│                             │                              │
│                      ┌──────▼──────┐                       │
│                      │    mtvec    │                       │
│                      │ (陷阱向量表)  │                       │
│                      └─────────────┘                       │
└─────────────────────────────────────────────────────────────┘
                             │
                             │
              ┌──────────────┼──────────────┐
              │              │              │
    ┌─────────▼──────┐  ┌───▼────┐  ┌─────▼─────┐
    │     CLINT      │  │  PLIC  │  │  软件中断  │
    │  (核心本地中断) │  │(平台级) │  │           │
    └────────────────┘  └────────┘  └───────────┘
           │                 │
    ┌──────┴──────┐   ┌─────┴────────────────┐
    │             │   │                      │
┌───▼───┐   ┌────▼──┐ │  ┌────────┐  ┌──────▼──┐
│ Timer │   │ MSI   │ │  │ UART0  │  │ VirtIO  │
│  中断  │   │  中断 │ │  │  IRQ   │  │   IRQ   │
└───────┘   └───────┘ └─►│  #10   │  │   #1-8  │
                         └────────┘  └─────────┘
```

## 2. CLINT (Core Local Interruptor)

### 2.1 基本信息
- **MMIO基址**: `0x0200_0000`
- **大小**: 64 KB (0x10000)
- **功能**: 处理核心本地中断（定时器中断和软件中断）

### 2.2 寄存器布局

```
偏移地址              寄存器名                    说明
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
0x0000               MSIP (Hart 0)             软件中断挂起位
0x0004               MSIP (Hart 1)             (多核系统)
...
0x4000               MTIMECMP (Hart 0) [Low]   定时器比较值低32位
0x4004               MTIMECMP (Hart 0) [High]  定时器比较值高32位
0x4008               MTIMECMP (Hart 1) [Low]   (多核系统)
...
0xBFF8               MTIME [Low]               当前时间低32位
0xBFFC               MTIME [High]              当前时间高32位
```

### 2.3 中断类型

| 中断名称 | 异常代码 | mip位 | mie位 | 说明 |
|---------|---------|-------|-------|------|
| 机器软件中断 (MSI) | 3 | MSIP (bit 3) | MSIE (bit 3) | 核间通信 |
| 机器定时器中断 (MTI) | 7 | MTIP (bit 7) | MTIE (bit 7) | 定时器超时 |

### 2.4 工作原理

**定时器中断**：
```rust
// 当 MTIME >= MTIMECMP 时触发中断
if MTIME >= MTIMECMP {
    mip.MTIP = 1;  // 设置挂起位
    if mie.MTIE && mstatus.MIE {
        // 触发机器模式定时器中断
        pc = mtvec;
    }
}
```

**频率**: MTIME通常以10MHz增长（QEMU默认）

## 3. PLIC (Platform-Level Interrupt Controller)

### 3.1 基本信息
- **MMIO基址**: `0x0C00_0000`
- **大小**: 64 MB (0x0400_0000)
- **功能**: 管理外部设备中断（如UART、VirtIO）
- **支持**: 最多127个中断源（IRQ 1-127，IRQ 0保留）

### 3.2 寄存器布局

```
偏移地址              寄存器名                    说明
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
0x000000             Reserved                  保留
0x000004             Priority 1                IRQ 1 优先级 (1-7)
0x000008             Priority 2                IRQ 2 优先级
...
0x001000             Pending                   挂起位图 (bit N = IRQ N)
0x002000             Enable (Context 0)        中断使能位图 (Hart 0 M-mode)
0x002080             Enable (Context 1)        中断使能位图 (Hart 0 S-mode)
...
0x200000             Priority Threshold (Ctx0) 优先级阈值 (Hart 0 M-mode)
0x200004             Claim/Complete (Ctx0)     声明/完成寄存器
0x201000             Priority Threshold (Ctx1) (Hart 0 S-mode)
0x201004             Claim/Complete (Ctx1)
```

### 3.3 QEMU virt外部中断映射

| IRQ编号 | 设备 | 说明 |
|---------|------|------|
| 0 | 保留 | 无效中断源 |
| 1 | VirtIO Block | 块设备 |
| 2 | VirtIO Net | 网络设备 |
| 3 | VirtIO Console | 控制台 |
| 4 | VirtIO RNG | 随机数生成器 |
| 5-8 | VirtIO其他 | 其他VirtIO设备 |
| 9 | PCIe | PCIe中断 (如果启用) |
| 10 | **UART0** | **串口0（你的项目在用）** |
| 11+ | 扩展设备 | 其他外设 |

### 3.4 中断处理流程

```
1. 外设产生中断 (例如 UART0 数据到达)
   └─► PLIC.Pending[10] = 1

2. PLIC检查优先级和使能
   - Priority[10] > Threshold ?
   - Enable[10] == 1 ?
   └─► 如果满足，向CPU发送外部中断信号

3. CPU响应
   - mip.MEIP = 1 (机器外部中断挂起)
   - 如果 mie.MEIE && mstatus.MIE
   └─► 跳转到 mtvec (异常代码 = 11)

4. 软件处理中断
   - 读取 PLIC Claim寄存器获取IRQ编号
   - 处理中断 (例如读取UART数据)
   - 写入 PLIC Complete寄存器完成中断
   - mret 返回
```

## 4. CSR寄存器详解

### 4.1 mstatus (机器状态寄存器)

```
Bit     名称    说明
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
3       MIE     机器模式全局中断使能
          0: 禁用所有中断
          1: 使能中断（还需检查mie）
7       MPIE    中断前的MIE值（用于mret恢复）
```

### 4.2 mie (机器中断使能寄存器)

```
Bit     名称    说明
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
3       MSIE    机器软件中断使能
7       MTIE    机器定时器中断使能
11      MEIE    机器外部中断使能（PLIC）
```

### 4.3 mip (机器中断挂起寄存器)

```
Bit     名称    说明
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
3       MSIP    机器软件中断挂起（只读）
7       MTIP    机器定时器中断挂起（只读）
11      MEIP    机器外部中断挂起（只读）
```

### 4.4 mtvec (机器陷阱向量寄存器)

```
Bits    说明
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
[31:2]  BASE: 陷阱处理程序基址（4字节对齐）
[1:0]   MODE: 中断模式
          0 (Direct): 所有陷阱跳转到 BASE
          1 (Vectored): 中断跳转到 BASE + 4*cause
```

### 4.5 mcause (机器陷阱原因寄存器)

```
Bit     说明
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
31      Interrupt: 1=中断, 0=异常
[30:0]  Exception Code:

中断代码 (mcause[31]=1):
  3  = 机器软件中断 (MSI)
  7  = 机器定时器中断 (MTI)
  11 = 机器外部中断 (MEI, 来自PLIC)

异常代码 (mcause[31]=0):
  0  = 指令地址未对齐
  1  = 指令访问异常
  2  = 非法指令
  3  = 断点
  ...
```

## 5. 完整的内存映射

```
地址范围                      设备                大小
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
0x0000_0000 - 0x0000_0FFF    Debug ROM           4 KB
0x0001_0000 - 0x0001_7FFF    Boot ROM            32 KB
0x0200_0000 - 0x0200_FFFF    CLINT               64 KB
0x0C00_0000 - 0x0FFF_FFFF    PLIC                64 MB
0x1000_0000 - 0x1000_00FF    UART0 (16550A)      256 B
0x1000_1000 - 0x1000_8FFF    VirtIO MMIO         32 KB
0x2000_0000 - 0x3FFF_FFFF    PCIe ECAM           512 MB
0x3000_0000 - 0x3FFF_FFFF    PCIe MMIO           256 MB
0x8000_0000 - 0x87FF_FFFF    DRAM                128 MB (默认)
```

## 6. 示例代码

### 6.1 启用定时器中断

```rust
// CLINT基址
const CLINT_BASE: usize = 0x0200_0000;
const MTIMECMP_OFFSET: usize = 0x4000;
const MTIME_OFFSET: usize = 0xBFF8;

pub struct Clint {
    base: usize,
}

impl Clint {
    pub const fn new() -> Self {
        Self { base: CLINT_BASE }
    }

    // 读取当前时间
    pub fn read_mtime(&self) -> u64 {
        unsafe {
            let addr = (self.base + MTIME_OFFSET) as *const u64;
            core::ptr::read_volatile(addr)
        }
    }

    // 设置定时器比较值（触发中断）
    pub fn set_mtimecmp(&self, hart_id: usize, value: u64) {
        unsafe {
            let addr = (self.base + MTIMECMP_OFFSET + hart_id * 8) as *mut u64;
            core::ptr::write_volatile(addr, value);
        }
    }

    // 设置相对延迟
    pub fn set_timer(&self, hart_id: usize, delta: u64) {
        let now = self.read_mtime();
        self.set_mtimecmp(hart_id, now + delta);
    }
}

// 启用定时器中断
pub fn enable_timer_interrupt() {
    use riscv::register::{mstatus, mie};

    // 1. 设置陷阱向量
    unsafe {
        core::arch::asm!(
            "la t0, trap_handler",
            "csrw mtvec, t0",
        );
    }

    // 2. 启用定时器中断位
    unsafe {
        mie::set_mtimer();
    }

    // 3. 启用全局中断
    unsafe {
        mstatus::set_mie();
    }

    // 4. 设置第一次中断时间（1秒后）
    let clint = Clint::new();
    clint.set_timer(0, 10_000_000);  // 10MHz * 1s
}
```

### 6.2 PLIC配置（UART中断）

```rust
const PLIC_BASE: usize = 0x0C00_0000;
const PLIC_PRIORITY_BASE: usize = PLIC_BASE + 0x0000;
const PLIC_ENABLE_BASE: usize = PLIC_BASE + 0x2000;
const PLIC_THRESHOLD_BASE: usize = PLIC_BASE + 0x20_0000;
const PLIC_CLAIM_BASE: usize = PLIC_BASE + 0x20_0004;

const UART0_IRQ: u32 = 10;

pub struct Plic {
    base: usize,
}

impl Plic {
    pub const fn new() -> Self {
        Self { base: PLIC_BASE }
    }

    // 设置IRQ优先级（1-7，0为禁用）
    pub fn set_priority(&self, irq: u32, priority: u8) {
        unsafe {
            let addr = (self.base + irq as usize * 4) as *mut u32;
            core::ptr::write_volatile(addr, priority as u32);
        }
    }

    // 使能IRQ（Context 0 = Hart 0 M-mode）
    pub fn enable_irq(&self, context: usize, irq: u32) {
        let offset = PLIC_ENABLE_BASE - PLIC_BASE + context * 0x80;
        let word_offset = irq / 32;
        let bit_offset = irq % 32;

        unsafe {
            let addr = (self.base + offset + word_offset as usize * 4) as *mut u32;
            let mut val = core::ptr::read_volatile(addr);
            val |= 1 << bit_offset;
            core::ptr::write_volatile(addr, val);
        }
    }

    // 设置优先级阈值（0-7）
    pub fn set_threshold(&self, context: usize, threshold: u8) {
        let offset = PLIC_THRESHOLD_BASE - PLIC_BASE + context * 0x1000;
        unsafe {
            let addr = (self.base + offset) as *mut u32;
            core::ptr::write_volatile(addr, threshold as u32);
        }
    }

    // 声明中断（返回IRQ编号）
    pub fn claim(&self, context: usize) -> u32 {
        let offset = PLIC_CLAIM_BASE - PLIC_BASE + context * 0x1000;
        unsafe {
            let addr = (self.base + offset) as *const u32;
            core::ptr::read_volatile(addr)
        }
    }

    // 完成中断
    pub fn complete(&self, context: usize, irq: u32) {
        let offset = PLIC_CLAIM_BASE - PLIC_BASE + context * 0x1000;
        unsafe {
            let addr = (self.base + offset) as *mut u32;
            core::ptr::write_volatile(addr, irq);
        }
    }
}

// 启用UART中断
pub fn enable_uart_interrupt() {
    let plic = Plic::new();

    // 1. 设置UART0优先级为7（最高）
    plic.set_priority(UART0_IRQ, 7);

    // 2. 使能UART0中断（Context 0 = Hart 0 M-mode）
    plic.enable_irq(0, UART0_IRQ);

    // 3. 设置优先级阈值为0（接受所有中断）
    plic.set_threshold(0, 0);

    // 4. 启用外部中断
    unsafe {
        riscv::register::mie::set_mext();
        riscv::register::mstatus::set_mie();
    }
}
```

### 6.3 陷阱处理器骨架

```rust
#[repr(C)]
pub struct TrapFrame {
    pub regs: [usize; 31],  // x1-x31
    pub pc: usize,
}

core::arch::global_asm!(
    ".align 4",
    ".global trap_handler",
    "trap_handler:",
    // 保存上下文
    "    addi sp, sp, -128",
    "    sw x1, 0(sp)",
    "    sw x2, 4(sp)",
    // ... 保存所有寄存器 ...
    "    mv a0, sp",          // TrapFrame指针作为参数
    "    call trap_handler_rust",
    // 恢复上下文
    "    lw x1, 0(sp)",
    // ... 恢复所有寄存器 ...
    "    addi sp, sp, 128",
    "    mret",
);

#[no_mangle]
extern "C" fn trap_handler_rust(frame: &mut TrapFrame) {
    use riscv::register::{mcause, mtval};

    let cause = mcause::read();

    if cause.is_interrupt() {
        match cause.code() {
            3 => handle_software_interrupt(),
            7 => handle_timer_interrupt(),
            11 => handle_external_interrupt(),
            _ => panic!("Unknown interrupt: {}", cause.code()),
        }
    } else {
        // 异常处理
        panic!("Exception: code={}, mtval={:#x}", cause.code(), mtval::read());
    }
}

fn handle_timer_interrupt() {
    kprintln!("[INTERRUPT] Timer");

    // 清除中断：设置下一次中断时间
    let clint = Clint::new();
    clint.set_timer(0, 10_000_000);  // 1秒后再次触发
}

fn handle_external_interrupt() {
    let plic = Plic::new();

    // 声明中断
    let irq = plic.claim(0);

    match irq {
        10 => {
            // UART0中断
            kprintln!("[INTERRUPT] UART0");
            // 处理UART数据...
        }
        _ => {
            kprintln!("[INTERRUPT] Unknown external IRQ {}", irq);
        }
    }

    // 完成中断
    plic.complete(0, irq);
}

fn handle_software_interrupt() {
    kprintln!("[INTERRUPT] Software");
    // 清除MSIP位...
}
```

## 7. 关键要点总结

### 7.1 中断优先级（硬件固定）
```
1. 机器外部中断 (MEI, PLIC)    - 最高
2. 机器软件中断 (MSI)
3. 机器定时器中断 (MTI)        - 最低
```

### 7.2 中断使能三要素
要触发中断，必须同时满足：
1. **全局使能**: `mstatus.MIE = 1`
2. **类型使能**: `mie.MTIE/MSIE/MEIE = 1`
3. **中断挂起**: `mip.MTIP/MSIP/MEIP = 1`（硬件设置）

### 7.3 PLIC注意事项
- IRQ 0 保留，有效范围 1-127
- Claim/Complete寄存器操作必须成对
- Context编号：Hart N M-mode = 2N, S-mode = 2N+1
- 优先级0表示禁用该中断

### 7.4 常见陷阱
1. **忘记mret**: 中断处理完必须用`mret`而非`ret`
2. **重复声明**: 调用`claim()`后必须`complete()`，否则中断不会再次触发
3. **栈污染**: 汇编handler必须完整保存/恢复所有寄存器
4. **定时器无限循环**: MTI触发后必须更新MTIMECMP，否则立即再次触发

## 8. QEMU调试技巧

```bash
# 启动QEMU并打印中断信息
qemu-system-riscv32 -machine virt -d int -nographic -kernel kernel

# 启动GDB调试中断
qemu-system-riscv32 -machine virt -s -S -nographic -kernel kernel
# 另一终端
riscv32-elf-gdb kernel
(gdb) target remote :1234
(gdb) break trap_handler
(gdb) continue
```

## 参考资料

1. RISC-V Privileged Spec v1.11: https://riscv.org/specifications/
2. QEMU virt源码: `hw/riscv/virt.c`
3. PLIC规范: RISC-V Platform-Level Interrupt Controller Specification
4. CLINT规范: SiFive Core-Local Interrupt Controller