use hal::SerialPort;

pub struct Bl808Uart {
    base_addr: usize,
}

impl Bl808Uart {
    pub const fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }
}

impl SerialPort for Bl808Uart {
    fn init(&mut self) {
        // TODO: Initialize BL808 UART peripheral
        // Need to determine:
        // - Register layout (compatible with 16550? Custom?)
        // - Clock configuration
        // - Pin muxing requirements
        todo!("BL808 UART initialization not yet implemented")
    }

    fn write_byte(&mut self, byte: u8) {
        // TODO: Implement byte write
        // Need UART register offsets and protocol
        todo!("BL808 UART write not yet implemented")
    }

    fn read_byte(&mut self) -> Option<u8> {
        // TODO: Implement byte read
        // Need UART status register and data register layout
        todo!("BL808 UART read not yet implemented")
    }

    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}
