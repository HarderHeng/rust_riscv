//! BL808 E907 CLIC interrupt controller driver.

use core::ptr::{read_volatile, write_volatile};

use hal::InterruptController;

const CLIC_BASE: usize = 0xE080_0000;
const CLIC_INFO: usize = CLIC_BASE + 0x04;
const CLIC_MINTTHRESH: usize = CLIC_BASE + 0x08;
const CLIC_INT_BASE: usize = CLIC_BASE + 0x1000;

/// E907 interrupt controller backed by the T-Head CLIC.
pub struct Bl808InterruptController;

impl Bl808InterruptController {
    /// Creates a CLIC handle.
    pub const fn new() -> Self {
        Self
    }

    #[inline]
    fn byte_address(irq: u32, offset: usize) -> usize {
        CLIC_INT_BASE + irq as usize * 4 + offset
    }
}

impl InterruptController for Bl808InterruptController {
    fn set_priority(&self, irq: u32, priority: u32) {
        let info = unsafe { read_volatile(CLIC_INFO as *const u32) };
        let nlbits = ((info >> 21) & 0xF).min(8);
        let shift = 8 - nlbits;
        let value = ((priority.min(0xF) << shift) & 0xF0) as u8;
        let address = Self::byte_address(irq, 3);
        let old = unsafe { read_volatile(address as *const u8) };
        unsafe { write_volatile(address as *mut u8, (old & 0x0F) | value) };
    }

    fn enable_irq(&self, irq: u32) {
        let address = Self::byte_address(irq, 1);
        let value = unsafe { read_volatile(address as *const u8) };
        unsafe { write_volatile(address as *mut u8, value | 1) };
    }

    fn disable_irq(&self, irq: u32) {
        let address = Self::byte_address(irq, 1);
        let value = unsafe { read_volatile(address as *const u8) };
        unsafe { write_volatile(address as *mut u8, value & !1) };
    }

    fn set_threshold(&self, threshold: u32) {
        unsafe { write_volatile(CLIC_MINTTHRESH as *mut u32, threshold) };
    }

    fn claim(&self) -> Option<u32> {
        // CLIC dispatches directly to the vector; unlike PLIC it has no
        // memory-mapped claim register.
        None
    }

    fn complete(&self, _irq: u32) {
        // CLIC external interrupts are completed by returning from the trap.
    }
}
