# QEMU virt-rv32 Board

This is the RISC-V 32-bit board support package for QEMU's `virt` machine.

## Target

- **Architecture**: `riscv32imac-unknown-none-elf`
- **Machine**: QEMU virt (virtualized RISC-V platform)
- **ISA**: RV32IMAC (Integer, Multiplication, Atomic, Compressed)

## Hardware

### Memory Map

> 完整权威映射见 `docs/architecture/memory-map.md`。本板为 RV32、M 模式、MMU 关闭，地址均为物理地址。

- **0x0010_0000**: VIRT_TEST (syscon; write 0x7777 = reboot)
- **0x0200_0000**: CLINT (software interrupt + timer)
- **0x0C00_0000**: PLIC (Platform-Level Interrupt Controller)
- **0x1000_0000**: UART0 (16550A, clock 3.6864 MHz, IRQ 10)
- **0x3000_0000**: PCIe ECAM | **0x4000_0000**: PCIe MMIO
- **0x8000_0000 - 0x8800_0000**: RAM (128 MiB)

### Peripherals

- **UART0**: 16550A UART at 0x1000_0000 (IRQ 10)
- **PLIC**: Interrupt controller at 0x0C00_0000

## Building

```bash
cd crates/boards/qemu-virt-rv32
cargo build
cargo build --release
```

## Running

```bash
cargo run           # Run in QEMU
cargo run --release # Run optimized build
```

## Debugging

```bash
cargo run -- gdb    # Start QEMU waiting for GDB on port 1234
```

In another terminal:
```bash
riscv32-unknown-elf-gdb target/riscv32imac-unknown-none-elf/debug/qemu-virt-rv32
(gdb) target remote :1234
(gdb) continue
```

## Structure

```
.
├── src/
│   ├── main.rs              # Entry point, panic handler
│   ├── platform.rs          # QemuVirtPlatform implementation
│   ├── startup.rs           # Linker symbols + riscv-common _start retainer
│   └── hal_impl/            # HAL trait implementations
│       ├── uart_16550a.rs   # SerialPort implementation
│       ├── plic.rs          # InterruptController implementation
│       └── mod.rs
├── linker.ld                # Linker script for memory layout
├── build.rs                 # Build script
├── qemu-runner.sh           # QEMU runner script
└── Cargo.toml               # Dependencies and build config
```

## Dependencies

- **kernel**: Platform-agnostic kernel with shell and trap handling
- **hal**: Hardware abstraction layer trait definitions
- **riscv-common**: Common RISC-V utilities (startup, CSR access)
