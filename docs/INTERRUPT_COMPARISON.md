# RISC-V vs ARM Cortex-M 中断架构对比

## 快速回答你的问题

**是的，你的理解是对的，但有细节补充：**

1. ✅ RISC-V确实主要依靠软件分发（在trap_handler中通过mcause判断）
2. ⚠️ RISC-V **有有限的硬件分发机制**（Vectored Mode），但只支持核心的3个中断
3. ❌ RISC-V **没有**像ARM那样的完整硬件向量表（每个外设一个表项）

---

## 架构对比图

### ARM Cortex-M（完整硬件向量表）

```
┌─────────────────────────────────────────────────────────┐
│                   向量表（ROM/Flash）                     │
├─────────────┬───────────────────────────────────────────┤
│  地址       │  向量（32位函数指针）                      │
├─────────────┼───────────────────────────────────────────┤
│ 0x0000_0000 │ _stack_top          (初始栈指针)          │
│ 0x0000_0004 │ Reset_Handler       (复位)                │
│ 0x0000_0008 │ NMI_Handler         (不可屏蔽中断)         │
│ 0x0000_000C │ HardFault_Handler   (硬故障)              │
│ 0x0000_0010 │ MemManage_Handler   (内存管理故障)         │
│     ...     │            ...                            │
│ 0x0000_0040 │ ► UART0_IRQHandler  (外设0)  ────┐       │
│ 0x0000_0044 │ ► UART1_IRQHandler  (外设1)      │       │
│ 0x0000_0048 │ ► Timer0_IRQHandler (外设2)      │ 硬件   │
│ 0x0000_004C │ ► GPIO_IRQHandler   (外设3)      │ 自动   │
│     ...     │            ...                   │ 索引   │
│ 0x0000_01FC │ ► IRQ127_Handler    (外设127)  ──┘       │
└─────────────┴───────────────────────────────────────────┘
                             │
            硬件自动根据中断号加载PC ← NVIC
                             │
           ┌─────────────────┴─────────────────┐
           │ 例如：UART0中断触发                 │
           │ → NVIC查表：vector_table[16]      │
           │ → PC = UART0_IRQHandler地址        │
           │ → 直接跳转，无软件开销！           │
           └───────────────────────────────────┘
```

**特点**：
- ✅ 硬件自动分发，延迟低
- ✅ 支持数百个中断源
- ✅ 每个中断有独立入口
- ❌ 需要完整向量表（占内存）
- ❌ 硬件复杂度高

---

### RISC-V（两级分发：核心硬件 + PLIC软件）

```
┌─────────────────────────────────────────────────────────┐
│              mtvec寄存器（CPU CSR）                      │
├─────────────────────────────────────────────────────────┤
│  [31:2] = BASE (trap handler基址)                       │
│  [1:0]  = MODE (00=Direct, 01=Vectored)                 │
└─────────────────────────────────────────────────────────┘
                             │
        ┌────────────────────┴─────────────────────┐
        │                                          │
   MODE = 0                                   MODE = 1
   Direct                                     Vectored
   (软件分发)                                 (有限硬件分发)
        │                                          │
        ▼                                          ▼
┌─────────────────┐                    ┌─────────────────────┐
│ 所有trap都跳转   │                    │ 中断：BASE+4*cause  │
│ 到 BASE         │                    │ 异常：BASE          │
├─────────────────┤                    ├─────────────────────┤
│ trap_handler:   │                    │ BASE+0x00: 异常     │
│   保存寄存器     │                    │ BASE+0x0C: MSI(3)   │
│   读mcause      │                    │ BASE+0x1C: MTI(7)   │
│   if 中断?      │                    │ BASE+0x2C: MEI(11)  │
│     match code: │                    │ （每项4字节）        │
│       3  → MSI  │                    └─────────────────────┘
│       7  → MTI  │                              │
│       11 → MEI ─┼──────────────────────────────┘
│   else:         │
│     异常处理     │
└─────────────────┘
         │
         │ MEI (机器外部中断)
         │ ↓ 还需要第二级分发！
         │
┌────────▼──────────────────────────────────────────────┐
│               PLIC (Platform-Level IC)                │
├───────────────────────────────────────────────────────┤
│  IRQ 1-127 都汇聚到一个MEI信号                         │
│                                                       │
│  软件必须查询PLIC才能知道具体哪个IRQ:                   │
│                                                       │
│  let irq = plic.claim(0);  // 读MMIO寄存器            │
│  match irq {                                          │
│      1  → VirtIO Block                                │
│      10 → UART0          ← 你的项目                   │
│      ...                                              │
│  }                                                    │
│  plic.complete(0, irq);    // 完成中断                │
└───────────────────────────────────────────────────────┘
```

**特点**：
- ✅ 硬件简单，灵活
- ✅ 支持任意数量外部中断（通过PLIC）
- ✅ 软件可控制复杂分发逻辑
- ❌ 两级分发有额外开销
- ❌ 中断延迟比ARM略高

---

## 中断处理时间对比

### ARM Cortex-M
```
1. 中断触发
2. 硬件自动压栈（8个寄存器）      约12个周期
3. 硬件查向量表并加载PC            约2个周期
4. 跳转到中断处理函数               约1个周期
   ━━━━━━━━━━━━━━━━━━━━━━━━━
   总计：约15个周期
```

### RISC-V（你的实现）
```
1. 中断触发
2. 跳转到trap_handler（硬件）      约2个周期
3. 保存31个寄存器（软件汇编）       约31条指令
4. 读mcause判断中断类型             约2个周期
5. match分支跳转                    约1-3个周期
6. 读PLIC claim获取IRQ号            约5个周期（MMIO）
7. match分支到具体处理器             约1-3个周期
   ━━━━━━━━━━━━━━━━━━━━━━━━━
   总计：约45-50个周期
```

**差距主要在寄存器保存和软件分发**

---

## 你的项目：实际中断流程

### 当用户输入字符 'A' 时：

```
1. UART0硬件接收字符 'A'
   └─► UART0_RX FIFO有数据

2. UART0触发中断 → PLIC
   └─► PLIC.Pending[10] = 1

3. PLIC检查优先级和使能
   - Priority[10] = 7 > Threshold[0] = 0 ✓
   - Enable[10] = 1 ✓
   └─► 向CPU发送MEI信号

4. CPU响应外部中断
   - mip.MEIP = 1
   - mie.MEIE = 1 ✓
   - mstatus.MIE = 1 ✓
   └─► PC = mtvec (trap_handler)

5. trap_handler (汇编, src/trap.rs:39-88)
   - 保存x1-x31到栈
   - 保存mepc
   - 调用trap_handler_rust(frame)

6. trap_handler_rust (Rust, src/trap.rs:106)
   - 读mcause: 0x8000_000B (中断, code=11)
   - is_interrupt = true, code = 11
   - 匹配到: 11 => handle_external_interrupt()

7. handle_external_interrupt (src/trap.rs:125)
   - irq = plic.claim(0)    // 返回10
   - 匹配到: 10 => handle_uart_interrupt()

8. handle_uart_interrupt (src/trap.rs:148)
   - c = uart.try_getc()    // 读取 'A'
   - uart.putc(c)           // 回显 'A'

9. 返回路径
   - plic.complete(0, 10)   // 清除中断
   - trap_handler恢复寄存器
   - mret返回到被中断的代码
```

---

## Vectored Mode示例（可选优化）

如果你想使用Vectored Mode减少延迟：

```rust
// 1. 创建向量表（每个入口4字节）
core::arch::global_asm!(
    ".align 4",
    ".global trap_vector_table",
    "trap_vector_table:",
    "    j handle_exception",       // 0x00: 异常入口
    "    j trap_vector_table",      // 0x04: 保留
    "    j trap_vector_table",      // 0x08: 保留
    "    j handle_msi",             // 0x0C: cause=3, 软件中断
    "    j trap_vector_table",      // 0x10: 保留
    "    j trap_vector_table",      // 0x14: 保留
    "    j trap_vector_table",      // 0x18: 保留
    "    j handle_mti",             // 0x1C: cause=7, 定时器中断
    "    j trap_vector_table",      // 0x20: 保留
    "    j trap_vector_table",      // 0x24: 保留
    "    j trap_vector_table",      // 0x28: 保留
    "    j handle_mei",             // 0x2C: cause=11, 外部中断
);

// 2. 设置mtvec为Vectored模式
pub fn init_vectored() {
    unsafe {
        core::arch::asm!(
            "la t0, trap_vector_table",
            "ori t0, t0, 1",        // MODE = 1
            "csrw mtvec, t0",
        );
    }
}

// 3. 每个处理器可以直接处理，无需软件分支
handle_mei:
    // 保存寄存器
    addi sp, sp, -128
    ... (保存所有寄存器)
    call handle_external_interrupt_rust
    ... (恢复所有寄存器)
    mret
```

**性能提升**：节省约5-10个周期（无需读mcause + match分支）

**但是**：
- 外部中断仍需要查询PLIC（无法避免）
- 代码复杂度增加（需要管理多个入口点）
- 对于只有3个核心中断的系统，收益有限

---

## 总结

| 特性 | ARM Cortex-M | RISC-V (Direct) | RISC-V (Vectored) |
|------|-------------|----------------|------------------|
| **硬件向量表** | ✅ 完整硬件 | ❌ 无 | ⚠️ 有限（3个核心中断）|
| **外设中断分发** | 硬件自动 | 软件查询PLIC | 软件查询PLIC |
| **延迟** | ~15周期 | ~45-50周期 | ~35-40周期 |
| **灵活性** | 低 | 高 | 中 |
| **硬件成本** | 高 | 低 | 低 |
| **适用场景** | MCU | 通用SoC | 性能敏感应用 |

**你的项目选择Direct模式是合理的**，因为：
1. 代码简单易懂
2. 只有1个外设中断（UART），性能差异可忽略
3. 灵活性高，易于扩展

如果将来有性能要求（如大量高频中断），可以考虑切换到Vectored模式。