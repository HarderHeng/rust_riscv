#![no_std]
#![no_main]

mod hal_impl;

use core::panic::PanicInfo;
use hal::{Platform, SerialPort};
use hal_impl::Bl808E907Platform;

/// Keeps the riscv-common startup object in the final link (`_start`).
///
/// The linker script sets `ENTRY(_start)`, so this symbol must resolve.
#[used]
static _KEEP_RISCV_COMMON: fn() -> (*mut u8, *mut u8) = riscv_common::bss_range;

/// Global platform instance.
static PLATFORM: Bl808E907Platform = Bl808E907Platform::new();

/// Kernel entry point called from riscv-common's `_start`.
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    PLATFORM.console().init();
    PLATFORM
        .console()
        .puts("BL808 E907 (riscv32imacf) - M4F Application Core with Hardware FPU\r\n");
    PLATFORM
        .console()
        .puts("Platform: PLACEHOLDER - Awaiting memory map and peripheral definitions\r\n");

    // Never returns (redline: kernel_main must end in an infinite loop)
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}

/// Placeholder panic handler that halts the core.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
