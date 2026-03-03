use hal::InterruptController;

pub struct Bl808InterruptController;

impl Bl808InterruptController {
    pub const fn new() -> Self {
        Self
    }
}

impl InterruptController for Bl808InterruptController {
    fn init(&mut self) {
        // TODO: Initialize BL808 interrupt controller
        // Need to determine:
        // - PLIC (Platform-Level Interrupt Controller) base address
        // - Number of interrupt sources
        // - Priority and threshold configuration
        todo!("BL808 interrupt controller initialization not yet implemented")
    }

    fn enable_interrupt(&mut self, irq: usize) {
        // TODO: Enable specific interrupt
        todo!("BL808 interrupt enable not yet implemented")
    }

    fn disable_interrupt(&mut self, irq: usize) {
        // TODO: Disable specific interrupt
        todo!("BL808 interrupt disable not yet implemented")
    }

    fn set_priority(&mut self, irq: usize, priority: u8) {
        // TODO: Set interrupt priority
        todo!("BL808 interrupt priority not yet implemented")
    }

    fn claim_interrupt(&mut self) -> Option<usize> {
        // TODO: Claim pending interrupt
        todo!("BL808 interrupt claim not yet implemented")
    }

    fn complete_interrupt(&mut self, irq: usize) {
        // TODO: Signal interrupt completion
        todo!("BL808 interrupt complete not yet implemented")
    }
}
