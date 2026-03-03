# BL808 E902 Board Support (riscv32emc)

## Overview

This crate provides bare-metal support for the **BL808 E902 core** - the low-power M0 RISC-V processor on the Bouffalo Lab BL808 SoC.

**Status**: PLACEHOLDER - Awaiting hardware specifications

**Target**: `riscv32emc-unknown-none-elf`
- **E**: Reduced register set (16 registers instead of 32)
- **M**: Integer multiplication/division
- **C**: Compressed instructions (16-bit)

## Core Specifications

- **Architecture**: RISC-V RV32EMC
- **Purpose**: Low-power M0 core for auxiliary tasks
- **Memory**: ITCM (Instruction TCM) + DTCM (Data TCM) + shared RAM
- **Typical Use Cases**: Low-power sensor processing, wake-on-interrupt tasks

## What's Needed

To complete this board support package, we need:

### 1. Memory Map
- [ ] ITCM base address and size
- [ ] DTCM base address and size
- [ ] Shared RAM base address and size
- [ ] MMIO peripheral region addresses
- [ ] Boot ROM location (if any)

**Resources**: BL808 datasheet, E902 technical reference manual

### 2. UART Configuration
- [ ] UART peripheral base address
- [ ] Register layout (16550-compatible? Custom?)
- [ ] Clock source and baud rate calculation
- [ ] Pin muxing requirements
- [ ] DMA support (if available)

### 3. Interrupt Controller
- [ ] PLIC base address (if present)
- [ ] Number of external interrupt sources
- [ ] Interrupt priority levels
- [ ] Core-local interrupts (CLINT) base address
- [ ] Inter-core interrupt mechanism (for E902 ↔ E907 ↔ C906 communication)

### 4. Clock Configuration
- [ ] E902 core clock frequency
- [ ] Peripheral clock domains
- [ ] PLL configuration registers
- [ ] Clock gating controls

### 5. Build and Debug Tools
- [ ] OpenOCD configuration for BL808 E902
- [ ] JTAG/SWD adapter setup
- [ ] Flash programming procedure
- [ ] GDB stub configuration

### 6. Startup Sequence
- [ ] Boot process from reset
- [ ] Clock initialization requirements
- [ ] Memory initialization (TCM enable, RAM init)
- [ ] Stack and heap placement strategy

## Current Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Cargo.toml | ✅ Done | Dependencies on kernel, hal, riscv-common |
| .cargo/config.toml | ✅ Done | Target: riscv32emc-unknown-none-elf |
| linker.ld | ⚠️ Template | PLACEHOLDER memory addresses |
| src/main.rs | ✅ Done | Entry point with todo!() placeholders |
| src/hal_impl/uart_bl808.rs | ⚠️ Stub | todo!() - need UART specs |
| src/hal_impl/interrupt.rs | ⚠️ Stub | todo!() - need PLIC specs |
| src/hal_impl/platform.rs | ⚠️ Stub | Placeholder values |
| openocd-runner.sh | ⚠️ Placeholder | Need OpenOCD config |

## Building

```bash
cd crates/boards/bl808-e902
cargo build
```

**Note**: Build will fail with `todo!()` panics until hardware specifications are provided.

## Resources Needed

1. **BL808 Datasheet** - Memory map, peripheral addresses, clock tree
2. **E902 Core Manual** - TCM configuration, CSR registers, interrupt handling
3. **Bouffalo Lab SDK** - Existing linker scripts and initialization code
4. **OpenOCD Config** - JTAG configuration for BL808 E902 core
5. **Hardware Sample** - For testing and validation

## Next Steps

1. Obtain BL808 datasheet and technical documentation
2. Extract memory map and peripheral addresses
3. Update `linker.ld` with correct memory regions
4. Implement UART driver based on register layout
5. Configure interrupt controller (PLIC/CLINT)
6. Set up OpenOCD for hardware debugging
7. Test on actual BL808 hardware

## Related Boards

- **bl808-e907**: Application core (riscv32imacf) with FPU
- **bl808-c906**: High-performance core (riscv64imac) for Linux
