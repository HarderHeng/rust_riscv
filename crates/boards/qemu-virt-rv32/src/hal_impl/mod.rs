//! HAL implementations for QEMU virt-rv32 board.

pub mod clint;
pub mod plic;
pub mod uart_16550a;
#[cfg(feature = "virtio-console")]
pub mod virtio_console;

pub use clint::{Clint, MTIME_HZ};
pub use plic::Plic;
pub use uart_16550a::Uart16550a;
#[cfg(feature = "virtio-console")]
pub use virtio_console::VirtioConsole;
