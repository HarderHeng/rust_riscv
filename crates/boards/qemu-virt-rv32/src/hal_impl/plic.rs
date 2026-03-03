//! PLIC (Platform-Level Interrupt Controller) driver for QEMU virt machine.
//!
//! PLIC is memory-mapped at 0x0C00_0000 and manages external device interrupts
//! (IRQ 1-127). All register accesses use `read_volatile` / `write_volatile`
//! to prevent compiler optimization of MMIO operations.
//!
//! # QEMU virt IRQ mapping
//!
//! - IRQ 0: Reserved
//! - IRQ 1-8: VirtIO devices
//! - IRQ 9: PCIe
//! - IRQ 10: UART0
//! - IRQ 11+: Extended devices

use hal::InterruptController;

// ---------------------------------------------------------------------------
// Hardware constants
// ---------------------------------------------------------------------------

const PLIC_BASE: usize = 0x0C00_0000;

/// Register offsets relative to PLIC base address.
mod reg {
    /// Priority registers base (4 bytes per IRQ, IRQ 0 reserved).
    pub const PRIORITY_BASE: usize = 0x0000_0000;
    /// Enable bits array base for context 0 (128 bytes per context).
    pub const ENABLE_BASE: usize = 0x0000_2000;
    /// Priority threshold and claim/complete base for context 0.
    pub const CONTEXT_BASE: usize = 0x0020_0000;
}

/// Context stride in bytes (0x1000 per context).
const CONTEXT_STRIDE: usize = 0x1000;

// ---------------------------------------------------------------------------
// Plic type
// ---------------------------------------------------------------------------

/// A handle to the memory-mapped PLIC peripheral.
///
/// This driver currently only supports context 0 (Hart 0 M-mode) for simplicity.
/// Multi-core support would require tracking the current context.
pub struct Plic {
    base: usize,
    context: usize,
}

impl Plic {
    /// Creates a handle for the PLIC at the standard base address.
    ///
    /// Defaults to context 0 (Hart 0 M-mode).
    pub const fn new() -> Self {
        Self {
            base: PLIC_BASE,
            context: 0,
        }
    }

    // --- register access ----------------------------------------------------

    #[inline]
    fn read_u32(&self, offset: usize) -> u32 {
        unsafe { core::ptr::read_volatile((self.base + offset) as *const u32) }
    }

    #[inline]
    fn write_u32(&self, offset: usize, val: u32) {
        unsafe { core::ptr::write_volatile((self.base + offset) as *mut u32, val) }
    }
}

impl InterruptController for Plic {
    /// Sets the priority for a given IRQ (valid range: 0-7).
    ///
    /// - Priority 0 disables the interrupt.
    /// - Priority 1-7: higher values have higher priority.
    fn set_priority(&self, irq: u32, priority: u32) {
        let offset = reg::PRIORITY_BASE + (irq as usize) * 4;
        self.write_u32(offset, priority);
    }

    /// Enables the specified IRQ for context 0.
    fn enable_irq(&self, irq: u32) {
        let word_idx = irq / 32;
        let bit_idx = irq % 32;
        let offset = reg::ENABLE_BASE + self.context * 0x80 + (word_idx as usize) * 4;

        let mut val = self.read_u32(offset);
        val |= 1 << bit_idx;
        self.write_u32(offset, val);
    }

    /// Disables the specified IRQ for context 0.
    fn disable_irq(&self, irq: u32) {
        let word_idx = irq / 32;
        let bit_idx = irq % 32;
        let offset = reg::ENABLE_BASE + self.context * 0x80 + (word_idx as usize) * 4;

        let mut val = self.read_u32(offset);
        val &= !(1 << bit_idx);
        self.write_u32(offset, val);
    }

    /// Sets the priority threshold for context 0.
    ///
    /// Only interrupts with priority > threshold will be delivered.
    fn set_threshold(&self, threshold: u32) {
        let offset = reg::CONTEXT_BASE + self.context * CONTEXT_STRIDE;
        self.write_u32(offset, threshold);
    }

    /// Claims the highest-priority pending interrupt for context 0.
    ///
    /// Returns Some(irq) if an interrupt is pending, or None if no interrupt.
    fn claim(&self) -> Option<u32> {
        let offset = reg::CONTEXT_BASE + self.context * CONTEXT_STRIDE + 4;
        let irq = self.read_u32(offset);
        if irq == 0 {
            None
        } else {
            Some(irq)
        }
    }

    /// Signals that interrupt handling is complete for the given IRQ.
    fn complete(&self, irq: u32) {
        let offset = reg::CONTEXT_BASE + self.context * CONTEXT_STRIDE + 4;
        self.write_u32(offset, irq);
    }
}

// Implement Send + Sync as required by the InterruptController trait.
// Safe because:
// - All hardware access uses volatile operations
// - The PLIC peripheral is a singleton at a fixed MMIO address
// - Multiple cores/contexts can safely access the same PLIC registers
unsafe impl Send for Plic {}
unsafe impl Sync for Plic {}
