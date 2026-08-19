//! BL808 C906 platform definition.

use hal::{MemoryLayout, Platform};

use super::{interrupt::Bl808InterruptController, uart_bl808::Bl808Uart};

/// BL808 D0 UART3 base address from the Bouffalo SDK.
pub const UART3_BASE: usize = 0x3000_2000;
/// BL808 C906 UART3 interrupt number from the SDK's D0 vector table.
pub const UART3_IRQ: u32 = 20;

/// Platform implementation for the BL808 C906 (riscv64imac, high-performance core).
pub struct Bl808C906Platform {
    uart: Bl808Uart,
    interrupt_controller: Bl808InterruptController,
}

impl Bl808C906Platform {
    /// Creates a new C906 platform instance.
    pub const fn new() -> Self {
        Self {
            uart: Bl808Uart::new_dsp(UART3_BASE, 16, 17),
            interrupt_controller: Bl808InterruptController::new(),
        }
    }
}

impl Platform for Bl808C906Platform {
    type Serial = Bl808Uart;
    type Interrupt = Bl808InterruptController;

    fn console(&self) -> &Self::Serial {
        &self.uart
    }

    fn interrupt_controller(&self) -> &Self::Interrupt {
        &self.interrupt_controller
    }

    fn console_irq(&self) -> u32 {
        UART3_IRQ
    }

    fn name(&self) -> &'static str {
        "BL808 C906 (riscv64imac)"
    }

    fn arch(&self) -> &'static str {
        "riscv64imac"
    }

    fn reboot(&self) -> ! {
        // TODO: Implement BL808 reboot sequence
        loop {
            unsafe { core::arch::asm!("wfi") }
        }
    }

    fn memory_layout(&self) -> MemoryLayout {
        // TODO: Populate from linker script symbols once defined
        MemoryLayout::new(0..0, 0..0, 0..0, 0..0, 0..0)
    }
}
