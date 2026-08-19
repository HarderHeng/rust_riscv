# BL808 C906 Board Support (riscv64imac)

## Overview

This crate provides bare-metal support for the **BL808 C906 core** - the high-performance 64-bit RISC-V processor on the Bouffalo Lab BL808 SoC, designed primarily for running Linux but capable of bare-metal execution.

**Status**: PARTIAL - UART/PLIC mappings implemented from Bouffalo SDK; boot/linker/cache integration remains

**Target**: `riscv64imac-unknown-none-elf`
- **I**: Integer base instruction set (64-bit)
- **M**: Integer multiplication/division
- **A**: Atomic instructions
- **C**: Compressed instructions (16-bit)

## Core Specifications

- **Architecture**: RISC-V RV64IMAC (T-Head C906)
- **Vendor**: T-Head (Alibaba Group)
- **Purpose**: High-performance application processor, typically runs Linux
- **Memory**: Large DRAM (often 16MB+), no TCM
- **Cache**: I-cache and D-cache (sizes TBD)
- **Typical Use Cases**: Linux, embedded applications, multimedia processing

## SDK-Derived Hardware Facts

- D0 RAM: `0x3EF8_0000..0x3F08_8000` (512 KiB DRAM + 32 KiB VRAM)
- UART3: `0x3000_2000`, IRQ `20`, GPIO16/17 with SDK GPIO function 21
- UART: Bouffalo FIFO UART, DSP XCLK-based 2,000,000 baud 8N1
- C906 PLIC: `0xE000_0000`; M-mode enable `+0x2000`, threshold `+0x200000`, claim/complete `+0x200004`

The remaining platform work is the SDK image header/XIP linker integration,
cache/MMU startup and a C906-compatible trap/vector path.

## What's Needed

To complete this board support package, we need:

### 1. Memory Map
- [ ] Main RAM base address and size (typically 16MB or more)
- [ ] MMIO peripheral region addresses
- [ ] Shared memory regions with E902 and E907 cores
- [ ] Cache-coherent memory regions (if multi-core cache coherency supported)
- [ ] Reserved regions (for bootloader, OpenSBI, etc.)

**Resources**: BL808 datasheet, C906 technical reference manual

### 2. C906-Specific Features
- [ ] T-Head custom extensions (if any)
- [ ] Cache configuration registers (I-cache, D-cache)
- [ ] MMU/TLB configuration (for potential Linux support)
- [ ] Performance monitoring unit (PMU) registers
- [ ] Power management features

### 3. UART Configuration
- [ ] UART peripheral base address (likely shared with E902/E907)
- [ ] Register layout (16550-compatible? Custom?)
- [ ] Clock source and baud rate calculation
- [ ] Pin muxing requirements
- [ ] DMA support for high-speed transfers

### 4. Interrupt Controller
- [ ] PLIC base address
- [ ] Number of external interrupt sources
- [ ] Interrupt priority levels
- [ ] Core-local interrupts (CLINT) base address
- [ ] Inter-core interrupt mechanism (C906 ↔ E902 and C906 ↔ E907)
- [ ] S-mode interrupt delegation (for potential Linux support)

### 5. Clock Configuration
- [ ] C906 core clock frequency (typically highest on SoC)
- [ ] Peripheral clock domains
- [ ] PLL configuration registers
- [ ] Clock gating controls

### 6. Build and Debug Tools
- [ ] OpenOCD configuration for BL808 C906 (T-Head core)
- [ ] JTAG/SWD adapter setup
- [ ] Flash programming procedure
- [ ] GDB stub configuration with 64-bit support
- [ ] T-Head vendor tools compatibility

### 7. Startup Sequence
- [ ] Boot process from reset (M-mode entry point)
- [ ] Clock initialization requirements
- [ ] Cache initialization (enable I-cache, D-cache)
- [ ] MMU setup (if needed for bare-metal)
- [ ] Memory initialization
- [ ] Stack and heap placement strategy
- [ ] Transition to S-mode (if supporting Linux compatibility)

## Current Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Cargo.toml | ✅ Done | Dependencies on kernel, hal, riscv-common |
| .cargo/config.toml | ✅ Done | Target: riscv64imac-unknown-none-elf |
| linker.ld | ✅ Partial | SDK memory regions are ported; image header/XIP boot wrapper remains |
| src/main.rs | ✅ Done | Entry point with todo!() placeholders |
| src/hal_impl/uart_bl808.rs | ✅ Partial | SDK-derived FIFO, clock, GPIO and RX interrupt access |
| src/hal_impl/interrupt.rs | ✅ Partial | SDK-derived C906 PLIC register access |
| src/hal_impl/platform.rs | ✅ Partial | SDK-derived UART3 base, IRQ and GPIO mapping |
| openocd-runner.sh | ⚠️ Placeholder | Need OpenOCD config for T-Head core |

## Building

```bash
cd crates/boards/bl808-c906
cargo build
```

**Note**: Workspace checking passes. Hardware execution still requires the
BL808 image header, boot flow, linker and cache/MMU integration.

## T-Head C906 Core Notes

The C906 is a **T-Head (Alibaba) custom RISC-V core**, not a SiFive or standard design. This means:

1. **Custom Extensions**: May include T-Head-specific ISA extensions
2. **Cache Design**: T-Head custom cache controller
3. **Debugging**: May require T-Head-specific debug tools
4. **Linux Support**: Primary use case is running Linux with OpenSBI

### Linux vs Bare-Metal

While the C906 is designed for Linux, bare-metal is possible:
- **M-mode**: Direct hardware access (what this crate targets)
- **S-mode**: Supervisor mode (requires M-mode firmware like OpenSBI)
- **U-mode**: User mode (requires OS)

For bare-metal, we operate in M-mode with full hardware control.

## Architecture Differences (64-bit)

Key differences from the 32-bit E902/E907 cores:

1. **Pointer Size**: 64-bit pointers (8 bytes vs 4 bytes)
2. **Register Width**: 64-bit general-purpose registers (x0-x31)
3. **Address Space**: Can address much larger memory
4. **Performance**: Higher clock speed and more powerful pipeline
5. **Memory**: Typically has more DRAM available

## Resources Needed

1. **BL808 Datasheet** - Memory map, peripheral addresses, clock tree
2. **T-Head C906 Manual** - Core architecture, custom extensions, cache
3. **Bouffalo Lab SDK** - Existing linker scripts and initialization code
4. **OpenOCD Config** - JTAG configuration for T-Head C906
5. **Hardware Sample** - For testing and validation
6. **OpenSBI** - If supporting S-mode or Linux compatibility

## Next Steps

1. Obtain BL808 datasheet and T-Head C906 technical documentation
2. Extract memory map and peripheral addresses
3. Update `linker.ld` with correct memory regions
4. Implement UART driver based on register layout
5. Configure interrupt controller (PLIC/CLINT)
6. Set up cache initialization sequence
7. Set up OpenOCD for hardware debugging (T-Head tools)
8. Test on actual BL808 hardware
9. Validate 64-bit pointer handling
10. Consider MMU setup for advanced memory management

## Related Boards

- **bl808-e902**: Low-power M0 core (riscv32emc) for auxiliary tasks
- **bl808-e907**: Application core (riscv32imafc) with hardware FPU

## Multi-Core Communication

The BL808 is a heterogeneous multi-core SoC with three different RISC-V cores. For inter-core communication:

- [ ] Shared memory protocol with E902/E907
- [ ] Inter-processor interrupts (IPI) mechanism
- [ ] Mailbox hardware (if present)
- [ ] Cache coherency considerations
