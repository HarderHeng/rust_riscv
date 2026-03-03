//! UART 16550A driver for the QEMU `virt` machine.
//!
//! UART0 is memory-mapped at [`UART0_BASE`] (0x1000_0000).
//! All register accesses use `read_volatile` / `write_volatile` to prevent
//! the compiler from caching or reordering MMIO reads and writes.
//!
//! # Usage
//!
//! ```rust
//! let uart = Uart::new(UART0_BASE);
//! uart.init();
//! uart.puts("hello\r\n");
//!
//! // Or via fmt::Write:
//! use core::fmt::Write;
//! write!(uart, "x = {}\r\n", 42).ok();
//! ```
//!
//! The free function [`print`] is the backend for the `kprint!` / `kprintln!`
//! macros defined in `main.rs`.

use core::fmt;

// ---------------------------------------------------------------------------
// Hardware constants
// ---------------------------------------------------------------------------

pub const UART0_BASE: usize = 0x1000_0000;

/// Register offsets relative to the UART base address (byte-addressable).
mod reg {
    /// Transmit Holding Register (W) / Receive Buffer Register (R).
    pub const THR: usize = 0;
    pub const RBR: usize = 0;
    /// Interrupt Enable Register.
    pub const IER: usize = 1;
    /// Interrupt Identification Register (R) / FIFO Control Register (W).
    #[allow(dead_code)]
    pub const IIR: usize = 2;
    pub const FCR: usize = 2;
    /// Line Control Register.
    pub const LCR: usize = 3;
    /// Modem Control Register.
    pub const MCR: usize = 4;
    /// Line Status Register (R).
    pub const LSR: usize = 5;
    /// Modem Status Register (R).
    #[allow(dead_code)]
    pub const MSR: usize = 6;
    /// Scratch Register.
    #[allow(dead_code)]
    pub const SCR: usize = 7;
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
// Uart type
// ---------------------------------------------------------------------------

/// A handle to a memory-mapped 16550A UART peripheral.
///
/// The struct is intentionally cheap to copy (just a `usize` base address)
/// so callers can create short-lived instances without overhead.
#[derive(Clone, Copy)]
pub struct Uart {
    base: usize,
}

impl Uart {
    /// Creates a handle for the UART mapped at `base`.
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

    // --- public API ---------------------------------------------------------

    /// Initialises the UART to 38400 8N1 with TX/RX FIFOs enabled.
    ///
    /// On QEMU the emulator pre-configures the device, so this call has no
    /// observable effect there. On real 16550A hardware it is required.
    pub fn init(&self) {
        self.write(reg::IER, 0x00);     // disable all interrupts

        self.write(reg::LCR, LCR_DLAB); // enable divisor latch (bit 7=1)
        // 38400 bps: divisor = 1_843_200 / (16 × 38400) = 3
        self.write(reg::DLL, 0x03);     // DLL (low byte of divisor)
        self.write(reg::DLM, 0x00);     // DLM (high byte of divisor)

        // LCR = 0x03: 8 data bits (bits 1-0=11), no parity (bit 3=0), 1 stop bit (bit 2=0), DLAB cleared (bit 7=0)
        self.write(reg::LCR, 0x03);
        // FCR = 0xC7: enable FIFO (bit 0=1), clear RX FIFO (bit 1=1), clear TX FIFO (bit 2=1), trigger level 14 bytes (bits 7-6=11)
        self.write(reg::FCR, 0xC7);
        // MCR = 0x03: assert DTR (bit 0=1) + RTS (bit 1=1)
        self.write(reg::MCR, 0x03);
    }

    /// Enables RX data available interrupt.
    ///
    /// When data arrives in the RX FIFO, the UART will assert its interrupt line
    /// (IRQ 10 on QEMU virt), which routes through PLIC to the CPU.
    pub fn enable_rx_interrupt(&self) {
        let mut ier = self.read(reg::IER);
        ier |= IER_RDA;
        self.write(reg::IER, ier);
    }

    /// Disables RX data available interrupt.
    #[allow(dead_code)]
    pub fn disable_rx_interrupt(&self) {
        let mut ier = self.read(reg::IER);
        ier &= !IER_RDA;
        self.write(reg::IER, ier);
    }

    /// Transmits one byte, blocking until the TX holding register is empty.
    pub fn putc(&self, byte: u8) {
        while self.read(reg::LSR) & LSR_TX_IDLE == 0 {}
        self.write(reg::THR, byte);
    }

    /// Tries to read one byte from the RX FIFO without blocking.
    ///
    /// Returns `Some(byte)` if data is available, or `None` if the RX FIFO is empty.
    pub fn try_getc(&self) -> Option<u8> {
        if self.read(reg::LSR) & LSR_DATA_READY != 0 {
            Some(self.read(reg::RBR))
        } else {
            None
        }
    }

    /// Reads one byte from the RX FIFO, blocking until data is available.
    #[allow(dead_code)]
    pub fn getc(&self) -> u8 {
        while self.read(reg::LSR) & LSR_DATA_READY == 0 {}
        self.read(reg::RBR)
    }

    /// Transmits every byte of `s`.
    pub fn puts(&self, s: &str) {
        for b in s.bytes() {
            self.putc(b);
        }
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.puts(s);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// kprint! / kprintln! backend
// ---------------------------------------------------------------------------

/// Formats `args` and writes the result to UART0.
///
/// This is the low-level backend called by the `kprint!` and `kprintln!`
/// macros.  A fresh [`Uart`] handle is created on the stack for each call —
/// it is just a base-address wrapper, so there is no runtime overhead.
pub fn print(args: fmt::Arguments) {
    use fmt::Write;
    Uart::new(UART0_BASE).write_fmt(args).ok();
}
