//! BL808 E902 platform definition.

use hal::{MemoryLayout, Platform};

use super::{interrupt::Bl808InterruptController, uart_bl808::Bl808Uart};

/// BL808 UART0 base address from the Bouffalo SDK.
pub const UART0_BASE: usize = 0x2000_A000;
/// BL808 E902 UART0 interrupt number from the SDK's M0 vector table.
pub const UART0_IRQ: u32 = 44;

/// Platform implementation for the BL808 E902 (riscv32emc, M0 low-power core).
pub struct Bl808E902Platform {
    uart: Bl808Uart,
    interrupt_controller: Bl808InterruptController,
}

impl Bl808E902Platform {
    /// Creates a new E902 platform instance.
    pub const fn new() -> Self {
        Self {
            uart: Bl808Uart::new_mcu(UART0_BASE, 16, 14, 2, 15, 3),
            interrupt_controller: Bl808InterruptController::new(),
        }
    }
}

impl Platform for Bl808E902Platform {
    type Serial = Bl808Uart;
    type Interrupt = Bl808InterruptController;

    fn console(&self) -> &Self::Serial {
        &self.uart
    }

    fn interrupt_controller(&self) -> &Self::Interrupt {
        &self.interrupt_controller
    }

    fn console_irq(&self) -> u32 {
        UART0_IRQ
    }

    fn name(&self) -> &'static str {
        "BL808 E902 (riscv32emc)"
    }

    fn arch(&self) -> &'static str {
        "riscv32emc"
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
