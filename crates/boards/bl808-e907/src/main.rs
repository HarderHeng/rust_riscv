#![no_std]
#![no_main]

mod hal_impl;
mod startup;

use core::panic::PanicInfo;
use hal::{InterruptController, Platform, SerialPort};
use hal_impl::Bl808E907Platform;

/// Global platform instance.
static PLATFORM: Bl808E907Platform = Bl808E907Platform::new();

/// Kernel entry point called from riscv-common's `_start`.
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    PLATFORM.console().init();
    kernel::identity::register_platform_identity(PLATFORM.name(), PLATFORM.arch());
    PLATFORM
        .console()
        .puts("BL808 E907 (riscv32imafc) - M4F Application Core with Hardware FPU\r\n");
    PLATFORM
        .console()
        .puts("Platform: SDK-derived UART mapping; boot/linker integration pending\r\n");

    let controller = PLATFORM.interrupt_controller();
    controller.set_threshold(0);
    controller.set_priority(PLATFORM.console_irq(), 1);
    controller.enable_irq(PLATFORM.console_irq());
    PLATFORM.console().enable_rx_interrupt();
    kernel::trap::enable_external_interrupts();
    kernel::trap::enable_global_interrupts();

    let io = kernel::PlatformIO::new(&PLATFORM);
    let mut shell = kernel::shell::Shell::new(io, kernel::shell::commands::COMMANDS, "e907> ");
    shell.run()
}

/// Placeholder panic handler that halts the core.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
