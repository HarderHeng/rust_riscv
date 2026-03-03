//! Interrupt controller trait for hardware abstraction.
//!
//! This module defines the `InterruptController` trait which abstracts
//! platform interrupt controllers (e.g., PLIC on RISC-V).

/// Trait for interrupt controller hardware.
///
/// Implementations must be `Send + Sync` to allow safe usage in static contexts.
/// This trait abstracts priority-based interrupt controllers like the RISC-V PLIC.
pub trait InterruptController: Send + Sync {
    /// Set the priority of an interrupt source.
    ///
    /// Higher priority values typically mean higher priority, but the exact
    /// semantics depend on the hardware implementation.
    ///
    /// # Arguments
    /// * `irq` - The interrupt source number
    /// * `priority` - The priority level (0 = disabled on most hardware)
    fn set_priority(&self, irq: u32, priority: u32);

    /// Enable an interrupt source.
    ///
    /// After calling this, the interrupt controller will forward interrupts
    /// from this source to the CPU (subject to priority and threshold).
    ///
    /// # Arguments
    /// * `irq` - The interrupt source number
    fn enable_irq(&self, irq: u32);

    /// Disable an interrupt source.
    ///
    /// After calling this, the interrupt controller will not forward interrupts
    /// from this source to the CPU.
    ///
    /// # Arguments
    /// * `irq` - The interrupt source number
    fn disable_irq(&self, irq: u32);

    /// Set the interrupt priority threshold.
    ///
    /// Only interrupts with priority strictly greater than the threshold will
    /// be delivered to the CPU.
    ///
    /// # Arguments
    /// * `threshold` - The minimum priority for interrupt delivery
    fn set_threshold(&self, threshold: u32);

    /// Claim a pending interrupt.
    ///
    /// This should be called in the interrupt handler to identify which
    /// interrupt source triggered. It typically atomically reads and clears
    /// the pending status.
    ///
    /// # Returns
    /// * `Some(irq)` - The interrupt source number that is pending
    /// * `None` - No interrupt is pending (spurious interrupt)
    fn claim(&self) -> Option<u32>;

    /// Signal completion of interrupt handling.
    ///
    /// This must be called after handling an interrupt claimed with `claim()`.
    /// It allows the interrupt controller to process the next pending interrupt.
    ///
    /// # Arguments
    /// * `irq` - The interrupt source number that was claimed and handled
    fn complete(&self, irq: u32);
}
