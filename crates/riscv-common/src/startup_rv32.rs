//! RISC-V 32-bit startup code.
//!
//! Provides the `_start` assembly entry point for RV32I/RV32E architectures.
//! This code:
//! - Initializes the stack pointer
//! - Zeros the BSS section
//! - Jumps to `kernel_main` in Rust
//!
//! Supports both RV32I (32 registers) and RV32E (16 registers) via conditional compilation.

// ---------------------------------------------------------------------------
// Linker-script symbol declarations
// ---------------------------------------------------------------------------

extern "C" {
    /// First byte of the BSS section (inclusive).
    static _sbss: u8;
    /// First byte past the BSS section (exclusive).
    static _ebss: u8;

    /// Runtime start of .data (VMA).
    static _sdata: u8;
    /// Runtime end of .data (VMA).
    static _edata: u8;
    /// Load address of .data initial values (LMA).
    static _sidata: u8;

    /// First usable heap byte.
    static _heap_start: u8;
    /// First byte past the heap region.
    static _heap_end: u8;

    /// Top of the kernel stack.
    static _stack_top: u8;
}

/// Returns the BSS region as a raw byte range `[start, end)`.
///
/// Used by allocators or diagnostic tools that need to inspect memory layout.
pub fn bss_range() -> (*mut u8, *mut u8) {
    unsafe {
        (
            &_sbss as *const u8 as *mut u8,
            &_ebss as *const u8 as *mut u8,
        )
    }
}

/// Returns the heap region as a raw byte range `[start, end)`.
pub fn heap_range() -> (*mut u8, *mut u8) {
    unsafe {
        (
            &_heap_start as *const u8 as *mut u8,
            &_heap_end as *const u8 as *mut u8,
        )
    }
}

// ---------------------------------------------------------------------------
// Reset vector (_start) for RV32
// ---------------------------------------------------------------------------

#[cfg(target_arch = "riscv32")]
core::arch::global_asm!(
    ".section .text.start",
    ".global _start",
    "_start:",
    //  1. Point the stack pointer at the top of the reserved stack region.
    "   la   sp, _stack_top",

    //  2. Zero the BSS segment (required by the C/Rust ABI: statics start at 0).
    "   la   t0, _sbss",
    "   la   t1, _ebss",
    "1: bgeu t0, t1, 2f",      // Branch if t0 >= t1 (done)
    "   sw   zero, 0(t0)",      // Store word (32-bit) zero at t0
    "   addi t0, t0, 4",        // Increment by 4 bytes
    "   j    1b",               // Jump back to loop start
    "2:",

    //  3. Hand off to Rust. kernel_main must never return.
    "   j    kernel_main",
);

// Optional: RV32E variant (16 registers: x0-x15)
// RV32E is used in embedded systems to reduce hardware cost.
// It uses the same instruction encoding but with a restricted register set.
//
// Note: RV32E requires a different ABI and calling convention.
// For now, we use the same startup code since we only use t0/t1/sp,
// all of which are available in both RV32I and RV32E.
//
// If you need explicit RV32E support with different conventions, add:
// #[cfg(all(target_arch = "riscv32", target_feature = "e"))]

