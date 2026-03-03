//! UART I/O adapter for Shell
//!
//! This module provides a ShellIO implementation that wraps the existing
//! UART driver, adding line editing and special character handling.

use crate::uart::{Uart, UART0_BASE};
use super::shell::ShellIO;

/// UART-based I/O for the shell
///
/// This wrapper provides line editing features like backspace handling
/// and character echo on top of the basic UART driver.
pub struct UartIO {
    uart: Uart,
}

impl UartIO {
    /// Create a new UartIO instance
    ///
    /// # Arguments
    /// * `base` - UART base address (typically UART0_BASE)
    pub fn new(base: usize) -> Self {
        let uart = Uart::new(base);
        Self { uart }
    }

    /// Create a UartIO instance for UART0 (convenience method)
    pub fn uart0() -> Self {
        Self::new(UART0_BASE)
    }

    /// Initialize the UART hardware
    ///
    /// This must be called before using the UartIO for shell I/O.
    pub fn init(&self) {
        self.uart.init();
    }
}

impl ShellIO for UartIO {
    fn read_byte(&mut self) -> Option<u8> {
        self.uart.try_getc()
    }

    fn write_byte(&mut self, byte: u8) {
        self.uart.putc(byte);
    }

    fn write_str(&mut self, s: &str) {
        self.uart.puts(s);
    }
}