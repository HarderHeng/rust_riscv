//! BL808 E907 platform definition (placeholder).

use hal::{MemoryLayout, Platform};

use super::{interrupt::Bl808InterruptController, uart_bl808::Bl808Uart};

/// Platform implementation for the BL808 E907 (riscv32imacf, M4F application core).
pub struct Bl808E907Platform {
    uart: Bl808Uart,
    interrupt_controller: Bl808InterruptController,
}

impl Bl808E907Platform {
    /// Creates a new E907 platform instance.
    pub const fn new() -> Self {
        Self {
            // TODO: Replace with actual UART base address
            uart: Bl808Uart::new(0x0000_0000),
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
        // TODO: Determine actual UART IRQ number
        0
    }

    fn name(&self) -> &'static str {
        "BL808 E907 (riscv32imacf)"
    }

    fn arch(&self) -> &'static str {
        "riscv32imacf"
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
