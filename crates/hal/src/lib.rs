//! Hardware Abstraction Layer (HAL) for bare-metal RISC-V kernels.
//!
//! This crate provides trait definitions for abstracting hardware-specific
//! functionality, enabling portable kernel code that can run on different
//! platforms (QEMU virt, physical boards, etc.).
//!
//! # Architecture
//!
//! The HAL is built around several key traits:
//!
//! - [`Platform`]: Top-level abstraction representing a complete hardware platform
//! - [`SerialPort`]: UART/serial communication interface
//! - [`InterruptController`]: Priority-based interrupt controller (e.g., PLIC)
//!
//! All traits require `Send + Sync` to enable safe usage in static contexts,
//! which is essential for bare-metal environments where global hardware access
//! is common.
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use hal::{Platform, SerialPort};
//!
//! fn kernel_main<P: Platform>(platform: &P) {
//!     // Initialize console
//!     platform.console().init();
//!     platform.console().puts("Hello from HAL!\r\n");
//!
//!     // Enable console interrupts
//!     let irq = platform.console_irq();
//!     platform.interrupt_controller().set_priority(irq, 1);
//!     platform.interrupt_controller().enable_irq(irq);
//! }
//! ```

#![no_std]
#![deny(missing_docs)]

pub mod interrupt;
pub mod memory;
pub mod platform;
pub mod serial;

// Re-export main types for convenience
pub use interrupt::InterruptController;
pub use memory::{MemoryLayout, MemoryRegion};
pub use platform::Platform;
pub use serial::SerialPort;
