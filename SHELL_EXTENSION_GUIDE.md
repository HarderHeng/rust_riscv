# Shell Extension Guide

## Quick Start: Adding a New Command

This guide walks through adding a new command to the shell in 5 minutes.

## Example: LED Control Command

Let's create a command to control an imaginary LED (can be adapted for real hardware).

### Step 1: Create the Command Handler

Open `/home/heng/test/rust_riscv/src/shell/commands.rs` and add:

```rust
// ---------------------------------------------------------------------------
// led - Control system LED (example command)
// ---------------------------------------------------------------------------

struct LedCommand;

impl CommandHandler for LedCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        if args.is_empty() {
            return Err("Usage: led <on|off|blink>");
        }

        match args[0] {
            "on" => {
                io.write_str("LED turned ON\r\n");
                // TODO: Write to GPIO register here
                // e.g., gpio::led_on();
                Ok(())
            }
            "off" => {
                io.write_str("LED turned OFF\r\n");
                // TODO: Write to GPIO register
                Ok(())
            }
            "blink" => {
                io.write_str("LED blinking...\r\n");
                // TODO: Start blink timer
                Ok(())
            }
            _ => Err("Invalid argument. Use: on, off, or blink"),
        }
    }

    fn help(&self) -> &'static str {
        "Control system LED (on|off|blink)"
    }
}

static LED_CMD: LedCommand = LedCommand;
```

### Step 2: Register in Command Table

In the same file, add to `COMMANDS` array (keep alphabetical order!):

```rust
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
    // Add your command here (alphabetically)
    Command {
        name: "led",
        handler: &LED_CMD,
    },
    Command {
        name: "meminfo",
        handler: &MEMINFO_CMD,
    },
    // ... rest of commands ...
];
```

### Step 3: Rebuild and Test

```bash
cargo run
```

In QEMU:
```
kernel> help
kernel> led on
LED turned ON
kernel> led invalid
Error: Invalid argument. Use: on, off, or blink
```

## Advanced Examples

### Command with Multiple Arguments

```rust
struct WriteCommand;

impl CommandHandler for WriteCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        if args.len() < 2 {
            return Err("Usage: write <address> <value>");
        }

        // Parse hex address
        let addr = if let Some(hex) = args[0].strip_prefix("0x") {
            usize::from_str_radix(hex, 16)
        } else {
            args[0].parse::<usize>()
        }.map_err(|_| "Invalid address")?;

        // Parse hex or decimal value
        let value = if let Some(hex) = args[1].strip_prefix("0x") {
            u32::from_str_radix(hex, 16)
        } else {
            args[1].parse::<u32>()
        }.map_err(|_| "Invalid value")?;

        // Write to memory (UNSAFE!)
        unsafe {
            core::ptr::write_volatile(addr as *mut u32, value);
        }

        use core::fmt::Write;
        let _ = write!(
            &mut Wrapper(io),
            "Wrote 0x{:08x} to 0x{:08x}\r\n",
            value, addr
        );

        Ok(())
    }

    fn help(&self) -> &'static str {
        "Write value to memory address"
    }
}
```

### Command with Formatted Output

```rust
use core::fmt::Write;

struct StatsCommand;

impl CommandHandler for StatsCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        // Create a wrapper for fmt::Write
        struct IoWriter<'a>(&'a mut dyn ShellIO);

        impl fmt::Write for IoWriter<'_> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.0.write_str(s);
                Ok(())
            }
        }

        let mut writer = IoWriter(io);

        // Now you can use write! and writeln! macros
        write!(writer, "System Statistics:\r\n").ok();
        write!(writer, "  Uptime: {} seconds\r\n", get_uptime()).ok();
        write!(writer, "  Free memory: {} bytes\r\n", get_free_mem()).ok();

        Ok(())
    }

    fn help(&self) -> &'static str {
        "Display system statistics"
    }
}
```

### Command that Accesses Hardware

```rust
struct UartStatsCommand;

impl CommandHandler for UartStatsCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        use crate::uart::{Uart, UART0_BASE};

        let uart = Uart::new(UART0_BASE);

        // Read UART status registers (example)
        // Note: Adjust based on actual UART register layout

        io.write_str("UART0 Status:\r\n");
        io.write_str("  Base address: 0x10000000\r\n");
        io.write_str("  Baud rate: 38400\r\n");
        io.write_str("  Data bits: 8\r\n");
        io.write_str("  Parity: None\r\n");

        Ok(())
    }

    fn help(&self) -> &'static str {
        "Display UART status"
    }
}
```

## Command Patterns

### Pattern 1: Simple Action

```rust
impl CommandHandler for MyCommand {
    fn execute(&self, _args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        // Do something
        io.write_str("Action completed\r\n");
        Ok(())
    }
}
```

### Pattern 2: Argument Validation

```rust
impl CommandHandler for MyCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        if args.len() != 2 {
            return Err("Usage: mycommand <arg1> <arg2>");
        }

        let arg1 = args[0];
        let arg2 = args[1];

        // Process args...
        Ok(())
    }
}
```

### Pattern 3: Subcommands

```rust
impl CommandHandler for MyCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        if args.is_empty() {
            return Err("Usage: mycommand <start|stop|status>");
        }

        match args[0] {
            "start" => {
                io.write_str("Starting...\r\n");
                Ok(())
            }
            "stop" => {
                io.write_str("Stopping...\r\n");
                Ok(())
            }
            "status" => {
                io.write_str("Status: Running\r\n");
                Ok(())
            }
            _ => Err("Unknown subcommand. Use: start, stop, or status"),
        }
    }
}
```

### Pattern 4: Optional Arguments

```rust
impl CommandHandler for MyCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        let count = if args.len() > 0 {
            args[0].parse::<usize>().unwrap_or(1)
        } else {
            1  // Default value
        };

        for i in 0..count {
            use core::fmt::Write;
            let _ = write!(&mut Wrapper(io), "Iteration {}\r\n", i + 1);
        }

        Ok(())
    }
}
```

## Creating a Command Module

For complex commands, create a separate module:

### Create `src/shell/my_commands.rs`

```rust
use crate::shell::shell::{CommandHandler, ShellIO};
use core::fmt::Write;

pub struct MyCommand;

impl CommandHandler for MyCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        // Implementation...
        Ok(())
    }

    fn help(&self) -> &'static str {
        "My custom command"
    }
}

pub static MY_CMD: MyCommand = MyCommand;
```

### Update `src/shell/mod.rs`

```rust
pub mod shell;
pub mod uart_io;
pub mod commands;
pub mod my_commands;  // Add this line
```

### Import in `commands.rs`

```rust
use crate::shell::my_commands::MY_CMD;

pub static COMMANDS: &[Command] = &[
    // ...
    Command {
        name: "mycommand",
        handler: &MY_CMD,
    },
    // ...
];
```

## Alternative I/O Backend

### Creating a Custom I/O Backend

```rust
use crate::shell::shell::ShellIO;

pub struct MyCustomIO {
    // Your fields here
}

impl ShellIO for MyCustomIO {
    fn read_byte(&mut self) -> Option<u8> {
        // Read from your custom input source
        None
    }

    fn write_byte(&mut self, byte: u8) {
        // Write to your custom output
    }

    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}
```

### Using Custom I/O

```rust
let io = MyCustomIO::new();
let mut shell = Shell::new(io, COMMANDS, "custom> ");
shell.run();
```

## Tips and Best Practices

### 1. Keep Commands Simple

Each command should do one thing well. Complex logic should be in separate modules.

### 2. Always Validate Arguments

```rust
if args.len() < required_count {
    return Err("Usage: ...");
}
```

### 3. Use Descriptive Error Messages

```rust
// Good
return Err("Invalid port number (must be 1-4)");

// Bad
return Err("Invalid input");
```

### 4. Provide Help Text

```rust
fn help(&self) -> &'static str {
    "Command name - Brief description (args)"
}
```

### 5. Handle Parse Errors Gracefully

```rust
let value = args[0]
    .parse::<u32>()
    .map_err(|_| "Invalid number format")?;
```

### 6. Use VT100 Codes for Formatting

```rust
io.write_str("\x1b[1m");  // Bold
io.write_str("Important text");
io.write_str("\x1b[0m");  // Reset

io.write_str("\x1b[31m"); // Red
io.write_str("Error");
io.write_str("\x1b[0m");  // Reset
```

Common VT100 codes:
- `\x1b[0m` - Reset all attributes
- `\x1b[1m` - Bold
- `\x1b[31m` - Red
- `\x1b[32m` - Green
- `\x1b[33m` - Yellow
- `\x1b[2J\x1b[H` - Clear screen and home

### 7. Maintain Alphabetical Order

Always insert commands in alphabetical order in the COMMANDS array!

## Common Pitfalls

### 1. Forgetting to Add to COMMANDS Array

**Symptom**: Command not found

**Solution**: Add to COMMANDS array and ensure it's exported.

### 2. Not Sorting Commands

**Symptom**: Some commands not found, or assertion failure at compile time

**Solution**: Keep COMMANDS array sorted alphabetically.

### 3. Borrowing Issues

**Symptom**: Compiler errors about mutable/immutable borrows

**Solution**: Use `Wrapper` pattern for fmt::Write:

```rust
struct Wrapper<'a>(&'a mut dyn ShellIO);

impl fmt::Write for Wrapper<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s);
        Ok(())
    }
}

let _ = write!(&mut Wrapper(io), "Formatted {}", value);
```

### 4. Buffer Overflow

**Symptom**: Command input truncated or ignored

**Solution**: Input limited to 256 bytes. Keep commands short or increase INPUT_BUFFER_SIZE in shell.rs.

## Testing Your Command

### Manual Testing Checklist

- [ ] Command responds to `help`
- [ ] Command appears in `help` list
- [ ] Basic functionality works
- [ ] Error cases return proper error messages
- [ ] Help text is descriptive
- [ ] Command doesn't crash on invalid input
- [ ] Command doesn't block forever

### Example Test Session

```
kernel> help
Available commands:
  mycommand    My custom command description

kernel> mycommand
Output from my command

kernel> mycommand invalid
Error: Invalid argument

kernel> help mycommand
mycommand: My custom command description
```

## Need Help?

- Review existing commands in `/home/heng/test/rust_riscv/src/shell/commands.rs`
- Check the main documentation in `/home/heng/test/rust_riscv/SHELL_README.md`
- Refer to the architecture design in `/home/heng/test/rust_riscv/shell_architecture.md`

---

Happy hacking! 🦀
