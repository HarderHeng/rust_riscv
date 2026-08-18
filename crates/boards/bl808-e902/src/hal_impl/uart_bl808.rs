//! BL808 UART driver (placeholder).

use hal::SerialPort;

/// A handle to the BL808 UART peripheral.
///
/// Placeholder implementation awaiting the BL808 memory map and register
/// definitions.
pub struct Bl808Uart {
    // Reserved for the future driver implementation.
    #[allow(dead_code)]
    base_addr: usize,
}

impl Bl808Uart {
    /// Creates a UART handle at `base_addr`.
    pub const fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }
}

impl SerialPort for Bl808Uart {
    /// Placeholder initialization.
    fn init(&self) {
        // TODO: Initialize BL808 UART peripheral
        // Need to determine:
        // - Register layout (compatible with 16550? Custom?)
        // - Clock configuration
        // - Pin muxing requirements
        todo!("BL808 UART initialization not yet implemented")
    }

    /// Placeholder byte write.
    fn putc(&self, _byte: u8) {
        // TODO: Implement byte write
        // Need UART register offsets and protocol
        todo!("BL808 UART write not yet implemented")
    }

    /// Placeholder byte read.
    fn try_getc(&self) -> Option<u8> {
        // TODO: Implement byte read
        // Need UART status register and data register layout
        todo!("BL808 UART read not yet implemented")
    }

    /// Placeholder RX interrupt enable.
    fn enable_rx_interrupt(&self) {
        todo!("BL808 UART RX interrupt enable not yet implemented")
    }

    /// Placeholder RX interrupt disable.
    fn disable_rx_interrupt(&self) {
        todo!("BL808 UART RX interrupt disable not yet implemented")
    }
}
