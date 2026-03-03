# riscv-common

Common RISC-V startup code and CSR utilities for RV32 and RV64 architectures.

## Features

### Startup Code

Provides `_start` assembly entry point that:
- Initializes stack pointer
- Zeros BSS section
- Jumps to `kernel_main` function

Automatically selects the correct implementation based on target:
- **RV32I/RV32E**: Uses 32-bit load/store instructions (`lw`/`sw`)
- **RV64I**: Uses 64-bit load/store instructions (`ld`/`sd`)

### CSR Access

Safe macros and helper functions for Control and Status Registers:

#### Macros

```rust
// Read a CSR
let mstatus = unsafe { csr_read!(mstatus) };

// Write a CSR
unsafe { csr_write!(mstatus, 0x1800) };

// Set specific bits
unsafe { csr_set!(mie, 0x8) };

// Clear specific bits
unsafe { csr_clear!(mie, 0x8) };

// Swap CSR value and return old value
let old = unsafe { csr_swap!(mstatus, new_value) };
```

#### Helper Functions

```rust
use riscv_common::csr;

// Enable/disable machine interrupts
unsafe { csr::enable_interrupts() };
unsafe { csr::disable_interrupts() };

// Check interrupt status
let enabled = csr::interrupts_enabled();

// Set trap vector
unsafe { csr::set_trap_vector(handler_addr, false) }; // Direct mode
unsafe { csr::set_trap_vector(handler_addr, true) };  // Vectored mode

// Read trap vector
let (addr, vectored) = csr::get_trap_vector();
```

#### Bit Field Constants

```rust
use riscv_common::csr::{mstatus, mie, mip, mcause};

// Machine Status Register
mstatus::MIE            // Machine Interrupt Enable
mstatus::MPIE           // Machine Previous Interrupt Enable
mstatus::MPP_MACHINE    // Previous privilege: Machine mode

// Machine Interrupt Enable
mie::MSIE               // Machine Software Interrupt Enable
mie::MTIE               // Machine Timer Interrupt Enable
mie::MEIE               // Machine External Interrupt Enable

// Machine Cause codes
mcause::ILLEGAL_INSTRUCTION
mcause::BREAKPOINT
mcause::ECALL_MACHINE
mcause::MACHINE_TIMER_INTERRUPT
```

## Requirements

### Linker Script

Your linker script must define these symbols:

```ld
SECTIONS {
    .text : {
        KEEP(*(.text.start))    /* _start must be first */
        *(.text .text.*)
    }

    .bss : {
        _sbss = .;
        *(.bss .bss.*)
        _ebss = .;
    }

    .data : {
        _sdata = .;
        *(.data .data.*)
        _edata = .;
    }

    _sidata = LOADADDR(.data);

    .stack : {
        . += 0x10000;  /* 64 KiB stack */
        _stack_top = .;
    }

    _heap_start = .;
    _heap_end = /* end of RAM */;
}
```

### Kernel Entry Point

Define a `kernel_main` function that never returns:

```rust
#[no_mangle]
extern "C" fn kernel_main() -> ! {
    // Your kernel initialization here

    loop {
        // Main kernel loop
    }
}
```

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
riscv-common = { path = "../crates/riscv-common" }
```

In your main kernel file:

```rust
#![no_std]
#![no_main]

use riscv_common::csr;

#[no_mangle]
extern "C" fn kernel_main() -> ! {
    // Enable interrupts
    unsafe {
        csr::set_trap_vector(trap_handler as usize, false);
        csr::enable_interrupts();
    }

    loop {
        unsafe { core::arch::asm!("wfi") }; // Wait for interrupt
    }
}

#[no_mangle]
extern "C" fn trap_handler() {
    let cause = unsafe { csr_read!(mcause) };
    // Handle trap...
}
```

## Architecture Support

- **riscv32**: RV32I and RV32E (embedded, 16 registers)
- **riscv64**: RV64I

The crate automatically selects the correct implementation at compile time.

## License

MIT OR Apache-2.0

