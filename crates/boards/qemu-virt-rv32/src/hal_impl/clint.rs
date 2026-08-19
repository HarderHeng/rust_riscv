//! CLINT timer driver for the QEMU `virt` machine.

use core::ptr::{read_volatile, write_volatile};

/// CLINT MMIO base address.
pub const CLINT_BASE: usize = 0x0200_0000;
const MTIMECMP_OFFSET: usize = 0x4000;
const MTIME_OFFSET: usize = 0xBFF8;

/// QEMU's default CLINT timebase frequency.
pub const MTIME_HZ: u64 = 10_000_000;

/// Handle for the machine timer registers of one QEMU `virt` CLINT.
pub struct Clint {
    base: usize,
}

impl Clint {
    /// Creates a handle for the QEMU `virt` CLINT.
    pub const fn new() -> Self {
        Self { base: CLINT_BASE }
    }

    /// Reads the 64-bit machine time register consistently.
    pub fn read_mtime(&self) -> u64 {
        loop {
            let high = unsafe { read_volatile((self.base + MTIME_OFFSET + 4) as *const u32) };
            let low = unsafe { read_volatile((self.base + MTIME_OFFSET) as *const u32) };
            let high_again = unsafe { read_volatile((self.base + MTIME_OFFSET + 4) as *const u32) };
            if high == high_again {
                return ((high as u64) << 32) | low as u64;
            }
        }
    }

    /// Sets a hart's absolute timer compare value.
    ///
    /// The high word is written first so a partially updated future deadline
    /// cannot briefly become an earlier deadline.
    pub fn set_mtimecmp(&self, hart: usize, value: u64) {
        let address = self.base + MTIMECMP_OFFSET + hart * 8;
        unsafe {
            write_volatile((address + 4) as *mut u32, (value >> 32) as u32);
            write_volatile(address as *mut u32, value as u32);
        }
    }

    /// Arms a one-shot timer `delta` ticks from the current time.
    pub fn set_timeout(&self, hart: usize, delta: u64) {
        self.set_mtimecmp(hart, self.read_mtime().wrapping_add(delta));
    }
}
