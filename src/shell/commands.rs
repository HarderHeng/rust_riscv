//! Built-in shell commands
//!
//! This module provides all built-in commands for the shell.

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
// reboot - Reboot the system
// ---------------------------------------------------------------------------

struct RebootCommand;

impl CommandHandler for RebootCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        io.write_str("Rebooting system...\r\n");

        // On QEMU virt machine, we can trigger a reboot via the test device
        // at address 0x100000. Writing 0x5555 to it causes QEMU to exit.
        // Note: This is a QEMU-specific feature, not standard RISC-V.

        const VIRT_TEST: usize = 0x100000;
        const VIRT_TEST_FINISHER_RESET: u32 = 0x7777;

        unsafe {
            core::ptr::write_volatile(VIRT_TEST as *mut u32, VIRT_TEST_FINISHER_RESET);
        }

        // If we get here, the reboot didn't work
        io.write_str("Reboot failed (not supported on this platform)\r\n");
        Ok(())
    }

    fn help(&self) -> &'static str {
        "Reboot the system"
    }
}

static REBOOT_CMD: RebootCommand = RebootCommand;

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
// meminfo - Display memory information
// ---------------------------------------------------------------------------

struct MeminfoCommand;

impl CommandHandler for MeminfoCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        use crate::startup;
        use core::fmt::Write;

        io.write_str("Memory Layout:\r\n");
        io.write_str("\r\n");

        // Get memory ranges from startup module
        let (bss_start, bss_end) = startup::bss_range();
        let (heap_start, heap_end) = startup::heap_range();

        // Text section (approximate - from linker script start)
        io.write_str("  .text   : 0x80000000 - (code section)\r\n");

        // BSS section
        let _ = write!(
            &mut Wrapper(io),
            "  .bss    : 0x{:08x} - 0x{:08x} ({} bytes)\r\n",
            bss_start as usize,
            bss_end as usize,
            (bss_end as usize) - (bss_start as usize)
        );

        // Heap
        let _ = write!(
            &mut Wrapper(io),
            "  heap    : 0x{:08x} - 0x{:08x} ({} bytes)\r\n",
            heap_start as usize,
            heap_end as usize,
            (heap_end as usize) - (heap_start as usize)
        );

        io.write_str("\r\n");
        io.write_str("Note: Heap allocator not initialized\r\n");

        Ok(())
    }

    fn help(&self) -> &'static str {
        "Display memory layout information"
    }
}

static MEMINFO_CMD: MeminfoCommand = MeminfoCommand;

// Wrapper to implement Write for ShellIO
struct Wrapper<'a>(&'a mut dyn ShellIO);

impl<'a> core::fmt::Write for Wrapper<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write_str(s);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Command Registry
// ---------------------------------------------------------------------------

/// All available shell commands, sorted alphabetically by name
///
/// This must be kept sorted for binary search to work correctly.
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
        name: "meminfo",
        handler: &MEMINFO_CMD,
    },
    Command {
        name: "panic",
        handler: &PANIC_CMD,
    },
    Command {
        name: "reboot",
        handler: &REBOOT_CMD,
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