//! HAL implementations for QEMU virt-rv32 board.

pub mod plic;
pub mod uart_16550a;

pub use plic::Plic;
pub use uart_16550a::Uart16550a;
