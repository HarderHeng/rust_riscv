# BL808 E907 Board Support (riscv32imacf)

## Overview

This crate provides bare-metal support for the **BL808 E907 core** - the application processor M4F RISC-V core on the Bouffalo Lab BL808 SoC with hardware floating-point support.

**Status**: PLACEHOLDER - Awaiting hardware specifications

**Target**: `riscv32imacf-unknown-none-elf`
- **I**: Integer base instruction set
- **M**: Integer multiplication/division
- **A**: Atomic instructions
- **C**: Compressed instructions (16-bit)
- **F**: Single-precision floating-point (hardware FPU)

## Core Specifications

- **Architecture**: RISC-V RV32IMACF
- **Purpose**: Main application core with hardware FPU
- **Memory**: ITCM (Instruction TCM) + DTCM (Data TCM) + shared RAM
- **FPU**: Hardware single-precision floating-point unit
- **Typical Use Cases**: Real-time signal processing, audio/video, computation-intensive tasks

## What's Needed

To complete this board support package, we need:

### 1. Memory Map
- [ ] ITCM base address and size (larger than E902)
- [ ] DTCM base address and size (larger than E902)
- [ ] Shared RAM base address and size
- [ ] MMIO peripheral region addresses
- [ ] Cache configuration (if present)

**Resources**: BL808 datasheet, E907 technical reference manual

### 2. FPU Configuration
- [ ] FPU enable sequence (CSR registers)
- [ ] Floating-point context save/restore
- [ ] FPU exception handling
- [ ] Compiler flags for optimal FPU usage

### 3. UART Configuration
- [ ] UART peripheral base address (likely shared with E902)
- [ ] Register layout (16550-compatible? Custom?)
- [ ] Clock source and baud rate calculation
- [ ] Pin muxing requirements
- [ ] DMA support for high-speed transfers

### 4. Interrupt Controller
- [ ] PLIC base address
- [ ] Number of external interrupt sources
- [ ] Interrupt priority levels
- [ ] Core-local interrupts (CLINT) base address
- [ ] Inter-core interrupt mechanism (E907 ↔ E902 and E907 ↔ C906)

### 5. Clock Configuration
- [ ] E907 core clock frequency (typically higher than E902)
- [ ] Peripheral clock domains
- [ ] PLL configuration registers
- [ ] Clock gating controls

### 6. Build and Debug Tools
- [ ] OpenOCD configuration for BL808 E907
- [ ] JTAG/SWD adapter setup
- [ ] Flash programming procedure
- [ ] GDB stub configuration with FPU register support

### 7. Startup Sequence
- [ ] Boot process from reset
- [ ] Clock initialization requirements
- [ ] FPU initialization
- [ ] Memory initialization (TCM enable, cache config)
- [ ] Stack and heap placement strategy

## Current Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Cargo.toml | ✅ Done | Dependencies on kernel, hal, riscv-common |
| .cargo/config.toml | ✅ Done | Target: riscv32imacf-unknown-none-elf |
| linker.ld | ⚠️ Template | PLACEHOLDER memory addresses |
| src/main.rs | ✅ Done | Entry point with todo!() placeholders |
| src/hal_impl/uart_bl808.rs | ⚠️ Stub | todo!() - need UART specs |
| src/hal_impl/interrupt.rs | ⚠️ Stub | todo!() - need PLIC specs |
| src/hal_impl/platform.rs | ⚠️ Stub | Placeholder values |
| openocd-runner.sh | ⚠️ Placeholder | Need OpenOCD config |

## Building

```bash
cd crates/boards/bl808-e907
cargo build
```

**Note**: Build will fail with `todo!()` panics until hardware specifications are provided.

## Hardware FPU Notes

The E907 core includes a single-precision floating-point unit. To use it effectively:

1. **Enable FPU in CSRs**: Set `mstatus.FS` field to enable FPU context
2. **Compiler Support**: The `riscv32imacf-unknown-none-elf` target enables hardware FP instructions
3. **Context Switching**: Save/restore FPU registers (f0-f31, fcsr) on interrupts if needed

Example FPU test (once platform is functional):
```rust
let a: f32 = 3.14159;
let b: f32 = 2.71828;
let c = a * b;  // Uses hardware fmul.s instruction
```

## Resources Needed

1. **BL808 Datasheet** - Memory map, peripheral addresses, clock tree
2. **E907 Core Manual** - TCM configuration, FPU registers, CSRs
3. **Bouffalo Lab SDK** - Existing linker scripts and initialization code
4. **OpenOCD Config** - JTAG configuration for BL808 E907 core
5. **Hardware Sample** - For testing and validation

## Next Steps

1. Obtain BL808 datasheet and technical documentation
2. Extract memory map and peripheral addresses
3. Update `linker.ld` with correct memory regions
4. Implement UART driver based on register layout
5. Configure interrupt controller (PLIC/CLINT)
6. Set up FPU initialization sequence
7. Set up OpenOCD for hardware debugging
8. Test on actual BL808 hardware with floating-point benchmarks

## Related Boards

- **bl808-e902**: Low-power M0 core (riscv32emc) for auxiliary tasks
- **bl808-c906**: High-performance 64-bit core (riscv64imac) for Linux
