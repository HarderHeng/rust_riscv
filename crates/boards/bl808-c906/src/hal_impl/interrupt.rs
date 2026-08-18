//! BL808 interrupt controller driver (placeholder).

use hal::InterruptController;

/// A handle to the BL808 interrupt controller.
///
/// Placeholder implementation awaiting the BL808 memory map and interrupt
/// controller register definitions.
pub struct Bl808InterruptController;

impl Bl808InterruptController {
    /// Creates a new interrupt controller handle.
    pub const fn new() -> Self {
        Self
    }
}

impl InterruptController for Bl808InterruptController {
    /// Placeholder priority configuration.
    fn set_priority(&self, _irq: u32, _priority: u32) {
        // TODO: Determine PLIC base address, source count and priority layout
        // Also C906-specific interrupt routing and M/S-mode handling
        todo!("BL808 interrupt priority not yet implemented")
    }

    /// Placeholder IRQ enable.
    fn enable_irq(&self, _irq: u32) {
        todo!("BL808 interrupt enable not yet implemented")
    }

    /// Placeholder IRQ disable.
    fn disable_irq(&self, _irq: u32) {
        todo!("BL808 interrupt disable not yet implemented")
    }

    /// Placeholder threshold configuration.
    fn set_threshold(&self, _threshold: u32) {
        todo!("BL808 interrupt threshold not yet implemented")
    }

    /// Placeholder IRQ claim.
    fn claim(&self) -> Option<u32> {
        todo!("BL808 interrupt claim not yet implemented")
    }

    /// Placeholder IRQ completion.
    fn complete(&self, _irq: u32) {
        todo!("BL808 interrupt complete not yet implemented")
    }
}
