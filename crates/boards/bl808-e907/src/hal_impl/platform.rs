//! BL808 E907 platform definition.

use hal::{MemoryLayout, Platform};

use super::{interrupt::Bl808InterruptController, uart_bl808::Bl808Uart};

/// BL808 UART1 base address from the Bouffalo SDK.
pub const UART1_BASE: usize = 0x2000_A100;
/// BL808 E907 UART1 interrupt number from the SDK's LP vector table.
pub const UART1_IRQ: u32 = 45;

/// Platform implementation for the BL808 E907 (riscv32imafc, M4F application core).
pub struct Bl808E907Platform {
    uart: Bl808Uart,
    interrupt_controller: Bl808InterruptController,
}

impl Bl808E907Platform {
    /// Creates a new E907 platform instance.
    pub const fn new() -> Self {
        Self {
            uart: Bl808Uart::new_mcu(UART1_BASE, 17, 18, 6, 19, 7),
            interrupt_controller: Bl808InterruptController::new(),
        }
    }
}

impl Platform for Bl808E907Platform {
    type Serial = Bl808Uart;
    type Interrupt = Bl808InterruptController;

    fn console(&self) -> &Self::Serial {
        &self.uart
    }

    fn interrupt_controller(&self) -> &Self::Interrupt {
        &self.interrupt_controller
    }

    fn console_irq(&self) -> u32 {
        UART1_IRQ
    }

    fn name(&self) -> &'static str {
        "BL808 E907 (riscv32imafc)"
    }

    fn arch(&self) -> &'static str {
        "riscv32imafc"
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
