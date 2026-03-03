//! CPU reset vector and memory layout symbols.
//!
//! `_start` (assembly) initialises the stack and zeros BSS,
//! then jumps to [`kernel_main`] in Rust. No C runtime is used.
//!
//! Linker-script symbols are declared as `extern "C" static`s; taking
//! their *address* yields the corresponding boundary pointer — the value
//! itself is meaningless and must never be read.

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
    /// Load address of .data initial values (LMA, equals VMA on QEMU).
    static _sidata: u8;

    /// First usable heap byte.
    static _heap_start: u8;
    /// First byte past the heap region.
    static _heap_end: u8;
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
// Reset vector (_start)
// ---------------------------------------------------------------------------

core::arch::global_asm!(
    ".section .text.start",
    ".global _start",
    "_start:",
    //  1. Point the stack pointer at the top of the reserved stack region.
    "   la   sp, _stack_top",
    //  2. Zero the BSS segment (required by the C/Rust ABI: statics start at 0).
    "   la   t0, _sbss",
    "   la   t1, _ebss",
    "1: bgeu t0, t1, 2f",
    "   sw   zero, 0(t0)",
    "   addi t0, t0, 4",
    "   j    1b",
    "2:",
    //  3. Hand off to Rust. kernel_main must never return.
    "   j    kernel_main",
);
