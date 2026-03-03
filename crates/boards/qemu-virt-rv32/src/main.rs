//! QEMU virt-rv32 board entry point.
//!
//! This binary crate ties together:
//! - The platform-agnostic kernel
//! - The HAL trait implementations for QEMU virt
//! - RISC-V startup code
//! - Board-specific configuration

#![no_std]
#![no_main]

mod hal_impl;
mod platform;
mod startup;

use platform::QemuVirtPlatform;

// ---------------------------------------------------------------------------
// Global platform instance
// ---------------------------------------------------------------------------

/// Global static platform instance.
///
/// This is safe because:
/// - QemuVirtPlatform is Sync (can be shared between threads/contexts)
/// - All hardware access uses volatile operations
/// - QEMU virt is a single-core machine in this configuration
static PLATFORM: QemuVirtPlatform = QemuVirtPlatform::new();

// ---------------------------------------------------------------------------
// Shell commands (imported from current src/)
// ---------------------------------------------------------------------------

// Use the platform-independent command set from kernel
use kernel::shell::commands;

// ---------------------------------------------------------------------------
// Kernel entry point
// ---------------------------------------------------------------------------

/// Kernel entry point called from startup code.
///
/// This is called after:
/// - Stack pointer is initialized
/// - BSS section is zeroed
///
/// This function never returns.
#[no_mangle]
extern "C" fn kernel_main() -> ! {
    use hal::{InterruptController, Platform, SerialPort};

    // Early console init for startup messages
    PLATFORM.console().init();
    PLATFORM.console().puts("\r\n[BOARD] QEMU virt-rv32 initializing...\r\n");

    // Register PLIC claim/complete handlers with the trap system
    kernel::trap::register_claim_handler(|| {
        // PLIC returns Option<u32>, convert to u32 (0 means no interrupt)
        PLATFORM.interrupt_controller().claim().unwrap_or(0)
    });

    kernel::trap::register_complete_handler(|irq| {
        PLATFORM.interrupt_controller().complete(irq);
    });

    // Hand off to kernel
    kernel::kernel_main(&PLATFORM, commands::COMMANDS, "riscv32> ")
}

// ---------------------------------------------------------------------------
// Panic handler
// ---------------------------------------------------------------------------

use core::panic::PanicInfo;

/// Panic handler that prints panic information to the console.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use hal::{Platform, SerialPort};

    PLATFORM.console().puts("\r\n\r\n");
    PLATFORM.console().puts("!!! KERNEL PANIC !!!\r\n");

    if let Some(location) = info.location() {
        PLATFORM.console().puts("Location: ");
        PLATFORM.console().puts(location.file());
        PLATFORM.console().puts(":");
        // Note: We can't easily print line number without alloc, so skip it
        PLATFORM.console().puts("\r\n");
    }

    // In newer Rust versions, message() returns PanicMessage directly, not Option
    let _msg = info.message();
    PLATFORM.console().puts("Message: <panic occurred>\r\n");

    PLATFORM.console().puts("\r\nSystem halted.\r\n");

    // Halt forever
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
