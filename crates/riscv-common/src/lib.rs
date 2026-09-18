//! RISC-V Common Utilities
//!
//! Provides shared startup code and CSR utilities for RISC-V 32-bit and 64-bit architectures.
//!
//! # Architecture Support
//!
//! This crate provides architecture-specific startup code that is automatically selected
//! based on the target:
//!
//! - **RV32I/RV32E**: Use `startup_rv32` module (32-bit instructions)
//! - **RV64I**: Use `startup_rv64` module (64-bit instructions)
//!
//! # Features
//!
//! - **Startup Code**: Assembly `_start` entry point that:
//!   - Initializes stack pointer
//!   - Zeros BSS section
//!   - Jumps to `kernel_main`
//!
//! - **CSR Access**: Safe macros and functions for Control and Status Register operations:
//!   - `csr_read!`, `csr_write!`, `csr_set!`, `csr_clear!`, `csr_swap!`
//!   - Helper functions: `enable_interrupts()`, `disable_interrupts()`, `set_trap_vector()`
//!   - Bit field definitions for common CSRs (mstatus, mie, mip, mcause)
//!
//! # Usage
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! riscv-common = { path = "../crates/riscv-common" }
//! ```
//!
//! The startup code requires these linker symbols to be defined in your linker script:
//!
//! - `_sbss`, `_ebss`: BSS section boundaries
//! - `_sdata`, `_edata`, `_sidata`: Data section boundaries
//! - `_heap_start`, `_heap_end`: Heap region boundaries
//! - `_stack_top`: Top of the kernel stack
//!
//! And requires a `kernel_main` function:
//!
//! ```no_run
//! #[no_mangle]
//! extern "C" fn kernel_main() -> ! {
//!     // Your kernel code here
//!     loop {}
//! }
//! ```

#![no_std]
#![deny(missing_docs)]

// Re-export CSR utilities (available for all architectures)
pub mod csr;

// Architecture-specific startup code
#[cfg(target_arch = "riscv32")]
mod startup_rv32;

#[cfg(target_arch = "riscv64")]
mod startup_rv64;

// Re-export the appropriate startup module based on architecture
#[cfg(target_arch = "riscv32")]
pub use startup_rv32::*;

#[cfg(target_arch = "riscv64")]
pub use startup_rv64::*;

// Ensure we're compiling for a supported architecture
#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
compile_error!("riscv-common only supports riscv32 and riscv64 architectures");
