# Shell System Architecture Design

## 1. Overview

This document describes the architecture of a bare-metal Shell system for the RISC-V kernel. The design prioritizes modularity, extensibility, and no_std compatibility.

## 2. Design Goals

- **Modularity**: Clear separation between I/O, parsing, and command execution
- **Extensibility**: Easy to add new commands without modifying core code
- **no_std Compatible**: Uses fixed-size buffers and heapless data structures
- **Resource Efficient**: Minimal memory footprint suitable for embedded systems
- **Type Safety**: Leverages Rust's type system for compile-time guarantees

## 3. Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│                     User Interface                       │
│                    (UART Terminal)                       │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                    I/O Abstraction                       │
│  ┌────────────────────────────────────────────────────┐ │
│  │          ShellIO Trait (read/write/prompt)         │ │
│  └────────────────────────────────────────────────────┘ │
│           │                              │               │
│           ▼                              ▼               │
│  ┌──────────────────┐         ┌──────────────────────┐ │
│  │  UartIO (Polling)│         │ UartIO (Interrupt)*  │ │
│  └──────────────────┘         └──────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                      Shell Core                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │              Shell State Machine                    │ │
│  │  • Input Buffer (256 bytes)                        │ │
│  │  • History Buffer (optional, 512 bytes)            │ │
│  │  • Cursor Position                                 │ │
│  │  • Shell Prompt                                    │ │
│  └────────────────────────────────────────────────────┘ │
│                            │                             │
│                            ▼                             │
│  ┌────────────────────────────────────────────────────┐ │
│  │              Command Parser                         │ │
│  │  • Tokenization (split by whitespace)              │ │
│  │  • Argument parsing                                │ │
│  │  • Quote handling (optional)                       │ │
│  └────────────────────────────────────────────────────┘ │
│                            │                             │
│                            ▼                             │
│  ┌────────────────────────────────────────────────────┐ │
│  │           Command Registry & Dispatch               │ │
│  │  • Static command table                            │ │
│  │  • Command lookup (binary search)                  │ │
│  │  • Execution dispatch                              │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                   Command Layer                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │         CommandHandler Trait (execute)             │ │
│  └────────────────────────────────────────────────────┘ │
│           │           │           │           │          │
│           ▼           ▼           ▼           ▼          │
│     ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐    │
│     │  help  │  │  echo  │  │ clear  │  │version │    │
│     └────────┘  └────────┘  └────────┘  └────────┘    │
│           ...more built-in commands...                   │
└─────────────────────────────────────────────────────────┘
```

## 4. Core Components

### 4.1 ShellIO Trait (I/O Abstraction)

**Purpose**: Abstract I/O operations to support different backends (polling, interrupt-driven, etc.)

**Interface**:
```rust
pub trait ShellIO {
    /// Read a single byte (non-blocking)
    fn read_byte(&mut self) -> Option<u8>;

    /// Write a single byte
    fn write_byte(&mut self, byte: u8);

    /// Write a string
    fn write_str(&mut self, s: &str);

    /// Write formatted output (using core::fmt::Write)
    fn write_fmt(&mut self, args: core::fmt::Arguments);
}
```

**Implementations**:
- `UartIO`: Wraps existing `uart::Uart` for UART-based I/O
- Future: `MockIO` for testing

### 4.2 Command System

**Command Struct**:
```rust
pub struct Command {
    name: &'static str,        // Command name (e.g., "help")
    help: &'static str,        // Short help text
    handler: &'static dyn CommandHandler,
}
```

**CommandHandler Trait**:
```rust
pub trait CommandHandler {
    /// Execute command with arguments
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str>;
}
```

**Command Registry**:
- Static array of Command structs: `static COMMANDS: &[Command] = &[...]`
- Sorted by name for binary search (O(log n) lookup)
- Commands registered at compile-time (no dynamic allocation)

### 4.3 Shell Core

**Shell Struct**:
```rust
pub struct Shell<'a, IO: ShellIO> {
    io: IO,
    input_buffer: [u8; 256],      // Current input line
    input_len: usize,              // Current input length
    cursor_pos: usize,             // Cursor position in buffer
    commands: &'a [Command],       // Command registry
    prompt: &'static str,          // Shell prompt
}
```

**Main Loop**:
1. **Read**: Poll for input characters
2. **Process**: Handle special keys (backspace, enter, Ctrl+C)
3. **Parse**: Split input into command and arguments
4. **Execute**: Lookup and dispatch to command handler
5. **Print**: Display output and prompt

### 4.4 Command Parser

**Tokenization Strategy**:
- Split input by whitespace (space, tab)
- Store tokens as string slices into input buffer (zero-copy)
- Use fixed-size array for argument pointers: `[&str; 16]`
- No quote handling in initial version (can be added later)

**Implementation**:
```rust
fn parse_command(input: &str) -> (&str, Vec<&str, 16>) {
    // Returns (command_name, arguments)
}
```

### 4.5 Line Editing

**Supported Features**:
- **Backspace/Delete**: Remove character, update display
- **Character Echo**: Echo printable characters as typed
- **Enter**: Process command line
- **Ctrl+C**: Clear current line
- **Ctrl+L**: Clear screen (optional)

**Not Supported (initially)**:
- Arrow keys (cursor movement)
- Command history
- Tab completion
- Multi-line editing

## 5. Memory Management

### 5.1 Buffer Sizes

- **Input Buffer**: 256 bytes (sufficient for typical commands)
- **Argument Array**: 16 pointers (16 arguments max)
- **History Buffer**: Optional, 512 bytes (4-5 commands)

### 5.2 Data Structures

- **heapless::Vec**: For dynamic arrays without allocation
- **heapless::String**: For string manipulation
- Fixed-size arrays where size is known

### 5.3 Memory Layout

Total estimated memory usage: ~1 KB
- Shell struct: ~300 bytes
- Command registry: ~500 bytes (depends on number of commands)
- Stack usage: ~200 bytes

## 6. Built-in Commands

### 6.1 Core Commands

| Command | Description | Arguments |
|---------|-------------|-----------|
| `help` | List all commands or show help for specific command | `[command_name]` |
| `echo` | Echo arguments to output | `args...` |
| `clear` | Clear screen (send VT100 escape codes) | none |
| `version` | Display kernel version | none |
| `reboot` | Reboot system (via SBI or MMIO) | none |

### 6.2 Utility Commands

| Command | Description | Arguments |
|---------|-------------|-----------|
| `uptime` | Show system uptime | none |
| `meminfo` | Display memory information | none |
| `peek` | Read memory address | `address` |
| `poke` | Write to memory address | `address value` |

### 6.3 Debug Commands

| Command | Description | Arguments |
|---------|-------------|-----------|
| `panic` | Trigger kernel panic (for testing) | none |
| `test` | Run self-tests | none |

## 7. Extensibility

### 7.1 Adding New Commands

To add a new command:

1. **Implement CommandHandler trait**:
```rust
struct MyCommand;
impl CommandHandler for MyCommand {
    fn execute(&self, args: &[&str], io: &mut dyn ShellIO) -> Result<(), &'static str> {
        // Implementation
        Ok(())
    }
}
```

2. **Register in command table**:
```rust
static MY_CMD: MyCommand = MyCommand;

static COMMANDS: &[Command] = &[
    // ... existing commands ...
    Command {
        name: "mycommand",
        help: "Description of my command",
        handler: &MY_CMD,
    },
];
```

3. **Recompile** - that's it!

### 7.2 Alternative I/O Backends

To add a new I/O backend:

1. **Implement ShellIO trait**:
```rust
struct MyIO { /* ... */ }
impl ShellIO for MyIO {
    // Implement trait methods
}
```

2. **Create Shell with new I/O**:
```rust
let io = MyIO::new();
let shell = Shell::new(io, &COMMANDS);
```

## 8. Error Handling

### 8.1 Error Types

- **Command Not Found**: Display "Command not found: <name>"
- **Invalid Arguments**: Command returns `Err("error message")`
- **I/O Errors**: Currently panics (can be improved with Result types)

### 8.2 Error Reporting

All errors are printed to the shell output with a consistent format:
```
Error: <error message>
```

## 9. Future Enhancements

### Phase 2:
- Command history (up/down arrow keys)
- Tab completion
- Command aliases
- Environment variables
- Command pipelines (|)

### Phase 3:
- Background jobs (&)
- Job control (fg, bg, jobs)
- Signal handling
- Script execution

### Phase 4:
- Interrupt-driven I/O with ring buffer
- DMA support for UART
- Multi-line command support
- Advanced line editing (emacs/vi mode)

## 10. Testing Strategy

### 10.1 Unit Tests

- Command parsing (tokenization)
- Individual command handlers
- Buffer management

### 10.2 Integration Tests

- Full shell loop with MockIO
- Command execution
- Error handling

### 10.3 Manual Testing

- QEMU testing with real UART
- Interactive testing of all commands
- Edge cases (long inputs, special characters)

## 11. Implementation Order

1. **Architecture Design** ✓
2. **Core Module** (shell.rs):
   - ShellIO trait
   - Command/CommandHandler traits
   - Shell struct
   - Parser
   - Main loop
3. **UART Adapter** (uart_io.rs):
   - UartIO implementation
   - Line editing
4. **Built-in Commands**:
   - help, echo, clear, version
   - Additional utilities
5. **Integration**:
   - mod.rs
   - main.rs integration
   - Testing
   - Documentation

## 12. Code Organization

```
src/
├── main.rs              (integrates shell)
├── shell/
│   ├── mod.rs          (module exports)
│   ├── shell.rs        (core Shell implementation)
│   ├── uart_io.rs      (UartIO implementation)
│   ├── commands/
│   │   ├── mod.rs      (command registry)
│   │   ├── help.rs     (help command)
│   │   ├── echo.rs     (echo command)
│   │   ├── clear.rs    (clear command)
│   │   ├── version.rs  (version command)
│   │   └── ...
│   └── parser.rs       (optional: command parser)
└── ...
```

## 13. Dependencies

### 13.1 External Crates (no_std compatible)

- `heapless` = "0.7" - For fixed-capacity Vec/String
- Optional: `arrayvec` - Alternative to heapless

### 13.2 Internal Dependencies

- `uart` module (existing)
- `kprintln!` macro (for debugging, not shell output)

## 14. Summary

This architecture provides a clean, modular, and extensible shell system suitable for bare-metal RISC-V environments. The design leverages Rust's type system for safety while maintaining efficiency through fixed-size buffers and zero-allocation operations. The command system is flexible and easy to extend, making it simple to add new functionality as the kernel evolves.