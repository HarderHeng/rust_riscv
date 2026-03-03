//! Built-in shell commands
//!
//! This module provides basic platform-independent commands for the shell.
//! Platform-specific commands (meminfo, reboot) should be implemented
//! by board-specific code.

use crate::shell::shell::{Command, CommandHandler, ShellIO};

// ---------------------------------------------------------------------------
// help - Display available commands or help for a specific command
// ---------------------------------------------------------------------------

struct HelpCommand;

impl CommandHandler for HelpCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        if args.is_empty() {
            // List all commands
            io.write_str("Available commands:\r\n");
            io.write_str("\r\n");

            for cmd in COMMANDS.iter() {
                io.write_str("  ");
                io.write_str(cmd.name);

                // Padding for alignment
                let padding = if cmd.name.len() < 12 {
                    12 - cmd.name.len()
                } else {
                    1
                };
                for _ in 0..padding {
                    io.write_byte(b' ');
                }

                io.write_str(cmd.handler.help());
                io.write_str("\r\n");
            }

            io.write_str("\r\n");
            io.write_str("Type 'help <command>' for more information on a specific command.\r\n");
        } else {
            // Show help for specific command
            let cmd_name = args[0];
            let found = COMMANDS.iter().find(|cmd| cmd.name == cmd_name);

            if let Some(cmd) = found {
                io.write_str(cmd.name);
                io.write_str(": ");
                io.write_str(cmd.handler.help());
                io.write_str("\r\n");
            } else {
                io.write_str("Unknown command: ");
                io.write_str(cmd_name);
                io.write_str("\r\n");
            }
        }

        Ok(())
    }

    fn help(&self) -> &'static str {
        "Display available commands or help for a specific command"
    }
}

static HELP_CMD: HelpCommand = HelpCommand;

// ---------------------------------------------------------------------------
// echo - Echo arguments to output
// ---------------------------------------------------------------------------

struct EchoCommand;

impl CommandHandler for EchoCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                io.write_byte(b' ');
            }
            io.write_str(arg);
        }
        io.write_str("\r\n");
        Ok(())
    }

    fn help(&self) -> &'static str {
        "Echo arguments to output"
    }
}

static ECHO_CMD: EchoCommand = EchoCommand;

// ---------------------------------------------------------------------------
// clear - Clear the screen
// ---------------------------------------------------------------------------

struct ClearCommand;

impl CommandHandler for ClearCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        // VT100 escape sequences:
        // ESC[2J - Clear entire screen
        // ESC[H - Move cursor to home position (1,1)
        io.write_str("\x1b[2J\x1b[H");
        Ok(())
    }

    fn help(&self) -> &'static str {
        "Clear the screen"
    }
}

static CLEAR_CMD: ClearCommand = ClearCommand;

// ---------------------------------------------------------------------------
// version - Display kernel version
// ---------------------------------------------------------------------------

struct VersionCommand;

impl CommandHandler for VersionCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        io.write_str("RISC-V Bare-Metal Kernel v0.1.0\r\n");
        io.write_str("Target: riscv32imac-unknown-none-elf\r\n");
        io.write_str("Build: ");
        io.write_str(env!("CARGO_PKG_VERSION"));
        io.write_str("\r\n");
        Ok(())
    }

    fn help(&self) -> &'static str {
        "Display kernel version information"
    }
}

static VERSION_CMD: VersionCommand = VersionCommand;

// ---------------------------------------------------------------------------
// uptime - Display system uptime (placeholder)
// ---------------------------------------------------------------------------

struct UptimeCommand;

impl CommandHandler for UptimeCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        io.write_str("Uptime: (timer not implemented yet)\r\n");
        Ok(())
    }

    fn help(&self) -> &'static str {
        "Display system uptime"
    }
}

static UPTIME_CMD: UptimeCommand = UptimeCommand;

// ---------------------------------------------------------------------------
// panic - Trigger a kernel panic (for testing)
// ---------------------------------------------------------------------------

struct PanicCommand;

impl CommandHandler for PanicCommand {
    fn execute(&self, _args: &[&str], _io: &mut dyn ShellIO) -> Result<(), &'static str> {
        panic!("User-requested panic from shell");
    }

    fn help(&self) -> &'static str {
        "Trigger a kernel panic (for testing)"
    }
}

static PANIC_CMD: PanicCommand = PanicCommand;

// ---------------------------------------------------------------------------
// Command Registry
// ---------------------------------------------------------------------------

/// Basic platform-independent shell commands, sorted alphabetically by name.
///
/// This must be kept sorted for binary search to work correctly.
/// Board-specific implementations should extend this list with additional
/// platform-specific commands (e.g., meminfo, reboot).
pub static COMMANDS: &[Command] = &[
    Command {
        name: "clear",
        handler: &CLEAR_CMD,
    },
    Command {
        name: "echo",
        handler: &ECHO_CMD,
    },
    Command {
        name: "help",
        handler: &HELP_CMD,
    },
    Command {
        name: "panic",
        handler: &PANIC_CMD,
    },
    Command {
        name: "uptime",
        handler: &UPTIME_CMD,
    },
    Command {
        name: "version",
        handler: &VERSION_CMD,
    },
];

// Compile-time check that commands are sorted
const _: () = {
    let mut i = 1;
    while i < COMMANDS.len() {
        let prev = COMMANDS[i - 1].name.as_bytes();
        let curr = COMMANDS[i].name.as_bytes();

        let mut j = 0;
        while j < prev.len() && j < curr.len() {
            assert!(prev[j] <= curr[j], "Commands must be sorted alphabetically");
            if prev[j] < curr[j] {
                break;
            }
            j += 1;
        }
        i += 1;
    }
};
