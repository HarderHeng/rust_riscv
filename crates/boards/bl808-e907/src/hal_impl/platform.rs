use hal::{InterruptController, Platform, SerialPort};
use super::{uart_bl808::Bl808Uart, interrupt::Bl808InterruptController};

pub struct Bl808E907Platform {
    uart: Bl808Uart,
    interrupt_controller: Bl808InterruptController,
}

impl Bl808E907Platform {
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
    type Interrupts = Bl808InterruptController;

    fn init(&mut self) {
        // TODO: Platform-level initialization
        // - Configure clocks (E907 may run at higher frequency than E902)
        // - Enable hardware FPU
        // - Set up memory regions
        // - Initialize power management
    }

    fn serial_port(&mut self) -> &mut Self::Serial {
        &mut self.uart
    }

    fn interrupt_controller(&mut self) -> &mut Self::Interrupts {
        &mut self.interrupt_controller
    }

    fn board_name(&self) -> &str {
        "BL808 E907 (riscv32imacf)"
    }

    fn cpu_frequency_hz(&self) -> usize {
        // TODO: Determine actual E907 frequency
        // E907 typically runs faster than E902
        // Placeholder value
        400_000_000
    }
}
