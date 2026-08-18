//! CPU reset vector and startup code for RISC-V 32-bit.
//!
//! This module provides linker symbol declarations for this board.
//! The `_start` assembly entry point is defined by the `riscv-common`
//! crate's startup code.

// ---------------------------------------------------------------------------
// Linker-script symbol declarations
// ---------------------------------------------------------------------------

/// Retains the riscv-common object in the final link.
///
/// Without a reference, the linker would not extract `riscv-common` from its
/// archive and would drop the `_start` entry symbol it defines via `global_asm!`.
/// The linker script sets `ENTRY(_start)`, so this symbol must resolve.
#[used]
static _KEEP_RISCV_COMMON: fn() -> (*mut u8, *mut u8) = riscv_common::bss_range;

extern "C" {
    /// Start of text section
    pub static _stext: u8;
    /// End of text section
    pub static _etext: u8;

    /// First byte of the BSS section (inclusive).
    pub static _sbss: u8;
    /// First byte past the BSS section (exclusive).
    pub static _ebss: u8;

    /// Runtime start of .data (VMA).
    pub static _sdata: u8;
    /// Runtime end of .data (VMA).
    pub static _edata: u8;
    /// Load address of .data initial values (LMA, equals VMA on QEMU).
    pub static _sidata: u8;

    /// Bottom of stack
    pub static _stack_bottom: u8;
    /// Top of stack (initial SP value)
    pub static _stack_top: u8;

    /// First usable heap byte.
    pub static _heap_start: u8;
    /// First byte past the heap region.
    pub static _heap_end: u8;
}
