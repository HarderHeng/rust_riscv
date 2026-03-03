//! PLIC (Platform-Level Interrupt Controller) driver for QEMU virt machine.
//!
//! PLIC is memory-mapped at [`PLIC_BASE`] (0x0C00_0000) and manages external
//! device interrupts (IRQ 1-127). All register accesses use `read_volatile` /
//! `write_volatile` to prevent compiler optimization of MMIO operations.
//!
//! # Usage
//!
//! ```rust
//! let plic = Plic::new();
//! plic.set_priority(UART0_IRQ, 7);         // Set UART IRQ priority to 7 (highest)
//! plic.enable_irq(0, UART0_IRQ);           // Enable UART IRQ for context 0 (Hart 0 M-mode)
//! plic.set_threshold(0, 0);                // Accept all interrupts (threshold = 0)
//!
//! // In trap handler:
//! let irq = plic.claim(0);                 // Claim interrupt
//! handle_irq(irq);                         // Handle the interrupt
//! plic.complete(0, irq);                   // Signal completion
//! ```
//!
//! # QEMU virt IRQ mapping
//!
//! - IRQ 0: Reserved
//! - IRQ 1-8: VirtIO devices
//! - IRQ 9: PCIe
//! - IRQ 10: UART0
//! - IRQ 11+: Extended devices

// ---------------------------------------------------------------------------
// Hardware constants
// ---------------------------------------------------------------------------

pub const PLIC_BASE: usize = 0x0C00_0000;

/// UART0 interrupt request number.
pub const UART0_IRQ: u32 = 10;

/// Register offsets relative to PLIC base address.
mod reg {
    /// Priority registers base (4 bytes per IRQ, IRQ 0 reserved).
    pub const PRIORITY_BASE: usize = 0x0000_0000;
    /// Pending bits array base (read-only bitmap).
    pub const PENDING_BASE: usize = 0x0000_1000;
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
/// The struct is cheap to copy (just a base address) so callers can create
/// short-lived instances without overhead.
#[derive(Clone, Copy)]
pub struct Plic {
    base: usize,
}

impl Plic {
    /// Creates a handle for the PLIC at the standard base address.
    pub const fn new() -> Self {
        Self { base: PLIC_BASE }
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

    // --- public API ---------------------------------------------------------

    /// Sets the priority for a given IRQ (valid range: 0-7).
    ///
    /// - Priority 0 disables the interrupt.
    /// - Priority 1-7: higher values have higher priority.
    /// - IRQ 0 is reserved and should not be used.
    pub fn set_priority(&self, irq: u32, priority: u8) {
        let offset = reg::PRIORITY_BASE + (irq as usize) * 4;
        self.write_u32(offset, priority as u32);
    }

    /// Enables the specified IRQ for the given context.
    ///
    /// Context mapping:
    /// - Context 0: Hart 0 M-mode
    /// - Context 1: Hart 0 S-mode
    /// - Context 2: Hart 1 M-mode (multi-core)
    /// - etc.
    pub fn enable_irq(&self, context: usize, irq: u32) {
        let word_idx = irq / 32;
        let bit_idx = irq % 32;
        let offset = reg::ENABLE_BASE + context * 0x80 + (word_idx as usize) * 4;

        let mut val = self.read_u32(offset);
        val |= 1 << bit_idx;
        self.write_u32(offset, val);
    }

    /// Disables the specified IRQ for the given context.
    #[allow(dead_code)]
    pub fn disable_irq(&self, context: usize, irq: u32) {
        let word_idx = irq / 32;
        let bit_idx = irq % 32;
        let offset = reg::ENABLE_BASE + context * 0x80 + (word_idx as usize) * 4;

        let mut val = self.read_u32(offset);
        val &= !(1 << bit_idx);
        self.write_u32(offset, val);
    }

    /// Sets the priority threshold for the given context.
    ///
    /// Only interrupts with priority > threshold will be delivered.
    /// - Threshold 0: accept all interrupts (1-7).
    /// - Threshold 7: block all interrupts.
    pub fn set_threshold(&self, context: usize, threshold: u8) {
        let offset = reg::CONTEXT_BASE + context * CONTEXT_STRIDE;
        self.write_u32(offset, threshold as u32);
    }

    /// Claims the highest-priority pending interrupt for the given context.
    ///
    /// Returns the IRQ number (1-127), or 0 if no interrupt is pending.
    /// The caller MUST call `complete()` with the returned IRQ after handling it.
    pub fn claim(&self, context: usize) -> u32 {
        let offset = reg::CONTEXT_BASE + context * CONTEXT_STRIDE + 4;
        self.read_u32(offset)
    }

    /// Signals that interrupt handling is complete for the given IRQ.
    ///
    /// This must be called after `claim()` to allow the PLIC to deliver
    /// subsequent interrupts. The IRQ parameter must match the value
    /// returned by `claim()`.
    pub fn complete(&self, context: usize, irq: u32) {
        let offset = reg::CONTEXT_BASE + context * CONTEXT_STRIDE + 4;
        self.write_u32(offset, irq);
    }

    /// Reads the pending status for a given IRQ.
    ///
    /// Returns true if the interrupt is currently pending.
    #[allow(dead_code)]
    pub fn is_pending(&self, irq: u32) -> bool {
        let word_idx = irq / 32;
        let bit_idx = irq % 32;
        let offset = reg::PENDING_BASE + (word_idx as usize) * 4;

        let val = self.read_u32(offset);
        (val & (1 << bit_idx)) != 0
    }
}