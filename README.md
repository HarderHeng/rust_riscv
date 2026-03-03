# RISC-V Bare-Metal Rust Kernel

A minimalist bare-metal kernel written in Rust for RISC-V 32-bit architecture, running on QEMU's `virt` machine. This project demonstrates low-level systems programming without an operating system or C runtime.

## Features

- **Pure Rust**: Written entirely in Rust with `#![no_std]` and `#![no_main]`
- **RISC-V 32-bit**: Target architecture `riscv32imac-unknown-none-elf`
  - `i`: Integer base instruction set
  - `m`: Multiplication/division extensions
  - `a`: Atomic instructions
  - `c`: Compressed 16-bit instructions
- **Bare-metal**: No operating system, no C runtime, direct hardware control
- **UART Driver**: 16550A UART driver for serial output with formatted printing
- **Custom Linker Script**: Precise control over memory layout
- **QEMU Support**: Ready to run on QEMU's RISC-V virt machine

## Quick Start

### Prerequisites

- Rust toolchain with `riscv32imac-unknown-none-elf` target
- QEMU RISC-V emulator (`qemu-system-riscv32`)
- (Optional) RISC-V GDB for debugging

Install the Rust target:
```bash
rustup target add riscv32imac-unknown-none-elf
```

### Build and Run

Build the kernel:
```bash
cargo build                    # Debug build
cargo build --release          # Optimized release build
```

Run in QEMU:
```bash
cargo run                      # Build and run kernel in QEMU
cargo run --release            # Run optimized kernel
```

Expected output:
```
RISC-V Bare-Metal Kernel
Initializing...
Hello from Rust kernel!
Kernel running, waiting for interrupts...
```

Press `Ctrl-A` then `X` to exit QEMU.

### Debug with GDB

Launch QEMU in debug mode (suspended, waiting for GDB):
```bash
cargo run -- gdb               # QEMU listens on port 1234
```

In another terminal, connect GDB:
```bash
riscv32-unknown-elf-gdb target/riscv32imac-unknown-none-elf/debug/kernel
```

Inside GDB:
```gdb
(gdb) target remote :1234      # Connect to QEMU
(gdb) break kernel_main        # Set breakpoint
(gdb) continue                 # Resume execution
```

### Inspect the Binary

View memory map:
```bash
less kernel.map
```

Disassemble the kernel:
```bash
rust-objdump -d target/riscv32imac-unknown-none-elf/debug/kernel
```

Check section sizes:
```bash
rust-size target/riscv32imac-unknown-none-elf/debug/kernel
```

## Project Structure

```
rust_riscv/
├── src/
│   ├── main.rs          # Kernel entry point, panic handler, macros
│   ├── startup.rs       # Assembly boot code, linker symbols
│   └── uart.rs          # 16550A UART driver
├── linker.ld            # Custom linker script for memory layout
├── build.rs             # Build script (linker configuration)
├── qemu-runner.sh       # QEMU launch wrapper
├── Cargo.toml           # Rust project configuration
└── .cargo/config.toml   # Cargo build configuration
```

### Module Overview

**`src/main.rs`**: Kernel entry and utilities
- `kernel_main()`: Main kernel function called from boot code
- `kprint!` / `kprintln!`: Macros for formatted UART output
- `panic()`: Custom panic handler for bare-metal environment

**`src/startup.rs`**: Low-level boot sequence
- `_start`: Assembly entry point that QEMU jumps to
- Initializes stack pointer and zeros BSS section
- Declares linker symbols (`_sbss`, `_ebss`, `_stack_top`, etc.)
- Helper functions: `bss_range()`, `heap_range()`

**`src/uart.rs`**: Serial communication driver
- 16550A UART driver (memory-mapped at `0x1000_0000`)
- Implements `core::fmt::Write` for formatted output
- All register access uses volatile reads/writes

## Memory Layout

Memory map for QEMU `virt` machine (defined in `linker.ld`):

| Address Range           | Region | Description |
|------------------------|--------|-------------|
| `0x0000_0000 - 0x0FFF_FFFF` | MMIO | Memory-mapped I/O (UART at `0x1000_0000`) |
| `0x8000_0000 - 0x8800_0000` | DRAM | 128 MiB RAM (kernel loaded here) |

Memory sections in RAM (starting at `0x8000_0000`):

1. **`.text`**: Code section, begins with `_start` entry point
2. **`.rodata`**: Read-only data (string literals, constants)
3. **`.data`**: Initialized read-write data
4. **`.bss`**: Zero-initialized data (cleared by boot code)
5. **`.stack`**: 64 KiB kernel stack
6. **heap**: Remaining memory after stack (not yet allocated)

## Boot Sequence

1. QEMU loads the ELF binary to `0x8000_0000` and jumps to `_start`
2. `_start` (assembly in `src/startup.rs`):
   - Sets up stack pointer (`sp` = `_stack_top`)
   - Zeros the `.bss` section
   - Calls `kernel_main()` in Rust (never returns)
3. `kernel_main()`:
   - Initializes UART hardware
   - Prints startup messages via `kprintln!`
   - Enters infinite `wfi` (Wait For Interrupt) loop

## Development

### Adding Formatted Output

Use the `kprint!` and `kprintln!` macros (similar to `print!` and `println!`):

```rust
kprintln!("Hello, world!");
kprint!("Value: {}", 42);
```

### Adding a Hardware Driver

1. Create a new module in `src/` (e.g., `src/timer.rs`)
2. Add `mod timer;` to `src/main.rs`
3. Document MMIO base address and register layout
4. Use `read_volatile` / `write_volatile` for all hardware access
5. Expose a safe public API (keep `unsafe` only at MMIO boundary)

### Memory Allocation

Currently, there is no heap allocator. To enable dynamic allocation:

1. Use `heap_range()` from `startup.rs` to get heap bounds
2. Implement `#[global_allocator]` (e.g., using `linked_list_allocator`)
3. This unlocks `alloc` crate types: `Vec`, `Box`, `String`, etc.

### Key Conventions

- **No standard library**: All code is `#![no_std]` (use `core::` instead of `std::`)
- **Volatile MMIO**: All hardware register access must use `read_volatile` / `write_volatile`
- **Line endings**: Use `\r\n` for UART output (terminal compatibility)
- **Panic behavior**: Configured to `abort` (no unwinding support)
- **Size optimization**: Release builds use `opt-level = "z"` and LTO

## Troubleshooting

**Kernel doesn't print anything**
- Ensure UART is initialized before calling `kprintln!`
- Verify `Uart::new(UART0_BASE).init()` is called in `kernel_main()`

**Linker errors about undefined symbols**
- Check that `linker.ld` exists in repository root
- Ensure `build.rs` exists (tells Cargo to track linker script changes)

**"error: requires `start` lang item"**
- This is expected in bare-metal environments
- The `#![no_main]` attribute and custom `_start` provide the entry point

**QEMU exits immediately**
- Kernel must never return from `kernel_main()`
- Always end with an infinite loop (usually `wfi` to wait for interrupts)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Resources

- [RISC-V Specifications](https://riscv.org/technical/specifications/)
- [QEMU RISC-V Documentation](https://www.qemu.org/docs/master/system/target-riscv.html)
- [Rust Embedded Book](https://rust-embedded.github.io/book/)
- [16550 UART Datasheet](http://caro.su/msx/ocm_de1/16550.pdf)