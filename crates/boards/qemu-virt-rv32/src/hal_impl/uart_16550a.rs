//! UART 16550A driver for the QEMU `virt` machine.
//!
//! UART0 is memory-mapped at 0x1000_0000.
//! All register accesses use `read_volatile` / `write_volatile` to prevent
//! the compiler from caching or reordering MMIO reads and writes.

use hal::SerialPort;

// ---------------------------------------------------------------------------
// Hardware constants
// ---------------------------------------------------------------------------

/// Register offsets relative to the UART base address (byte-addressable).
mod reg {
    /// Transmit Holding Register (W) / Receive Buffer Register (R).
    pub const THR: usize = 0;
    pub const RBR: usize = 0;
    /// Interrupt Enable Register.
    pub const IER: usize = 1;
    /// Interrupt Identification Register (R) / FIFO Control Register (W).
    pub const FCR: usize = 2;
    /// Line Control Register.
    pub const LCR: usize = 3;
    /// Modem Control Register.
    pub const MCR: usize = 4;
    /// Line Status Register (R).
    pub const LSR: usize = 5;
    /// Divisor Latch LSB (when DLAB=1).
    pub const DLL: usize = 0;
    /// Divisor Latch MSB (when DLAB=1).
    pub const DLM: usize = 1;
}

/// IER bit 0 — Received Data Available Interrupt Enable.
const IER_RDA: u8 = 1 << 0;
/// LSR bit 0 — Data Ready (RX has data).
const LSR_DATA_READY: u8 = 1 << 0;
/// LSR bit 5 — Transmitter Holding Register Empty (TX ready).
const LSR_TX_IDLE: u8 = 1 << 5;
/// LCR bit 7 — Divisor Latch Access Bit (enables baud-rate registers).
const LCR_DLAB: u8 = 1 << 7;

// ---------------------------------------------------------------------------
// Uart16550a type
// ---------------------------------------------------------------------------

/// A handle to a memory-mapped 16550A UART peripheral.
pub struct Uart16550a {
    base: usize,
}

impl Uart16550a {
    /// Creates a handle for the UART mapped at `base`.
    #[allow(dead_code)]
    pub const fn new(base: usize) -> Self {
        Self { base }
    }

    // --- register access ----------------------------------------------------

    #[inline]
    fn read(&self, offset: usize) -> u8 {
        unsafe { core::ptr::read_volatile((self.base + offset) as *const u8) }
    }

    #[inline]
    fn write(&self, offset: usize, val: u8) {
        unsafe { core::ptr::write_volatile((self.base + offset) as *mut u8, val) }
    }
}

impl SerialPort for Uart16550a {
    /// Initializes the UART to 38400 8N1 with TX/RX FIFOs enabled.
    fn init(&self) {
        self.write(reg::IER, 0x00); // disable all interrupts

        self.write(reg::LCR, LCR_DLAB); // enable divisor latch (bit 7=1)
                                        // 38400 bps: divisor = 1_843_200 / (16 × 38400) = 3
        self.write(reg::DLL, 0x03); // DLL (low byte of divisor)
        self.write(reg::DLM, 0x00); // DLM (high byte of divisor)

        // LCR = 0x03: 8 data bits (bits 1-0=11), no parity (bit 3=0), 1 stop bit (bit 2=0), DLAB cleared (bit 7=0)
        self.write(reg::LCR, 0x03);
        // FCR = 0xC7: enable FIFO (bit 0=1), clear RX FIFO (bit 1=1), clear TX FIFO (bit 2=1), trigger level 14 bytes (bits 7-6=11)
        self.write(reg::FCR, 0xC7);
        // MCR = 0x03: assert DTR (bit 0=1) + RTS (bit 1=1)
        self.write(reg::MCR, 0x03);
    }

    /// Transmits one byte, blocking until the TX holding register is empty.
    fn putc(&self, byte: u8) {
        while self.read(reg::LSR) & LSR_TX_IDLE == 0 {}
        self.write(reg::THR, byte);
    }

    /// Tries to read one byte from the RX FIFO without blocking.
    fn try_getc(&self) -> Option<u8> {
        if self.read(reg::LSR) & LSR_DATA_READY != 0 {
            Some(self.read(reg::RBR))
        } else {
            None
        }
    }

    /// Enables RX data available interrupt.
    fn enable_rx_interrupt(&self) {
        let mut ier = self.read(reg::IER);
        ier |= IER_RDA;
        self.write(reg::IER, ier);
    }

    /// Disables RX data available interrupt.
    fn disable_rx_interrupt(&self) {
        let mut ier = self.read(reg::IER);
        ier &= !IER_RDA;
        self.write(reg::IER, ier);
    }
}

// Implement Send + Sync as required by the SerialPort trait.
// Safe because:
// - All hardware access uses volatile operations
// - The UART peripheral is a singleton at a fixed MMIO address
// - Multiple cores/contexts can safely access the same UART registers
unsafe impl Send for Uart16550a {}
unsafe impl Sync for Uart16550a {}
