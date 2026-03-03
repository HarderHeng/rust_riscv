# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a bare-metal Rust kernel for RISC-V 32-bit (riscv32imac) running on QEMU's `virt` machine. It's a no_std, no_main environment with no operating system or C runtime underneath.

**Target Architecture**: riscv32imac-unknown-none-elf
- **i**: Integer base instruction set
- **m**: Multiplication/division extensions
- **a**: Atomic instructions
- **c**: Compressed instructions (16-bit)

## Build and Run Commands

### Build the kernel
```bash
cargo build                    # Debug build
cargo build --release          # Optimized release build
```

### Run in QEMU
```bash
cargo run                      # Build and run kernel in QEMU
cargo run --release            # Run optimized kernel
```

### Debug with GDB
```bash
cargo run -- gdb               # Launch QEMU suspended, waiting for GDB on port 1234
```

Then in another terminal:
```bash
riscv32-unknown-elf-gdb target/riscv32imac-unknown-none-elf/debug/kernel
# Inside gdb:
(gdb) target remote :1234
(gdb) continue
```

### Inspect the binary
```bash
# View memory map
less kernel.map

# Disassemble
rust-objdump -d target/riscv32imac-unknown-none-elf/debug/kernel

# Check sizes
rust-size target/riscv32imac-unknown-none-elf/debug/kernel
```

## Memory Layout

Memory map is defined in `linker.ld` for QEMU virt machine:
- **0x0000_0000 - 0x0FFF_FFFF**: MMIO region (UART0 at 0x1000_0000)
- **0x8000_0000 - 0x8800_0000**: DRAM (128 MiB, kernel loaded here)

Memory sections (all in RAM):
1. **`.text`** (0x8000_0000): Code, starts with `_start` entry point
2. **`.rodata`**: Read-only data (string literals, etc.)
3. **`.data`**: Initialized read-write data
4. **`.bss`**: Zero-initialized data (cleared by `_start`)
5. **`.stack`**: 64 KiB kernel stack
6. **heap**: Everything after stack until end of RAM

## Architecture

### Boot Sequence
1. QEMU loads ELF to 0x8000_0000 and jumps to `_start` (defined in `src/startup.rs`)
2. `_start` assembly code:
   - Initializes stack pointer to `_stack_top`
   - Zeros BSS section (`_sbss` to `_ebss`)
   - Jumps to `kernel_main` in Rust (never returns)
3. `kernel_main` (in `src/main.rs`):
   - Initializes UART
   - Prints startup messages
   - Enters infinite WFI (Wait For Interrupt) loop

### Module Structure

**`src/startup.rs`**: CPU reset vector and linker symbol declarations
- Declares linker symbols as `extern "C" static` (use address-of, never read value)
- Contains `_start` assembly entry point
- Provides helper functions: `bss_range()`, `heap_range()`

**`src/uart.rs`**: 16550A UART driver for QEMU virt machine
- Memory-mapped at 0x1000_0000
- All register access via `read_volatile`/`write_volatile` to prevent compiler optimization
- Implements `core::fmt::Write` for formatted output
- Backend for `kprint!`/`kprintln!` macros

**`src/main.rs`**: Kernel entry and macros
- Defines `kprint!` and `kprintln!` macros (like `print!`/`println!` but for UART)
- `kernel_main()`: Main kernel function
- `panic()`: Panic handler (prints location, then infinite WFI)

### Key Conventions

1. **No standard library**: All code is `#![no_std]`, use `core::` instead of `std::`
2. **MMIO safety**: All hardware register access must use `read_volatile`/`write_volatile`
3. **Line endings**: Use `\r\n` for UART output (terminal compatibility)
4. **Panic behavior**: Both dev and release profiles use `panic = "abort"` (no unwinding)
5. **Optimization**: Release builds use `opt-level = "z"` (optimize for size) and LTO

## Adding New Code

### Adding a new hardware driver
- Create new module in `src/` (e.g., `src/timer.rs`)
- Add `mod timer;` to `src/main.rs`
- Document MMIO base address and register layout
- Use volatile reads/writes for all hardware access
- Make the driver's public interface safe Rust (unsafe only at MMIO boundary)

### Adding formatted output
Use the existing `kprint!`/`kprintln!` macros:
```rust
kprintln!("Value: {}", x);
kprint!("No newline");
```

### Memory allocation
Currently no allocator. To add one:
- Use `heap_range()` from `startup.rs` to get heap bounds
- Implement `#[global_allocator]` using a crate like `linked_list_allocator`
- This enables `alloc` crate types: `Vec`, `Box`, `String`, etc.

## Common Issues

**Kernel doesn't print anything**: UART must be initialized before calling `kprintln!`. Ensure `Uart::new(UART0_BASE).init()` is called first in `kernel_main()`.

**Linker errors about undefined symbols**: Make sure `linker.ld` is in the repository root and `build.rs` exists (it tells cargo to relink when the linker script changes).

**"error: requires `start` lang item"**: This is expected in bare-metal. The `#![no_main]` attribute and `_start` assembly provide the entry point instead.

**QEMU exits immediately**: Kernel must never return from `kernel_main()`. Always end with an infinite loop (typically `wfi` to wait for interrupts).