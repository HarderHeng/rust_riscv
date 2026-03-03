//! Trap (interrupt and exception) handler for RISC-V M-mode.
//!
//! This module provides the low-level trap entry point (`trap_handler`) in
//! assembly, which saves all general-purpose registers to the stack, calls
//! the Rust dispatcher (`trap_handler_rust`), and restores registers before
//! returning via `mret`.
//!
//! # CSR Registers
//!
//! - `mtvec`: Trap vector base address, points to `trap_handler`.
//! - `mcause`: Trap cause (interrupt bit + exception code).
//! - `mepc`: Machine exception program counter (return address).
//! - `mstatus`: Machine status (MIE = global interrupt enable).
//! - `mie`: Machine interrupt enable (MTIE, MSIE, MEIE).

use crate::plic::{Plic, UART0_IRQ};
use crate::uart::Uart;
use crate::{kprintln, uart};

// ---------------------------------------------------------------------------
// TrapFrame structure
// ---------------------------------------------------------------------------

/// Saved register context for trap handling.
///
/// Layout matches the assembly code in `trap_handler`:
/// - regs[0..31] = x1-x31 (x0 is hardwired to 0, not saved)
/// - mepc = return address
#[repr(C)]
pub struct TrapFrame {
    pub regs: [usize; 31],  // x1-x31
    pub mepc: usize,        // Machine exception PC
}

// ---------------------------------------------------------------------------
// Assembly trap entry point
// ---------------------------------------------------------------------------

core::arch::global_asm!(
    ".align 4",
    ".global trap_handler",
    "trap_handler:",
    // Save all registers to stack (32 words = 128 bytes)
    "    addi sp, sp, -128",
    "    sw x1,   0(sp)",    // ra
    "    sw x2,   4(sp)",    // sp (original value before addi)
    "    sw x3,   8(sp)",    // gp
    "    sw x4,  12(sp)",    // tp
    "    sw x5,  16(sp)",    // t0
    "    sw x6,  20(sp)",    // t1
    "    sw x7,  24(sp)",    // t2
    "    sw x8,  28(sp)",    // s0/fp
    "    sw x9,  32(sp)",    // s1
    "    sw x10, 36(sp)",    // a0
    "    sw x11, 40(sp)",    // a1
    "    sw x12, 44(sp)",    // a2
    "    sw x13, 48(sp)",    // a3
    "    sw x14, 52(sp)",    // a4
    "    sw x15, 56(sp)",    // a5
    "    sw x16, 60(sp)",    // a6
    "    sw x17, 64(sp)",    // a7
    "    sw x18, 68(sp)",    // s2
    "    sw x19, 72(sp)",    // s3
    "    sw x20, 76(sp)",    // s4
    "    sw x21, 80(sp)",    // s5
    "    sw x22, 84(sp)",    // s6
    "    sw x23, 88(sp)",    // s7
    "    sw x24, 92(sp)",    // s8
    "    sw x25, 96(sp)",    // s9
    "    sw x26, 100(sp)",   // s10
    "    sw x27, 104(sp)",   // s11
    "    sw x28, 108(sp)",   // t3
    "    sw x29, 112(sp)",   // t4
    "    sw x30, 116(sp)",   // t5
    "    sw x31, 120(sp)",   // t6
    // Save mepc
    "    csrr t0, mepc",
    "    sw t0, 124(sp)",
    // Call Rust handler with TrapFrame pointer
    "    mv a0, sp",
    "    call trap_handler_rust",
    // Restore mepc
    "    lw t0, 124(sp)",
    "    csrw mepc, t0",
    // Restore all registers
    "    lw x1,   0(sp)",
    "    lw x3,   8(sp)",
    "    lw x4,  12(sp)",
    "    lw x5,  16(sp)",
    "    lw x6,  20(sp)",
    "    lw x7,  24(sp)",
    "    lw x8,  28(sp)",
    "    lw x9,  32(sp)",
    "    lw x10, 36(sp)",
    "    lw x11, 40(sp)",
    "    lw x12, 44(sp)",
    "    lw x13, 48(sp)",
    "    lw x14, 52(sp)",
    "    lw x15, 56(sp)",
    "    lw x16, 60(sp)",
    "    lw x17, 64(sp)",
    "    lw x18, 68(sp)",
    "    lw x19, 72(sp)",
    "    lw x20, 76(sp)",
    "    lw x21, 80(sp)",
    "    lw x22, 84(sp)",
    "    lw x23, 88(sp)",
    "    lw x24, 92(sp)",
    "    lw x25, 96(sp)",
    "    lw x26, 100(sp)",
    "    lw x27, 104(sp)",
    "    lw x28, 108(sp)",
    "    lw x29, 112(sp)",
    "    lw x30, 116(sp)",
    "    lw x31, 120(sp)",
    "    lw x2,   4(sp)",    // sp last
    "    addi sp, sp, 128",
    // Return from trap
    "    mret",
);

// ---------------------------------------------------------------------------
// CSR register access
// ---------------------------------------------------------------------------

/// Machine cause register (read-only view of trap cause).
#[inline]
fn read_mcause() -> usize {
    let cause: usize;
    unsafe {
        core::arch::asm!("csrr {}, mcause", out(reg) cause);
    }
    cause
}

/// Machine trap value register (faulting address or other info).
#[inline]
fn read_mtval() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!("csrr {}, mtval", out(reg) val);
    }
    val
}

/// Checks if the trap was an interrupt (vs. exception).
#[inline]
fn is_interrupt(mcause: usize) -> bool {
    (mcause & (1 << 31)) != 0
}

/// Extracts the exception/interrupt code from mcause.
#[inline]
fn cause_code(mcause: usize) -> usize {
    mcause & 0x7FFF_FFFF
}

// ---------------------------------------------------------------------------
// Rust trap dispatcher
// ---------------------------------------------------------------------------

/// Rust entry point for trap handling.
///
/// Called from assembly `trap_handler` with a pointer to the saved TrapFrame.
/// Dispatches to specific handlers based on mcause.
#[no_mangle]
extern "C" fn trap_handler_rust(_frame: &mut TrapFrame) {
    let mcause = read_mcause();

    if is_interrupt(mcause) {
        match cause_code(mcause) {
            3 => handle_software_interrupt(),
            7 => handle_timer_interrupt(),
            11 => handle_external_interrupt(),
            _ => {
                kprintln!("[TRAP] Unknown interrupt code: {}", cause_code(mcause));
            }
        }
    } else {
        // Exception (synchronous trap)
        let code = cause_code(mcause);
        let mtval = read_mtval();
        kprintln!("[TRAP] Exception: code={}, mtval={:#010x}", code, mtval);
        kprintln!("       Halting...");
        loop {
            unsafe { core::arch::asm!("wfi") };
        }
    }
}

// ---------------------------------------------------------------------------
// Interrupt handlers
// ---------------------------------------------------------------------------

/// Handles machine software interrupt (MSI, code 3).
fn handle_software_interrupt() {
    kprintln!("[INTERRUPT] Software");
    // To clear MSI: write 0 to CLINT MSIP register for this hart
    // (Not implemented yet, CLINT driver needed)
}

/// Handles machine timer interrupt (MTI, code 7).
fn handle_timer_interrupt() {
    kprintln!("[INTERRUPT] Timer");
    // To clear MTI: update CLINT MTIMECMP to a future value
    // (Not implemented yet, CLINT driver needed)
}

/// Handles machine external interrupt (MEI, code 11, from PLIC).
fn handle_external_interrupt() {
    let plic = Plic::new();
    let irq = plic.claim(0);  // Context 0 = Hart 0 M-mode

    if irq == 0 {
        kprintln!("[INTERRUPT] External: spurious (IRQ 0)");
        return;
    }

    match irq {
        UART0_IRQ => handle_uart_interrupt(),
        _ => {
            kprintln!("[INTERRUPT] External: unknown IRQ {}", irq);
        }
    }

    plic.complete(0, irq);
}

/// Handles UART0 receive interrupt.
fn handle_uart_interrupt() {
    let uart = Uart::new(uart::UART0_BASE);

    // Read all available bytes and echo them back
    while let Some(byte) = uart.try_getc() {
        // Echo the character
        uart.putc(byte);
    }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Initializes the trap vector to point to the assembly `trap_handler`.
///
/// This must be called before enabling any interrupts.
pub fn init() {
    unsafe {
        core::arch::asm!(
            "la t0, trap_handler",
            "csrw mtvec, t0",
        );
    }
}

/// Enables machine external interrupts (MEI) from PLIC.
pub fn enable_external_interrupts() {
    unsafe {
        // Set MEIE bit (bit 11) in mie register
        let mask: usize = 1 << 11;  // 0x800
        core::arch::asm!("csrs mie, {}", in(reg) mask);
    }
}

/// Enables global interrupts by setting MIE bit in mstatus.
pub fn enable_global_interrupts() {
    unsafe {
        // Set MIE bit (bit 3) in mstatus register
        let mask: usize = 1 << 3;  // 0x8
        core::arch::asm!("csrs mstatus, {}", in(reg) mask);
    }
}