# Shell System Documentation

## Overview

A modular, extensible command-line shell system for the bare-metal RISC-V kernel. The shell provides an interactive interface for kernel control and debugging.

## Architecture

The shell system consists of four main components:

### 1. I/O Abstraction Layer (`shell.rs`)

**ShellIO Trait**: Abstracts I/O operations to support different backends.

```rust
pub trait ShellIO {
    fn read_byte(&mut self) -> Option<u8>;
    fn write_byte(&mut self, byte: u8);
    fn write_str(&mut self, s: &str);
}
```

**Implementations**:
- `UartIO` (polling-based UART I/O) - `/home/heng/test/rust_riscv/src/shell/uart_io.rs`
- Future: interrupt-driven I/O, MockIO for testing

### 2. Command System (`shell.rs`)

**CommandHandler Trait**: Interface for all shell commands.

```rust
pub trait CommandHandler: Sync {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str>;
    fn help(&self) -> &'static str;
}
```

**Command Registry**: Static array of commands, sorted alphabetically for O(log n) lookup.

### 3. Shell Core (`shell.rs`)

**Shell Struct**: Main state machine that implements the read-parse-execute-print loop.

Features:
- 256-byte input buffer
- Line editing (backspace, Ctrl+C, Ctrl+L)
- Command parsing and dispatch
- Non-blocking polling mode
- VT100 escape sequence support

### 4. Built-in Commands (`commands.rs`)

| Command | Description |
|---------|-------------|
| `help` | List all commands or show help for specific command |
| `echo` | Echo arguments to output |
| `clear` | Clear the screen (VT100 escape codes) |
| `version` | Display kernel version and build info |
| `uptime` | Show system uptime (placeholder) |
| `reboot` | Reboot system via QEMU test device |
| `panic` | Trigger kernel panic for testing |
| `meminfo` | Display memory layout (BSS, heap) |

## Usage

### Basic Usage

```rust
use shell::{Shell, UartIO};
use shell::commands::COMMANDS;

let io = UartIO::uart0();
let mut shell = Shell::new(io, COMMANDS, "kernel> ");
shell.run(); // Never returns
```

### Integration with Kernel Loop

```rust
let mut shell = Shell::new(io, COMMANDS, "kernel> ");
shell.start();

loop {
    shell.poll(); // Non-blocking
    // Other kernel tasks...
    unsafe { core::arch::asm!("wfi"); }
}
```

## Adding New Commands

### Step 1: Implement CommandHandler

Create a new command handler struct:

```rust
struct MyCommand;

impl CommandHandler for MyCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        io.write_str("Hello from my command!\r\n");

        // Access arguments
        if args.len() > 0 {
            io.write_str("First argument: ");
            io.write_str(args[0]);
            io.write_str("\r\n");
        }

        Ok(())
    }

    fn help(&self) -> &'static str {
        "My custom command"
    }
}

static MY_CMD: MyCommand = MyCommand;
```

### Step 2: Register in Command Table

Add to the COMMANDS array in `/home/heng/test/rust_riscv/src/shell/commands.rs`:

```rust
pub static COMMANDS: &[Command] = &[
    // ... existing commands ...
    Command {
        name: "mycommand",  // MUST be in alphabetical order!
        handler: &MY_CMD,
    },
    // ... more commands ...
];
```

**IMPORTANT**: Commands must be sorted alphabetically by name for binary search to work.

### Step 3: Recompile

```bash
cargo build
```

That's it! Your command is now available in the shell.

## Line Editing Features

### Supported

- **Backspace/DEL (0x08, 0x7F)**: Delete last character
- **Enter (CR/LF)**: Execute command
- **Ctrl+C (0x03)**: Cancel current line
- **Ctrl+L (0x0C)**: Clear screen
- **Character echo**: All printable ASCII characters

### Not Yet Implemented

- Arrow keys (left/right cursor movement)
- Command history (up/down arrows)
- Tab completion
- Multi-line editing

## Memory Usage

Estimated memory footprint:

- Shell struct: ~300 bytes
- Command registry: ~500 bytes (8 commands)
- Input buffer: 256 bytes
- **Total: ~1 KB**

All memory is statically allocated - no heap required.

## Error Handling

### Command Errors

Commands return `Result<(), &'static str>`:

```rust
fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
    if args.len() < 1 {
        return Err("Missing required argument");
    }
    // ... command logic ...
    Ok(())
}
```

Errors are displayed as: `Error: <message>`

### Unknown Commands

When a command is not found:
```
Command not found: <name>
Type 'help' for available commands.
```

## Testing

### Manual Testing in QEMU

```bash
cargo run
```

Interactive commands to test:
```
kernel> help
kernel> echo hello world
kernel> version
kernel> meminfo
kernel> clear
kernel> reboot
```

### Unit Tests (Future)

To add unit tests, create a MockIO implementation:

```rust
struct MockIO {
    input: Vec<u8>,
    output: Vec<u8>,
}

impl ShellIO for MockIO {
    fn read_byte(&mut self) -> Option<u8> {
        self.input.pop()
    }

    fn write_byte(&mut self, byte: u8) {
        self.output.push(byte);
    }
}
```

## File Organization

```
src/shell/
├── mod.rs           - Module exports
├── shell.rs         - Core Shell implementation
│                     (ShellIO trait, Shell struct, parser)
├── uart_io.rs       - UART I/O adapter (UartIO)
└── commands.rs      - Built-in commands
                      (help, echo, clear, version, etc.)
```

## Performance

- **Command lookup**: O(log n) via binary search
- **Parsing**: O(n) single-pass tokenization
- **Memory**: Zero heap allocations (heapless::Vec)
- **Polling overhead**: Minimal (~1 UART read per iteration)

## Limitations

1. **Buffer size**: Input limited to 256 characters
2. **Arguments**: Maximum 16 arguments per command
3. **No quotes**: Quote handling not implemented
4. **No pipes**: Command pipelines not supported
5. **No scripting**: No variable expansion or control flow

## Future Enhancements

### Phase 2
- Command history with up/down arrows
- Tab completion
- Command aliases
- Configurable prompts

### Phase 3
- Environment variables
- Command pipelines (|)
- Output redirection (>, >>)
- Background jobs (&)

### Phase 4
- Interrupt-driven I/O
- Script execution
- Multi-line commands
- Advanced line editing (emacs/vi modes)

## Troubleshooting

### Shell doesn't start

**Symptom**: No prompt appears

**Solution**: Ensure UART is initialized before creating shell:
```rust
let uart = Uart::new(UART0_BASE);
uart.init();  // MUST call this first!
```

### Commands not found

**Symptom**: "Command not found" for valid commands

**Possible causes**:
1. Command name misspelled
2. Commands not sorted alphabetically in COMMANDS array
3. Command not added to COMMANDS array

**Solution**: Verify command name and COMMANDS array order.

### Backspace doesn't work

**Symptom**: Backspace prints garbage characters

**Solution**: Your terminal may be sending different control codes. The shell handles both 0x08 (BS) and 0x7F (DEL).

### Shell hangs

**Symptom**: Shell stops responding

**Possible causes**:
1. Command panicked (check for panic messages)
2. Infinite loop in command handler
3. UART hardware issue

**Solution**: Press Ctrl+C to cancel current command, or reboot system with `reboot` command.

## API Reference

### Shell::new()

```rust
pub fn new(io: IO, commands: &'a [Command], prompt: &'static str) -> Self
```

Create a new shell instance.

**Parameters**:
- `io`: I/O backend implementing ShellIO
- `commands`: Sorted command registry
- `prompt`: Shell prompt string (e.g., "kernel> ")

### Shell::start()

```rust
pub fn start(&mut self)
```

Display welcome message and initial prompt. Call before entering main loop.

### Shell::poll()

```rust
pub fn poll(&mut self) -> bool
```

Process available input (non-blocking). Returns `true` if command was executed.

Use in kernel main loop for integration with other tasks.

### Shell::run()

```rust
pub fn run(&mut self) -> !
```

Run shell in blocking loop (never returns). Uses WFI instruction for power saving.

Use if shell is the only kernel task.

## License

Part of the RISC-V Bare-Metal Kernel project.

## Contributing

To add new commands or features:

1. Implement CommandHandler trait
2. Add to COMMANDS array (sorted!)
3. Test in QEMU
4. Update documentation
5. Submit changes

---

**Last updated**: 2026-03-03
**Version**: 0.1.0
