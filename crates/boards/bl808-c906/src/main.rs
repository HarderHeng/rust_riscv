#![no_std]
#![no_main]

mod hal_impl;
mod startup;

use core::panic::PanicInfo;
use hal::{InterruptController, Platform, SerialPort};
use hal_impl::Bl808C906Platform;

/// Global platform instance.
static PLATFORM: Bl808C906Platform = Bl808C906Platform::new();

/// Kernel entry point called from riscv-common's `_start`.
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    PLATFORM.console().init();
    kernel::identity::register_platform_identity(PLATFORM.name(), PLATFORM.arch());
    PLATFORM
        .console()
        .puts("BL808 C906 (riscv64imac) - High-Performance 64-bit Core\r\n");
    PLATFORM
        .console()
        .puts("Platform: SDK-derived UART mapping; boot/linker integration pending\r\n");
    PLATFORM
        .console()
        .puts("Note: This core typically runs Linux, but can run bare-metal kernels\r\n");

    kernel::trap::register_claim_handler(|| PLATFORM.interrupt_controller().claim().unwrap_or(0));
    kernel::trap::register_complete_handler(|irq| {
        PLATFORM.interrupt_controller().complete(irq);
    });

    let controller = PLATFORM.interrupt_controller();
    controller.set_threshold(0);
    controller.set_priority(PLATFORM.console_irq(), 1);
    controller.enable_irq(PLATFORM.console_irq());
    PLATFORM.console().enable_rx_interrupt();
    kernel::trap::enable_external_interrupts();
    kernel::trap::enable_global_interrupts();

    let io = kernel::PlatformIO::new(&PLATFORM);
    let mut shell = kernel::shell::Shell::new(io, kernel::shell::commands::COMMANDS, "c906> ");
    shell.run()
}

/// Placeholder panic handler that halts the core.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
