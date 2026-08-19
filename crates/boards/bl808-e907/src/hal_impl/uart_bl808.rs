//! BL808 UART driver based on the Bouffalo SDK UART register map.

use core::ptr::{read_volatile, write_volatile};

use hal::SerialPort;

const GLB_BASE: usize = 0x2000_0000;
const HBN_BASE: usize = 0x2000_F000;
const HBN_GLB_OFFSET: usize = 0x30;
const GLB_CGEN1: usize = GLB_BASE + 0x584;
const GLB_UART_CFG0: usize = GLB_BASE + 0x150;
const GLB_UART_CFG1: usize = GLB_BASE + 0x154;
const GLB_UART_CFG2: usize = GLB_BASE + 0x158;

const UTX_CONFIG: usize = 0x00;
const URX_CONFIG: usize = 0x04;
const BIT_PRD: usize = 0x08;
const DATA_CONFIG: usize = 0x0C;
const INT_MASK: usize = 0x24;
const FIFO_CONFIG_0: usize = 0x80;
const FIFO_CONFIG_1: usize = 0x84;
const FIFO_WDATA: usize = 0x88;
const FIFO_RDATA: usize = 0x8C;

const UTX_EN: u32 = 1 << 0;
const UTX_FRM_EN: u32 = 1 << 2;
const UTX_PRT_EN: u32 = 1 << 4;
const UTX_PRT_SEL: u32 = 1 << 5;
const UTX_BIT_CNT_D_MASK: u32 = 0x7 << 8;
const UTX_BIT_CNT_P_MASK: u32 = 0x3 << 11;
const URX_EN: u32 = 1 << 0;
const URX_PRT_EN: u32 = 1 << 4;
const URX_PRT_SEL: u32 = 1 << 5;
const URX_BIT_CNT_D_MASK: u32 = 0x7 << 8;

const UART_INT_URX_FIFO_MASK: u32 = 1 << 3;
const UART_INT_URX_RTO_MASK: u32 = 1 << 4;
const UART_DMA_TX_EN: u32 = 1 << 0;
const UART_DMA_RX_EN: u32 = 1 << 1;
const UART_TX_FIFO_CLR: u32 = 1 << 2;
const UART_RX_FIFO_CLR: u32 = 1 << 3;
const UART_TX_FIFO_CNT_MASK: u32 = 0x3F;
const UART_RX_FIFO_CNT_MASK: u32 = 0x3F << 8;

const UART_CLOCK_HZ: u64 = 40_000_000;
const UART_BAUD_RATE: u64 = 2_000_000;

/// A handle to one BL808 MCU UART peripheral.
pub struct Bl808Uart {
    base_addr: usize,
    clock_gate_bit: u8,
    tx_pin: u8,
    tx_signal: u8,
    rx_pin: u8,
    rx_signal: u8,
}

impl Bl808Uart {
    /// Creates an MCU UART using the SDK's XCLK clock path.
    pub const fn new_mcu(
        base_addr: usize,
        clock_gate_bit: u8,
        tx_pin: u8,
        tx_signal: u8,
        rx_pin: u8,
        rx_signal: u8,
    ) -> Self {
        Self {
            base_addr,
            clock_gate_bit,
            tx_pin,
            tx_signal,
            rx_pin,
            rx_signal,
        }
    }

    #[inline]
    fn read(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.base_addr + offset) as *const u32) }
    }

    #[inline]
    fn write(&self, offset: usize, value: u32) {
        unsafe { write_volatile((self.base_addr + offset) as *mut u32, value) }
    }

    fn configure_clock(&self) {
        let mut cgen = unsafe { read_volatile(GLB_CGEN1 as *const u32) };
        cgen |= 1 << self.clock_gate_bit;
        unsafe { write_volatile(GLB_CGEN1 as *mut u32, cgen) };

        let hbn_addr = HBN_BASE + HBN_GLB_OFFSET;
        let mut hbn = unsafe { read_volatile(hbn_addr as *const u32) };
        hbn |= 1 << 0;
        hbn &= !(1 << 2);
        hbn |= 1 << 15;
        unsafe { write_volatile(hbn_addr as *mut u32, hbn) };

        let mut cfg = unsafe { read_volatile(GLB_UART_CFG0 as *const u32) };
        cfg &= !(0x7 | (1 << 4));
        unsafe { write_volatile(GLB_UART_CFG0 as *mut u32, cfg) };
        cfg |= 1 << 4;
        unsafe { write_volatile(GLB_UART_CFG0 as *mut u32, cfg) };
    }

    fn configure_signal(&self, pin: u8, signal: u8) {
        let index = (pin % 12) as usize;
        let (address, slot) = if index < 8 {
            (GLB_UART_CFG1, index)
        } else {
            (GLB_UART_CFG2, index - 8)
        };
        let shift = slot * 4;
        let mut value = unsafe { read_volatile(address as *const u32) };
        value = (value & !(0xF << shift)) | ((signal as u32) << shift);
        unsafe { write_volatile(address as *mut u32, value) };
    }

    fn configure_gpio(&self, pin: u8) {
        let address = GLB_BASE + 0x8C4 + (pin as usize) * 4;
        let value = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 4) | (7 << 8) | (1 << 22) | (1 << 30);
        unsafe { write_volatile(address as *mut u32, value) };
    }
}

impl SerialPort for Bl808Uart {
    fn init(&self) {
        self.configure_clock();
        self.configure_signal(self.tx_pin, self.tx_signal);
        self.configure_signal(self.rx_pin, self.rx_signal);
        self.configure_gpio(self.tx_pin);
        self.configure_gpio(self.rx_pin);

        let mut tx = self.read(UTX_CONFIG) & !UTX_EN;
        let mut rx = self.read(URX_CONFIG) & !URX_EN;
        self.write(UTX_CONFIG, tx);
        self.write(URX_CONFIG, rx);

        let divisor = (UART_CLOCK_HZ * 10 / UART_BAUD_RATE + 5) / 10;
        let bit_period = (divisor - 1) as u32;
        self.write(BIT_PRD, bit_period | (bit_period << 16));

        tx &= !(UTX_PRT_EN | UTX_PRT_SEL | UTX_BIT_CNT_D_MASK | UTX_BIT_CNT_P_MASK);
        tx |= UTX_FRM_EN | (7 << 8) | (1 << 11);
        rx &= !(URX_PRT_EN | URX_PRT_SEL | URX_BIT_CNT_D_MASK);
        rx |= 7 << 8;
        self.write(UTX_CONFIG, tx);
        self.write(URX_CONFIG, rx);
        self.write(DATA_CONFIG, self.read(DATA_CONFIG) & !1);

        let mut fifo = self.read(FIFO_CONFIG_1);
        fifo &= !((0x1F << 16) | (0x1F << 24));
        fifo |= (7 << 16) | (7 << 24);
        self.write(FIFO_CONFIG_1, fifo);

        let mut fifo = self.read(FIFO_CONFIG_0);
        fifo &= !(UART_DMA_TX_EN | UART_DMA_RX_EN);
        fifo |= UART_TX_FIFO_CLR | UART_RX_FIFO_CLR;
        self.write(FIFO_CONFIG_0, fifo);
        self.write(INT_MASK, u32::MAX);

        tx |= UTX_EN;
        rx |= URX_EN;
        self.write(UTX_CONFIG, tx);
        self.write(URX_CONFIG, rx);
    }

    fn putc(&self, byte: u8) {
        while self.read(FIFO_CONFIG_1) & UART_TX_FIFO_CNT_MASK == 0 {}
        unsafe {
            write_volatile((self.base_addr + FIFO_WDATA) as *mut u8, byte);
        }
    }

    fn try_getc(&self) -> Option<u8> {
        if self.read(FIFO_CONFIG_1) & UART_RX_FIFO_CNT_MASK == 0 {
            return None;
        }
        Some(unsafe { read_volatile((self.base_addr + FIFO_RDATA) as *const u8) })
    }

    fn enable_rx_interrupt(&self) {
        let mut mask = self.read(INT_MASK);
        mask &= !(UART_INT_URX_FIFO_MASK | UART_INT_URX_RTO_MASK);
        self.write(INT_MASK, mask);
    }

    fn disable_rx_interrupt(&self) {
        let mut mask = self.read(INT_MASK);
        mask |= UART_INT_URX_FIFO_MASK | UART_INT_URX_RTO_MASK;
        self.write(INT_MASK, mask);
    }
}
