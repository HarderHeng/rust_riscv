//! RISC-V 64-bit startup code.
//!
//! Provides the `_start` assembly entry point for RV64I architecture.
//! This code:
//! - Initializes the stack pointer
//! - Copies initialized data and zeros the BSS section
//! - Jumps to `kernel_main` in Rust
//!
//! Key difference from RV32: Uses 64-bit loads/stores (ld/sd) instead of 32-bit (lw/sw).

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
// Reset vector (_start) for RV64
// ---------------------------------------------------------------------------

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    ".section .text.start",
    ".global _start",
    "_start:",
    //  1. Point the stack pointer at the top of the reserved stack region.
    "   la   sp, _stack_top",
    //  2. Copy initialized data from its load address to its runtime address.
    "   la   t0, _sidata",
    "   la   t1, _sdata",
    "   la   t2, _edata",
    "3: bgeu t1, t2, 4f",
    "   ld   t3, 0(t0)",
    "   sd   t3, 0(t1)",
    "   addi t0, t0, 8",
    "   addi t1, t1, 8",
    "   j    3b",
    "4:",
    //  3. Zero the BSS segment (required by the C/Rust ABI: statics start at 0).
    "   la   t0, _sbss",
    "   la   t1, _ebss",
    "5: bgeu t0, t1, 6f",
    "   sd   zero, 0(t0)",
    "   addi t0, t0, 8",
    "   j    5b",
    "6:",
    //  4. Hand off to Rust. kernel_main must never return.
    "   j    kernel_main",
);
