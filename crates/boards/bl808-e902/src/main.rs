#![no_std]
#![no_main]

use hal::{InterruptController, Platform, SerialPort};

mod hal_impl;
use hal_impl::Bl808E902Platform;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // Initialize platform
    let mut platform = Bl808E902Platform::new();
    platform.init();

    // Get serial port
    let mut uart = platform.serial_port();
    uart.init();
    uart.write_str("BL808 E902 (riscv32emc) - M0 Core\r\n");
    uart.write_str("Platform: PLACEHOLDER - Awaiting memory map and peripheral definitions\r\n");

    // Main kernel loop
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // TODO: Print panic info when UART is available
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
