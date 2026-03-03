use hal::{InterruptController, Platform, SerialPort};
use super::{uart_bl808::Bl808Uart, interrupt::Bl808InterruptController};

pub struct Bl808C906Platform {
    uart: Bl808Uart,
    interrupt_controller: Bl808InterruptController,
}

impl Bl808C906Platform {
    pub const fn new() -> Self {
        Self {
            // TODO: Replace with actual UART base address
            uart: Bl808Uart::new(0x0000_0000),
            interrupt_controller: Bl808InterruptController::new(),
        }
    }
}

impl Platform for Bl808C906Platform {
    type Serial = Bl808Uart;
    type Interrupts = Bl808InterruptController;

    fn init(&mut self) {
        // TODO: Platform-level initialization for C906
        // - Configure clocks (C906 runs at highest frequency)
        // - Set up MMU/virtual memory if needed
        // - Initialize cache (I-cache, D-cache)
        // - Set up memory regions (may have more DRAM than E902/E907)
        // - Initialize power management
        // - Configure privilege modes (M-mode vs S-mode)
    }

    fn serial_port(&mut self) -> &mut Self::Serial {
        &mut self.uart
    }

    fn interrupt_controller(&mut self) -> &mut Self::Interrupts {
        &mut self.interrupt_controller
    }

    fn board_name(&self) -> &str {
        "BL808 C906 (riscv64imac)"
    }

    fn cpu_frequency_hz(&self) -> usize {
        // TODO: Determine actual C906 frequency
        // C906 typically runs at highest frequency
        // Placeholder value - often 480MHz or higher
        480_000_000
    }
}
