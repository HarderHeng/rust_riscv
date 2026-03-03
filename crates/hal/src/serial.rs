//! Serial port trait for hardware abstraction.
//!
//! This module defines the `SerialPort` trait which abstracts UART/serial
//! communication hardware for bare-metal kernels.

/// Trait for serial port hardware (UART).
///
/// Implementations must be `Send + Sync` to allow safe usage in static contexts.
/// All methods operate on immutable references to support interrupt-driven I/O.
pub trait SerialPort: Send + Sync {
    /// Initialize the serial port hardware.
    ///
    /// This should configure baud rate, data bits, stop bits, and enable
    /// the transmitter. It may also enable the receiver if needed.
    fn init(&self);

    /// Write a single character to the serial port.
    ///
    /// This method blocks until the transmit buffer has space.
    ///
    /// # Arguments
    /// * `c` - The byte to transmit
    fn putc(&self, c: u8);

    /// Attempt to read a character from the serial port without blocking.
    ///
    /// # Returns
    /// * `Some(byte)` - A byte was available and read
    /// * `None` - No data available (receive buffer empty)
    fn try_getc(&self) -> Option<u8>;

    /// Write a string to the serial port.
    ///
    /// Default implementation writes character-by-character using `putc()`.
    ///
    /// # Arguments
    /// * `s` - The string to write
    fn puts(&self, s: &str) {
        for byte in s.bytes() {
            self.putc(byte);
        }
    }

    /// Enable receive (RX) interrupts.
    ///
    /// After calling this, the serial port should generate interrupts when
    /// data is received.
    fn enable_rx_interrupt(&self);

    /// Disable receive (RX) interrupts.
    ///
    /// After calling this, the serial port should not generate interrupts
    /// when data is received.
    fn disable_rx_interrupt(&self);
}
