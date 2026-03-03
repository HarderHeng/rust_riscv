//! Core Shell implementation for bare-metal RISC-V kernel
//!
//! This module provides a modular, extensible shell system with:
//! - I/O abstraction (ShellIO trait)
//! - Command registration and dispatch
//! - Command parsing
//! - Line editing support

use core::fmt;
use heapless::{Vec, String};

/// Maximum length of input buffer
const INPUT_BUFFER_SIZE: usize = 256;

/// Maximum number of command arguments
const MAX_ARGS: usize = 16;

/// I/O abstraction trait for shell input/output
///
/// This trait allows the shell to work with different I/O backends
/// (polling, interrupt-driven, etc.) without modification.
pub trait ShellIO {
    /// Read a single byte (non-blocking)
    ///
    /// Returns `Some(byte)` if data is available, `None` otherwise.
    fn read_byte(&mut self) -> Option<u8>;

    /// Write a single byte
    fn write_byte(&mut self, byte: u8);

    /// Write a string slice
    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// Write formatted output (implements core::fmt::Write)
    fn write_fmt(&mut self, args: fmt::Arguments) {
        struct Writer<'a, IO: ShellIO + ?Sized>(&'a mut IO);

        impl<IO: ShellIO + ?Sized> fmt::Write for Writer<'_, IO> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.0.write_str(s);
                Ok(())
            }
        }

        let _ = fmt::Write::write_fmt(&mut Writer(self), args);
    }
}

/// Command handler trait
///
/// All shell commands must implement this trait.
pub trait CommandHandler: Sync {
    /// Execute the command with given arguments
    ///
    /// # Arguments
    /// * `args` - Command arguments (not including command name)
    /// * `io` - Shell I/O for output
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(msg)` on error with error message
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str>;

    /// Get help text for this command
    fn help(&self) -> &'static str;
}

/// Command registration structure
///
/// Each command is represented by this struct in the command registry.
#[derive(Copy, Clone)]
pub struct Command {
    /// Command name (must be lowercase)
    pub name: &'static str,

    /// Command handler implementation
    pub handler: &'static dyn CommandHandler,
}

/// Shell state and main loop
///
/// This struct maintains the shell state and provides the main
/// read-parse-execute-print loop.
pub struct Shell<'a, IO: ShellIO> {
    /// I/O backend
    io: IO,

    /// Input buffer
    input_buffer: [u8; INPUT_BUFFER_SIZE],

    /// Current input length
    input_len: usize,

    /// Command registry (sorted by name for binary search)
    commands: &'a [Command],

    /// Shell prompt string
    prompt: &'static str,
}

impl<'a, IO: ShellIO> Shell<'a, IO> {
    /// Create a new shell instance
    ///
    /// # Arguments
    /// * `io` - I/O backend implementation
    /// * `commands` - Command registry (must be sorted by name)
    /// * `prompt` - Shell prompt string
    pub fn new(io: IO, commands: &'a [Command], prompt: &'static str) -> Self {
        Self {
            io,
            input_buffer: [0; INPUT_BUFFER_SIZE],
            input_len: 0,
            commands,
            prompt,
        }
    }

    /// Display the shell prompt
    fn show_prompt(&mut self) {
        self.io.write_str(self.prompt);
    }

    /// Clear current input line
    fn clear_input(&mut self) {
        self.input_len = 0;
        self.input_buffer.fill(0);
    }

    /// Process a single input character
    ///
    /// Returns `true` if a complete line is ready to be processed.
    fn process_char(&mut self, ch: u8) -> bool {
        match ch {
            // Backspace or DEL
            0x08 | 0x7F => {
                if self.input_len > 0 {
                    self.input_len -= 1;
                    // Send backspace sequence: \b \b (move back, space, move back)
                    self.io.write_str("\x08 \x08");
                }
                false
            }

            // Carriage return or newline
            b'\r' | b'\n' => {
                self.io.write_str("\r\n");
                true
            }

            // Ctrl+C (ETX)
            0x03 => {
                self.io.write_str("^C\r\n");
                self.clear_input();
                self.show_prompt();
                false
            }

            // Ctrl+L (form feed) - clear screen
            0x0C => {
                self.io.write_str("\x1b[2J\x1b[H"); // VT100: clear screen and home
                self.show_prompt();
                // Re-display current input
                if self.input_len > 0 {
                    let input = core::str::from_utf8(&self.input_buffer[..self.input_len])
                        .unwrap_or("");
                    self.io.write_str(input);
                }
                false
            }

            // Printable ASCII characters
            0x20..=0x7E => {
                if self.input_len < INPUT_BUFFER_SIZE {
                    self.input_buffer[self.input_len] = ch;
                    self.input_len += 1;
                    // Echo character
                    self.io.write_byte(ch);
                }
                false
            }

            // Ignore other control characters
            _ => false,
        }
    }

    /// Parse command line into command and arguments
    ///
    /// Returns (command_name, arguments) or None if line is empty.
    /// The strings are copied to avoid borrow checker issues.
    fn parse_command_line(&self) -> Option<(String<32>, Vec<String<64>, MAX_ARGS>)> {
        if self.input_len == 0 {
            return None;
        }

        // Convert input buffer to string
        let input = core::str::from_utf8(&self.input_buffer[..self.input_len])
            .ok()?;

        // Split by whitespace and collect into Vec
        let mut tokens: Vec<String<64>, MAX_ARGS> = Vec::new();
        for token in input.split_whitespace() {
            if let Ok(s) = String::try_from(token) {
                if tokens.push(s).is_err() {
                    // Too many arguments
                    break;
                }
            }
        }

        if tokens.is_empty() {
            return None;
        }

        let cmd: String<32> = tokens[0].as_str().try_into().ok()?;
        let args = tokens[1..].iter().cloned().collect::<Vec<_, MAX_ARGS>>();

        Some((cmd, args))
    }

    /// Lookup command by name
    ///
    /// Uses binary search on sorted command registry.
    fn find_command(&self, name: &str) -> Option<&Command> {
        self.commands
            .binary_search_by(|cmd| cmd.name.cmp(name))
            .ok()
            .map(|idx| &self.commands[idx])
    }

    /// Execute a command
    fn execute_command(&mut self, cmd_name: &str, args: &[String<64>]) {
        match self.find_command(cmd_name) {
            Some(cmd) => {
                // Convert String<64> to &str for the handler
                let arg_refs: Vec<&str, MAX_ARGS> = args.iter()
                    .map(|s| s.as_str())
                    .collect();

                match cmd.handler.execute(arg_refs.as_slice(), &mut self.io) {
                    Ok(()) => {}
                    Err(msg) => {
                        self.io.write_str("Error: ");
                        self.io.write_str(msg);
                        self.io.write_str("\r\n");
                    }
                }
            }
            None => {
                self.io.write_str("Command not found: ");
                self.io.write_str(cmd_name);
                self.io.write_str("\r\n");
                self.io.write_str("Type 'help' for available commands.\r\n");
            }
        }
    }

    /// Run one iteration of the shell loop
    ///
    /// This should be called repeatedly from the main kernel loop.
    /// Returns `true` if a command was processed.
    pub fn poll(&mut self) -> bool {
        // Read available input
        while let Some(ch) = self.io.read_byte() {
            if self.process_char(ch) {
                // Complete line received - parse and execute
                let parse_result = self.parse_command_line();
                if let Some((cmd, args)) = parse_result {
                    self.execute_command(cmd.as_str(), args.as_slice());
                }
                self.clear_input();
                self.show_prompt();
                return true;
            }
        }
        false
    }

    /// Start the shell (display welcome message and prompt)
    pub fn start(&mut self) {
        self.io.write_str("\r\n");
        self.io.write_str("=================================\r\n");
        self.io.write_str("  RISC-V Bare-Metal Kernel Shell\r\n");
        self.io.write_str("=================================\r\n");
        self.io.write_str("Type 'help' for available commands.\r\n");
        self.io.write_str("\r\n");
        self.show_prompt();
    }

    /// Run the shell in a blocking loop
    ///
    /// This is a convenience method that runs the shell until
    /// a fatal error occurs. For integration with other kernel
    /// tasks, use `poll()` instead.
    pub fn run(&mut self) -> ! {
        self.start();

        loop {
            self.poll();

            // WFI to save power while waiting for input
            #[cfg(target_arch = "riscv32")]
            unsafe {
                core::arch::asm!("wfi");
            }
        }
    }
}

/// Helper function to create a sorted command registry
///
/// This should be called at compile-time to ensure commands are sorted.
/// Use like: `const COMMANDS: &[Command] = &sorted_commands![cmd1, cmd2, cmd3];`
#[macro_export]
macro_rules! sorted_commands {
    ($($cmd:expr),* $(,)?) => {{
        const fn is_sorted(commands: &[Command]) -> bool {
            let mut i = 1;
            while i < commands.len() {
                // Compare adjacent command names
                let prev = commands[i - 1].name.as_bytes();
                let curr = commands[i].name.as_bytes();

                let mut j = 0;
                while j < prev.len() && j < curr.len() {
                    if prev[j] > curr[j] {
                        return false;
                    }
                    if prev[j] < curr[j] {
                        break;
                    }
                    j += 1;
                }
                if j == curr.len() && prev.len() > curr.len() {
                    return false;
                }
                i += 1;
            }
            true
        }

        const COMMANDS: &[Command] = &[$($cmd),*];
        const _: () = assert!(is_sorted(COMMANDS), "Commands must be sorted by name");
        COMMANDS
    }};
}