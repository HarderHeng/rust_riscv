#![no_std]
#![no_main]

mod plic;
mod startup;
mod trap;
mod uart;

use core::panic::PanicInfo;
use plic::{Plic, UART0_IRQ};
use uart::Uart;

// ---------------------------------------------------------------------------
// Formatted output macros
// ---------------------------------------------------------------------------

/// Print to UART0 without a trailing newline.
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::uart::print(format_args!($($arg)*))
    };
}

/// Print to UART0 with a trailing `\r\n`.
#[macro_export]
macro_rules! kprintln {
    ()              => { $crate::kprint!("\r\n") };
    ($($arg:tt)*)   => {{
        $crate::uart::print(format_args!($($arg)*));
        $crate::uart::print(format_args!("\r\n"));
    }};
}

// ---------------------------------------------------------------------------
// Kernel entry
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // Initialize UART
    let uart = Uart::new(uart::UART0_BASE);
    uart.init();

    kprintln!("Hello, World!");
    kprintln!("QEMU RISC-V32 bare-metal Rust kernel running.");
    kprintln!();

    // Initialize interrupt system
    kprintln!("[INIT] Setting up trap handler...");
    trap::init();

    // Configure PLIC
    kprintln!("[INIT] Configuring PLIC...");
    let plic = Plic::new();
    plic.set_priority(UART0_IRQ, 7);        // Set UART IRQ priority to highest
    plic.enable_irq(0, UART0_IRQ);          // Enable UART IRQ for context 0 (Hart 0 M-mode)
    plic.set_threshold(0, 0);               // Accept all interrupts (threshold = 0)

    // Enable UART RX interrupt
    kprintln!("[INIT] Enabling UART RX interrupt...");
    uart.enable_rx_interrupt();

    // Enable interrupts in CPU
    kprintln!("[INIT] Enabling machine external interrupts...");
    trap::enable_external_interrupts();
    trap::enable_global_interrupts();

    kprintln!();
    kprintln!("Interrupt system initialized. Type to see echo:");
    kprintln!();

    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}

// ---------------------------------------------------------------------------
// Panic handler
// ---------------------------------------------------------------------------

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprint!("\r\n[PANIC]");
    if let Some(loc) = info.location() {
        kprint!(" {}:{}", loc.file(), loc.line());
    }
    kprint!(": {}", info.message());
    kprintln!();
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
