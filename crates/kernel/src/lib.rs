//! Platform-agnostic kernel crate for bare-metal RISC-V systems.
//!
//! This crate provides the core kernel functionality that is independent
//! of specific hardware platforms. It uses the HAL (Hardware Abstraction Layer)
//! traits to interact with hardware in a portable way.
//!
//! # Architecture
//!
//! The kernel is built around the [`Platform`] trait from the HAL crate.
//! Board-specific implementations provide concrete types that implement
//! this trait, and the kernel uses those to access hardware.
//!
//! # Main Components
//!
//! - **Shell**: Interactive command-line interface ([`shell`] module)
//! - **Trap handling**: Interrupt and exception dispatcher ([`trap`] module)
//! - **Platform I/O**: Wrapper that adapts Platform to ShellIO ([`PlatformIO`])
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use kernel::kernel_main;
//! use my_board::QemuVirtPlatform;
//!
//! static PLATFORM: QemuVirtPlatform = QemuVirtPlatform::new();
//!
//! #[no_mangle]
//! extern "C" fn main() -> ! {
//!     kernel_main(&PLATFORM)
//! }
//! ```

#![no_std]
#![deny(missing_docs)]

pub mod shell;
pub mod trap;

use hal::{Platform, SerialPort};
use shell::shell::{Shell, ShellIO, Command};

/// Platform I/O adapter that implements ShellIO using the Platform trait.
///
/// This struct wraps a reference to a Platform and provides the ShellIO
/// interface needed by the shell system. It translates ShellIO calls
/// into HAL Platform calls.
pub struct PlatformIO<'a, P: Platform> {
    platform: &'a P,
}

impl<'a, P: Platform> PlatformIO<'a, P> {
    /// Creates a new PlatformIO wrapper for the given platform.
    pub fn new(platform: &'a P) -> Self {
        Self { platform }
    }
}

impl<P: Platform> ShellIO for PlatformIO<'_, P> {
    fn read_byte(&mut self) -> Option<u8> {
        self.platform.console().try_getc()
    }

    fn write_byte(&mut self, byte: u8) {
        self.platform.console().putc(byte);
    }

    fn write_str(&mut self, s: &str) {
        self.platform.console().puts(s);
    }
}

/// Main kernel entry point.
///
/// This function initializes the kernel subsystems and starts the
/// interactive shell. It never returns.
///
/// # Initialization Sequence
///
/// 1. Initialize console
/// 2. Initialize trap handling (set mtvec)
/// 3. Enable console RX interrupt
/// 4. Enable machine external interrupts and global interrupts
/// 5. Start the shell with registered commands
///
/// # Platform Responsibilities
///
/// Before calling this function, the board code must:
/// - Register interrupt claim/complete handlers via `trap::register_claim_handler()`
///   and `trap::register_complete_handler()`
/// - Register any platform-specific IRQ handlers
/// - Configure the interrupt controller (priorities, thresholds, enable IRQs)
///
/// # Type Parameters
///
/// - `P`: Platform type implementing the [`Platform`] trait
///
/// # Arguments
///
/// - `platform`: Reference to the platform implementation
/// - `commands`: Command registry for the shell (must be sorted by name)
/// - `prompt`: Shell prompt string
pub fn kernel_main<P: Platform>(
    platform: &'static P,
    commands: &'static [Command],
    prompt: &'static str,
) -> ! {
    // Initialize console
    platform.console().init();
    platform.console().puts("\r\n");
    platform.console().puts("=================================\r\n");
    platform.console().puts("  Bare-Metal RISC-V Kernel\r\n");
    platform.console().puts("=================================\r\n");
    platform.console().puts("\r\n");

    // Initialize trap handling
    trap::init();
    platform.console().puts("[KERNEL] Trap handler initialized\r\n");

    // Enable console RX interrupt
    platform.console().enable_rx_interrupt();
    platform.console().puts("[KERNEL] Console RX interrupt enabled\r\n");

    // Enable machine external interrupts (MEI) and global interrupts
    trap::enable_external_interrupts();
    trap::enable_global_interrupts();
    platform.console().puts("[KERNEL] Interrupts enabled\r\n");
    platform.console().puts("\r\n");

    // Create platform I/O adapter
    let io = PlatformIO::new(platform);

    // Create and start shell
    let mut shell = Shell::new(io, commands, prompt);
    shell.run()
}
