//! QEMU virt-specific shell commands.

use core::ops::Range;

use hal::Platform;
use kernel::shell::{Command, CommandHandler, ShellIO};

use crate::PLATFORM;

fn write_decimal(io: &mut dyn ShellIO, mut value: usize) {
    if value == 0 {
        io.write_byte(b'0');
        return;
    }

    let mut digits = [0u8; 20];
    let mut len = 0;
    while value != 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    while len != 0 {
        len -= 1;
        io.write_byte(digits[len]);
    }
}

fn write_hex(io: &mut dyn ShellIO, mut value: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    io.write_str("0x");
    if value == 0 {
        io.write_byte(b'0');
        return;
    }

    let mut digits = [0u8; 16];
    let mut len = 0;
    while value != 0 {
        digits[len] = HEX[value & 0xf];
        value >>= 4;
        len += 1;
    }

    while len != 0 {
        len -= 1;
        io.write_byte(digits[len]);
    }
}

fn write_region(io: &mut dyn ShellIO, name: &str, region: &Range<usize>) {
    io.write_str("  ");
    io.write_str(name);
    io.write_str(": ");
    write_hex(io, region.start);
    io.write_str(" - ");
    write_hex(io, region.end);
    io.write_str(" (");
    write_decimal(io, region.end.saturating_sub(region.start));
    io.write_str(" bytes)\r\n");
}

struct HelpCommand;

impl CommandHandler for HelpCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        if args.is_empty() {
            io.write_str("Available commands:\r\n\r\n");
            for command in COMMANDS {
                io.write_str("  ");
                io.write_str(command.name);
                let padding = if command.name.len() < 12 {
                    12 - command.name.len()
                } else {
                    1
                };
                for _ in 0..padding {
                    io.write_byte(b' ');
                }
                io.write_str(command.handler.help());
                io.write_str("\r\n");
            }
            io.write_str(
                "\r\nType 'help <command>' for more information on a specific command.\r\n",
            );
        } else if let Some(command) = COMMANDS.iter().find(|command| command.name == args[0]) {
            io.write_str(command.name);
            io.write_str(": ");
            io.write_str(command.handler.help());
            io.write_str("\r\n");
        } else {
            io.write_str("Unknown command: ");
            io.write_str(args[0]);
            io.write_str("\r\n");
        }
        Ok(())
    }

    fn help(&self) -> &'static str {
        "Display available commands or help for a specific command"
    }
}

struct MeminfoCommand;

impl CommandHandler for MeminfoCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        let layout = PLATFORM.memory_layout();
        io.write_str("Memory layout:\r\n");
        write_region(io, "heap ", &layout.heap);
        write_region(io, "stack", &layout.stack);
        write_region(io, "text ", &layout.text);
        write_region(io, "data ", &layout.data);
        write_region(io, "bss  ", &layout.bss);
        Ok(())
    }

    fn help(&self) -> &'static str {
        "Display linker-defined memory regions"
    }
}

struct RebootCommand;

impl CommandHandler for RebootCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        io.write_str("Rebooting...\r\n");
        PLATFORM.reboot()
    }

    fn help(&self) -> &'static str {
        "Reboot the QEMU machine"
    }
}

static HELP_CMD: HelpCommand = HelpCommand;
static MEMINFO_CMD: MeminfoCommand = MeminfoCommand;
static REBOOT_CMD: RebootCommand = RebootCommand;

/// Complete QEMU shell command registry, kept alphabetically for binary search.
pub static COMMANDS: &[Command] = kernel::sorted_commands![
    Command {
        name: "clear",
        handler: &kernel::shell::commands::CLEAR_CMD,
    },
    Command {
        name: "echo",
        handler: &kernel::shell::commands::ECHO_CMD,
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
        handler: &kernel::shell::commands::PANIC_CMD,
    },
    Command {
        name: "reboot",
        handler: &REBOOT_CMD,
    },
    Command {
        name: "uptime",
        handler: &kernel::shell::commands::UPTIME_CMD,
    },
    Command {
        name: "version",
        handler: &kernel::shell::commands::VERSION_CMD,
    },
];
