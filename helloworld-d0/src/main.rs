#![no_std]
#![no_main]

use bouffalo_hal::{prelude::*, uart::Config};
use bouffalo_rt::{Clocks, Peripherals, entry};
use core::ptr::{read_volatile, write_volatile};
use embedded_time::rate::*;
use panic_halt as _;

/// M1s Dock onboard LED, same pin as the C helloworld (GPIO8).
const LED_PIN_NOTE: &str = "gpio8";

/// Boot header `dsp_clk = 0x03` is WiFi PLL 320 MHz. C later switches DSP
/// to 400 MHz; we don't, so D0 stays at 320 MHz. UART still uses XCLK 40 MHz.
const DSP_HZ: u32 = 320_000_000;
const XTAL_HZ: u32 = 40_000_000;

const MM_GLB_BASE: usize = 0x3000_7000;
const MM_CLK_CTRL_CPU: usize = MM_GLB_BASE;
const MM_CLK_CTRL_PERI: usize = MM_GLB_BASE + 0x10;
const DSP_UART_CLK_XCLK: u32 = 2;

const IPC_SYNC_ADDR1: usize = 0x4000_0000;
const IPC_SYNC_ADDR2: usize = 0x4000_0004;
const IPC_SYNC_FLAG: u32 = 0x1234_5678;

/// HAL `Clocks` still hardcodes UART3 as 160 MHz (`clocks/v2.rs` TODO).
/// M0 already set DSP UART0 to XCLK 40 MHz; tell `freerun` the real rate.
struct Uart3Xclk;

impl bouffalo_hal::uart::Clock for Uart3Xclk {
    fn uart_clock<const I: usize>(self) -> Hertz {
        Hertz(XTAL_HZ)
    }
}

#[entry]
fn main(p: Peripherals, _c: Clocks) -> ! {
    enable_uart3_clock();

    let tx = p.gpio.io16.into_mm_uart();
    let rx = p.gpio.io17.into_mm_uart();
    let config = Config::default().set_baudrate(2_000_000.Bd());
    let mut serial = p.uart3.freerun(config, (tx, rx), Uart3Xclk).unwrap();

    write!(serial, "rust helloworld from D0/C906, uart up\r\n").ok();
    serial.flush().ok();

    wait_for_m0();
    write!(serial, "rust helloworld from D0/C906, ipc synced\r\n").ok();
    serial.flush().ok();

    // C `SystemInit` enables I-cache after IPC. Without it, `riscv::asm::delay`
    // fetches every loop iteration from XIP and a "5s" wait lasts minutes.
    enable_icache();
    write!(serial, "rust helloworld from D0/C906, icache on\r\n").ok();
    serial.flush().ok();

    let mut led = p.gpio.io8.into_floating_output();
    let mut led_on = false;
    let mut count: u32 = 0;

    loop {
        led_on = !led_on;
        if led_on {
            led.set_high().ok();
        } else {
            led.set_low().ok();
        }
        count = count.wrapping_add(1);

        write!(
            serial,
            "hello world from D0/C906, count={count}, {LED_PIN_NOTE} led={}\r\n",
            if led_on { "on" } else { "off" }
        )
        .ok();
        serial.flush().ok();

        delay_ms(5000);
    }
}

/// Same as M0: C `GLB_Set_DSP_UART0_CLK(XCLK, div=0)`.
fn enable_uart3_clock() {
    unsafe {
        let cpu = read_volatile(MM_CLK_CTRL_CPU as *const u32);
        write_volatile(
            MM_CLK_CTRL_CPU as *mut u32,
            (cpu & !(0x3 << 4)) | (DSP_UART_CLK_XCLK << 4),
        );

        let peri = read_volatile(MM_CLK_CTRL_PERI as *const u32);
        write_volatile(MM_CLK_CTRL_PERI as *mut u32, peri & !(1 << 16));

        let peri = read_volatile(MM_CLK_CTRL_PERI as *const u32);
        write_volatile(
            MM_CLK_CTRL_PERI as *mut u32,
            (peri & !(0x7 << 17)) | (1 << 16),
        );
    }
}

/// Same handshake as C `system_bl808.c` on D0: wait for M0, then clear flags.
fn wait_for_m0() {
    unsafe {
        while read_volatile(IPC_SYNC_ADDR1 as *const u32) != IPC_SYNC_FLAG
            || read_volatile(IPC_SYNC_ADDR2 as *const u32) != IPC_SYNC_FLAG
        {
            // C notes this is required if D-cache is on.
            dcache_invalidate(IPC_SYNC_ADDR1);
            core::hint::spin_loop();
        }
        write_volatile(IPC_SYNC_ADDR1 as *mut u32, 0);
        write_volatile(IPC_SYNC_ADDR2 as *mut u32, 0);
    }
}

/// C `csi_icache_enable`: invalidate, then set MHCR.IE.
fn enable_icache() {
    unsafe {
        core::arch::asm!("fence", options(nostack));
        core::arch::asm!("fence.i", options(nostack));
        // T-Head `icache.iall` — same word as CSI on E907 (`0x0100000b`).
        core::arch::asm!(".word 0x0100000b", options(nostack));
        let mut mhcr: usize;
        core::arch::asm!("csrr {mhcr}, 0x7C1", mhcr = out(reg) mhcr, options(nostack));
        mhcr |= 1;
        core::arch::asm!("csrw 0x7C1, {mhcr}", mhcr = in(reg) mhcr, options(nostack));
        core::arch::asm!("fence", options(nostack));
        core::arch::asm!("fence.i", options(nostack));
    }
}

/// T-Head `dcache.ipa` (same encoding as C `__CUSTOM_DCACHE_IPA`).
fn dcache_invalidate(addr: usize) {
    unsafe {
        core::arch::asm!(
            ".insn i 0x0b, 0x0, x0, {addr}, 0x2a",
            addr = in(reg) addr,
            options(nostack)
        );
        core::arch::asm!("fence", options(nostack));
    }
}

fn rdcycle() -> u64 {
    let cycles: u64;
    unsafe {
        core::arch::asm!("rdcycle {cycles}", cycles = out(reg) cycles, options(nomem, nostack));
    }
    cycles
}

fn delay_ms(ms: u32) {
    let start = rdcycle();
    let need = DSP_HZ as u64 * ms as u64 / 1000;
    while rdcycle().wrapping_sub(start) < need {
        core::hint::spin_loop();
    }
}
