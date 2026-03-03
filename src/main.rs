#![no_std]
#![no_main]

mod startup;
mod uart;

use core::panic::PanicInfo;
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
    ($($arg:tt)*)   => { $crate::kprint!("{}\r\n", format_args!($($arg)*)) };
}

// ---------------------------------------------------------------------------
// Kernel entry
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    Uart::new(uart::UART0_BASE).init();

    kprintln!("Hello, World!");
    kprintln!("QEMU RISC-V32 bare-metal Rust kernel running.");

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
    kprintln!();
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
