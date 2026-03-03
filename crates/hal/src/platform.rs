//! Platform trait for complete hardware abstraction.
//!
//! This module defines the `Platform` trait which serves as the top-level
//! abstraction for a complete hardware platform (e.g., QEMU virt machine).

use crate::interrupt::InterruptController;
use crate::memory::MemoryLayout;
use crate::serial::SerialPort;

/// Trait representing a complete hardware platform.
///
/// This is the main abstraction that ties together all hardware components.
/// Implementations provide access to peripherals and platform-specific information.
///
/// The trait uses associated types to allow platform-specific implementations
/// of the serial port and interrupt controller while maintaining type safety.
pub trait Platform: Send + Sync {
    /// The type of serial port provided by this platform.
    type Serial: SerialPort;

    /// The type of interrupt controller provided by this platform.
    type Interrupt: InterruptController;

    /// Get a reference to the console serial port.
    ///
    /// This is typically the primary UART used for kernel output and debugging.
    ///
    /// # Returns
    /// Reference to the console serial port
    fn console(&self) -> &Self::Serial;

    /// Get a reference to the interrupt controller.
    ///
    /// # Returns
    /// Reference to the platform's interrupt controller
    fn interrupt_controller(&self) -> &Self::Interrupt;

    /// Get the IRQ number for console serial port interrupts.
    ///
    /// This is the interrupt source number that should be enabled/claimed
    /// to receive console serial port interrupts.
    ///
    /// # Returns
    /// Console IRQ number
    fn console_irq(&self) -> u32;

    /// Get the platform name.
    ///
    /// # Returns
    /// A human-readable platform name (e.g., "QEMU virt")
    fn name(&self) -> &'static str;

    /// Get the target architecture.
    ///
    /// # Returns
    /// Architecture string (e.g., "riscv32imac")
    fn arch(&self) -> &'static str;

    /// Reboot the system.
    ///
    /// This method should trigger a system reset or reboot if supported
    /// by the platform. If not supported, it may fall back to an infinite
    /// loop or halt.
    fn reboot(&self) -> !;

    /// Get the memory layout of the system.
    ///
    /// # Returns
    /// A `MemoryLayout` structure describing all major memory regions
    fn memory_layout(&self) -> MemoryLayout;
}
