//! UART I/O adapter for Shell with interrupt-driven input buffering
//!
//! This module provides a ShellIO implementation that wraps the existing
//! UART driver with an interrupt-driven circular buffer for input.

use crate::uart::{Uart, UART0_BASE};
use super::shell::ShellIO;
use spin::Mutex;

/// Size of the interrupt-driven input buffer
const INPUT_BUFFER_SIZE: usize = 128;

/// Circular buffer for interrupt-driven UART input
///
/// This buffer is written by the UART interrupt handler and read by the shell.
/// Using a Mutex ensures safe concurrent access.
struct InputBuffer {
    data: [u8; INPUT_BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
    count: usize,
}

impl InputBuffer {
    const fn new() -> Self {
        Self {
            data: [0; INPUT_BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            count: 0,
        }
    }

    /// Push a byte into the buffer (called from interrupt)
    ///
    /// Returns false if buffer is full.
    fn push(&mut self, byte: u8) -> bool {
        if self.count >= INPUT_BUFFER_SIZE {
            return false; // Buffer full
        }

        self.data[self.write_pos] = byte;
        self.write_pos = (self.write_pos + 1) % INPUT_BUFFER_SIZE;
        self.count += 1;
        true
    }

    /// Pop a byte from the buffer (called from shell)
    ///
    /// Returns None if buffer is empty.
    fn pop(&mut self) -> Option<u8> {
        if self.count == 0 {
            return None;
        }

        let byte = self.data[self.read_pos];
        self.read_pos = (self.read_pos + 1) % INPUT_BUFFER_SIZE;
        self.count -= 1;
        Some(byte)
    }

    /// Check if buffer has data
    #[allow(dead_code)]
    fn has_data(&self) -> bool {
        self.count > 0
    }
}

/// Global input buffer for UART0
///
/// This is accessed by both the interrupt handler and the shell.
static INPUT_BUF: Mutex<InputBuffer> = Mutex::new(InputBuffer::new());

/// Push a byte into the UART input buffer
///
/// This function should be called from the UART interrupt handler.
/// Returns false if the buffer is full.
///
/// # Safety
/// This function is interrupt-safe due to Mutex protection.
pub fn push_input_byte(byte: u8) -> bool {
    INPUT_BUF.lock().push(byte)
}

/// Check if there is buffered input available
#[allow(dead_code)]
pub fn has_input() -> bool {
    INPUT_BUF.lock().has_data()
}

/// UART-based I/O for the shell
///
/// This wrapper provides both interrupt-driven input (via global buffer)
/// and direct output through the UART driver.
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
    #[allow(dead_code)]
    pub fn init(&self) {
        self.uart.init();
    }
}

impl ShellIO for UartIO {
    /// Read a byte from the interrupt buffer (non-blocking)
    ///
    /// This reads from the interrupt-driven input buffer, not directly
    /// from UART hardware.
    fn read_byte(&mut self) -> Option<u8> {
        INPUT_BUF.lock().pop()
    }

    fn write_byte(&mut self, byte: u8) {
        self.uart.putc(byte);
    }

    fn write_str(&mut self, s: &str) {
        self.uart.puts(s);
    }
}