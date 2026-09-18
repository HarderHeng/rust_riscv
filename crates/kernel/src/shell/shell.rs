//! Core Shell implementation for bare-metal RISC-V kernel
//!
//! This module provides a modular, extensible shell system with:
//! - I/O abstraction (ShellIO trait)
//! - Command registration and dispatch
//! - Command parsing
//! - Line editing support

use core::fmt;
use heapless::Vec;

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
    #[allow(dead_code)]
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

    /// Whether the next line feed terminates a preceding carriage return.
    skip_lf_after_cr: bool,

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
            skip_lf_after_cr: false,
            commands,
            prompt,
        }
    }

    /// Display the shell prompt
    fn show_prompt(&mut self) {
        self.io.write_str(self.prompt);
    }

    /// Clear current input line.
    ///
    /// Only resets `input_len` — do not `fill(0)` the whole buffer. Leaves
    /// `skip_lf_after_cr` alone so a CR-terminated line still consumes the
    /// following LF; Ctrl+C clears that flag explicitly.
    fn clear_input(&mut self) {
        self.input_len = 0;
    }

    /// Process a single input character
    ///
    /// Returns `true` if a complete line is ready to be processed.
    fn process_char(&mut self, ch: u8) -> bool {
        if ch == b'\n' && self.skip_lf_after_cr {
            self.skip_lf_after_cr = false;
            return false;
        }

        self.skip_lf_after_cr = ch == b'\r';

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
                    let input =
                        core::str::from_utf8(&self.input_buffer[..self.input_len]).unwrap_or("");
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

    /// Parse the current input line and dispatch the command.
    ///
    /// Tokenizes to `&str` slices that borrow `input_buffer` directly (no
    /// `heapless::String` copies). Field destructuring keeps the buffer
    /// borrow disjoint from `&mut io` / the command table.
    fn dispatch_line(&mut self) {
        if self.input_len == 0 {
            return;
        }

        let Shell {
            io,
            input_buffer,
            input_len,
            commands,
            ..
        } = self;

        let Ok(input) = core::str::from_utf8(&input_buffer[..*input_len]) else {
            return;
        };

        let mut tokens: Vec<&str, MAX_ARGS> = Vec::new();
        for token in input.split_whitespace() {
            if tokens.push(token).is_err() {
                // Too many arguments — drop the rest
                break;
            }
        }
        if tokens.is_empty() {
            return;
        }

        let cmd_name = tokens[0];
        let args = &tokens[1..];

        match commands.binary_search_by(|cmd| cmd.name.cmp(cmd_name)) {
            Ok(idx) => {
                let handler = commands[idx].handler;
                match handler.execute(args, io) {
                    Ok(()) => {}
                    Err(msg) => {
                        io.write_str("Error: ");
                        io.write_str(msg);
                        io.write_str("\r\n");
                    }
                }
            }
            Err(_) => {
                io.write_str("Command not found: ");
                io.write_str(cmd_name);
                io.write_str("\r\n");
                io.write_str("Type 'help' for available commands.\r\n");
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
                self.dispatch_line();
                self.clear_input();
                self.show_prompt();
                return true;
            }
        }
        false
    }

    /// Start the shell prompt (boot banner is printed once by `kernel_main`).
    pub fn start(&mut self) {
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
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;

    struct MockIo {
        input: Vec<u8, 512>,
        input_index: usize,
        output: Vec<u8, 512>,
    }

    impl MockIo {
        fn with_input(input: &[u8]) -> Self {
            let mut mock = Self {
                input: Vec::new(),
                input_index: 0,
                output: Vec::new(),
            };
            mock.input.extend_from_slice(input).unwrap();
            mock
        }

        fn output(&self) -> &str {
            core::str::from_utf8(self.output.as_slice()).unwrap()
        }
    }

    impl ShellIO for MockIo {
        fn read_byte(&mut self) -> Option<u8> {
            let byte = self.input.get(self.input_index).copied();
            if byte.is_some() {
                self.input_index += 1;
            }
            byte
        }

        fn write_byte(&mut self, byte: u8) {
            self.output.push(byte).unwrap();
        }
    }

    struct TestEchoCommand;

    impl CommandHandler for TestEchoCommand {
        fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
            io.write_str("handled: ");
            for (index, arg) in args.iter().enumerate() {
                if index != 0 {
                    io.write_byte(b' ');
                }
                io.write_str(arg);
            }
            io.write_str("\r\n");
            Ok(())
        }

        fn help(&self) -> &'static str {
            "Test command"
        }
    }

    static TEST_ECHO_COMMAND: TestEchoCommand = TestEchoCommand;
    static TEST_COMMANDS: &[Command] = &[Command {
        name: "echo",
        handler: &TEST_ECHO_COMMAND,
    }];

    fn shell_with_input(input: &[u8]) -> Shell<'static, MockIo> {
        Shell::new(MockIo::with_input(input), TEST_COMMANDS, "test> ")
    }

    #[test]
    fn dispatches_a_command_and_reprints_the_prompt() {
        let mut shell = shell_with_input(b"echo hello world\r");

        assert!(shell.poll());
        assert_eq!(
            shell.io.output(),
            "echo hello world\r\nhandled: hello world\r\ntest> "
        );
    }

    #[test]
    fn backspace_edits_the_input_before_dispatch() {
        let mut shell = shell_with_input(b"echo hellp\x08o\r");

        assert!(shell.poll());
        assert!(shell.io.output().contains("\x08 \x08"));
        assert!(shell.io.output().contains("handled: hello\r\n"));
    }

    #[test]
    fn control_c_discards_the_current_line() {
        let mut shell = shell_with_input(b"echo ignored\x03echo ok\r");

        assert!(shell.poll());
        assert!(shell.io.output().contains("^C\r\ntest> "));
        assert!(shell.io.output().contains("handled: ok\r\n"));
        assert!(!shell.io.output().contains("handled: ignored\r\n"));
    }

    #[test]
    fn crlf_is_processed_as_one_line_terminator() {
        let mut shell = shell_with_input(b"echo ok\r\n");

        assert!(shell.poll());
        assert!(!shell.poll());
        assert_eq!(shell.io.output(), "echo ok\r\nhandled: ok\r\ntest> ");
    }

    #[test]
    fn unknown_commands_report_an_error() {
        let mut shell = shell_with_input(b"missing\r");

        assert!(shell.poll());
        assert!(shell.io.output().contains("Command not found: missing\r\n"));
    }
}
