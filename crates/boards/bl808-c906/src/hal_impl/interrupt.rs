//! BL808 C906 PLIC interrupt controller driver.

use core::ptr::{read_volatile, write_volatile};

use hal::InterruptController;

// The C906 SDK maps its PLIC at this core-local address.
const PLIC_BASE: usize = 0xE000_0000;
const PLIC_ENABLE_BASE: usize = 0x2000;
const PLIC_THRESHOLD: usize = 0x200000;
const PLIC_CLAIM: usize = 0x200004;

/// C906 interrupt controller backed by the SDK PLIC.
pub struct Bl808InterruptController;

impl Bl808InterruptController {
    /// Creates a PLIC handle for hart 0 machine mode.
    pub const fn new() -> Self {
        Self
    }

    #[inline]
    fn read(offset: usize) -> u32 {
        unsafe { read_volatile((PLIC_BASE + offset) as *const u32) }
    }

    #[inline]
    fn write(offset: usize, value: u32) {
        unsafe { write_volatile((PLIC_BASE + offset) as *mut u32, value) }
    }
}

impl InterruptController for Bl808InterruptController {
    fn set_priority(&self, irq: u32, priority: u32) {
        Self::write(irq as usize * 4, priority);
    }

    fn enable_irq(&self, irq: u32) {
        let word = irq / 32;
        let bit = irq % 32;
        let offset = PLIC_ENABLE_BASE + word as usize * 4;
        Self::write(offset, Self::read(offset) | (1 << bit));
    }

    fn disable_irq(&self, irq: u32) {
        let word = irq / 32;
        let bit = irq % 32;
        let offset = PLIC_ENABLE_BASE + word as usize * 4;
        Self::write(offset, Self::read(offset) & !(1 << bit));
    }

    fn set_threshold(&self, threshold: u32) {
        Self::write(PLIC_THRESHOLD, threshold);
    }

    fn claim(&self) -> Option<u32> {
        match Self::read(PLIC_CLAIM) {
            0 => None,
            irq => Some(irq),
        }
    }

    fn complete(&self, irq: u32) {
        Self::write(PLIC_CLAIM, irq);
    }
}
