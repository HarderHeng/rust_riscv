//! QEMU virt-rv32 platform implementation.
//!
//! This module provides the Platform trait implementation for the QEMU virt
//! machine with RISC-V 32-bit (riscv32imac) target.

#[cfg(feature = "virtio-console")]
use crate::hal_impl::VirtioConsole;
use crate::hal_impl::{Plic, Uart16550a};
use hal::{MemoryLayout, Platform, SerialPort};

// ---------------------------------------------------------------------------
// Hardware addresses
// ---------------------------------------------------------------------------

/// UART0 base address.
#[cfg_attr(feature = "virtio-console", allow(dead_code))]
pub const UART0_BASE: usize = 0x1000_0000;

/// UART0 IRQ number.
pub const UART0_IRQ: u32 = 10;

/// First virtio-mmio slot IRQ number used by the optional console device.
#[cfg_attr(not(feature = "virtio-console"), allow(dead_code))]
pub const VIRTIO_CONSOLE_IRQ: u32 = 1;

/// VIRT_TEST device address for system control.
/// Writing certain values here triggers reboot/poweroff.
const VIRT_TEST: usize = 0x100000;

// ---------------------------------------------------------------------------
// Linker symbols
// ---------------------------------------------------------------------------

extern "C" {
    static _stext: u8;
    static _etext: u8;
    static _sdata: u8;
    static _edata: u8;
    static _sbss: u8;
    static _ebss: u8;
    static _stack_bottom: u8;
    static _stack_top: u8;
    static _heap_start: u8;
    static _heap_end: u8;
}

// ---------------------------------------------------------------------------
// QemuVirtPlatform
// ---------------------------------------------------------------------------

/// Console backend selected by the board build feature.
#[cfg_attr(feature = "virtio-console", allow(dead_code))]
pub enum QemuConsole {
    /// QEMU's standard ns16550a UART.
    Uart(Uart16550a),
    /// Modern virtio-console on virtio-mmio slot 0.
    #[cfg(feature = "virtio-console")]
    Virtio(VirtioConsole),
}

impl QemuConsole {
    const fn new() -> Self {
        #[cfg(feature = "virtio-console")]
        {
            return Self::Virtio(VirtioConsole::new(0x1000_1000));
        }

        #[cfg(not(feature = "virtio-console"))]
        {
            Self::Uart(Uart16550a::new(UART0_BASE))
        }
    }

    const fn irq(&self) -> u32 {
        #[cfg(feature = "virtio-console")]
        if matches!(self, Self::Virtio(_)) {
            return VIRTIO_CONSOLE_IRQ;
        }
        UART0_IRQ
    }
}

impl SerialPort for QemuConsole {
    fn init(&self) {
        match self {
            Self::Uart(uart) => uart.init(),
            #[cfg(feature = "virtio-console")]
            Self::Virtio(console) => console.init(),
        }
    }

    fn putc(&self, byte: u8) {
        match self {
            Self::Uart(uart) => uart.putc(byte),
            #[cfg(feature = "virtio-console")]
            Self::Virtio(console) => console.putc(byte),
        }
    }

    fn try_getc(&self) -> Option<u8> {
        match self {
            Self::Uart(uart) => uart.try_getc(),
            #[cfg(feature = "virtio-console")]
            Self::Virtio(console) => console.try_getc(),
        }
    }

    fn enable_rx_interrupt(&self) {
        match self {
            Self::Uart(uart) => uart.enable_rx_interrupt(),
            #[cfg(feature = "virtio-console")]
            Self::Virtio(console) => console.enable_rx_interrupt(),
        }
    }

    fn disable_rx_interrupt(&self) {
        match self {
            Self::Uart(uart) => uart.disable_rx_interrupt(),
            #[cfg(feature = "virtio-console")]
            Self::Virtio(console) => console.disable_rx_interrupt(),
        }
    }
}

/// Platform implementation for QEMU virt-rv32.
pub struct QemuVirtPlatform {
    console: QemuConsole,
    plic: Plic,
}

impl QemuVirtPlatform {
    /// Creates a new QEMU virt platform instance.
    pub const fn new() -> Self {
        Self {
            console: QemuConsole::new(),
            plic: Plic::new(),
        }
    }
}

impl Platform for QemuVirtPlatform {
    type Serial = QemuConsole;
    type Interrupt = Plic;

    fn console(&self) -> &Self::Serial {
        &self.console
    }

    fn interrupt_controller(&self) -> &Self::Interrupt {
        &self.plic
    }

    fn console_irq(&self) -> u32 {
        self.console.irq()
    }

    fn name(&self) -> &'static str {
        "QEMU virt-rv32"
    }

    fn arch(&self) -> &'static str {
        "riscv32imac"
    }

    fn reboot(&self) -> ! {
        // QEMU virt machine: write 0x7777 to VIRT_TEST to trigger reboot
        unsafe {
            core::ptr::write_volatile(VIRT_TEST as *mut u32, 0x7777);
        }
        // If reboot fails, loop forever
        loop {
            unsafe {
                core::arch::asm!("wfi");
            }
        }
    }

    fn memory_layout(&self) -> MemoryLayout {
        unsafe {
            MemoryLayout::new(
                // heap
                &_heap_start as *const u8 as usize..&_heap_end as *const u8 as usize,
                // stack
                &_stack_bottom as *const u8 as usize..&_stack_top as *const u8 as usize,
                // text
                &_stext as *const u8 as usize..&_etext as *const u8 as usize,
                // data
                &_sdata as *const u8 as usize..&_edata as *const u8 as usize,
                // bss
                &_sbss as *const u8 as usize..&_ebss as *const u8 as usize,
            )
        }
    }
}

// QemuVirtPlatform is safe to use as a global static
unsafe impl Send for QemuVirtPlatform {}
unsafe impl Sync for QemuVirtPlatform {}
