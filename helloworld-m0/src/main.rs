#![no_std]
#![no_main]

use bouffalo_hal::{hbn::UartClockSource, prelude::*, uart::Config};
use bouffalo_rt::{Clocks, Peripherals, entry};
use core::ptr::{read_volatile, write_volatile};
use embedded_time::rate::*;
use panic_halt as _;

/// Boot header `mcu_clk = 0x04` is WiFi PLL 320 MHz (not 240).
const MCU_HZ: u32 = 320_000_000;

const D0_FLASH_ADDR: u32 = 0x100000;
const D0_XIP_ENTRY: u32 = 0x58000000;
const TZC_MM_BMX_TZMID: usize = 0x2000_5000 + 0x300;
const TZC_MM_BMX_TZMID_LOCK: usize = 0x2000_5000 + 0x304;
const MM_MISC_CPU0_BOOT: usize = 0x3000_0000;
const MM_GLB_BASE: usize = 0x3000_7000;
const MM_CLK_CTRL_CPU: usize = MM_GLB_BASE;
const MM_CLK_CTRL_PERI: usize = MM_GLB_BASE + 0x10;
const MM_SW_SYS_RESET: usize = MM_GLB_BASE + 0x40;
const MMCPU0_CLK_EN: u32 = 1 << 12;
const MMCPU0_RESET: u32 = 1 << 8;
const DSP_UART_CLK_XCLK: u32 = 2;
const SF_CTRL_ID1_OFFSET: usize = 0x2000_B000 + 0xA4;
const IPC_SYNC_ADDR1: usize = 0x4000_0000;
const IPC_SYNC_ADDR2: usize = 0x4000_0004;
const IPC_SYNC_FLAG: u32 = 0x1234_5678;

#[entry]
fn main(p: Peripherals, mut c: Clocks) -> ! {
    // C `board_init` clocks UART from XCLK 40 MHz. HAL otherwise assumes 80 MHz bclk.
    enable_uart0_clock(&p, &mut c);

    // GPIO8 before UART: if the image runs, the LED moves even when UART is wrong.
    let mut led = p.gpio.io8.into_floating_output();
    led.set_high().ok();

    let tx = p.uart_muxes.sig2.into_transmit(p.gpio.io14);
    let rx = p.uart_muxes.sig3.into_receive(p.gpio.io15);
    let config = Config::default().set_baudrate(2_000_000.Bd());
    let mut serial = p.uart0.freerun(config, (tx, rx), &c).unwrap();

    write!(serial, "rust helloworld from M0/E907\r\n").ok();
    serial.flush().ok();

    enable_icache();
    enable_uart3_clock();
    start_d0_core();
    write!(serial, "rust helloworld from M0/E907, started D0\r\n").ok();
    write!(serial, "led pin=gpio8, period=5s\r\n").ok();
    serial.flush().ok();

    let mut led_on = true;
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
            "hello world from M0/E907, count={count}, gpio8 led={}\r\n",
            if led_on { "on" } else { "off" }
        )
        .ok();
        serial.flush().ok();
        delay_ms(5000);
    }
}

/// C `PERIPHERAL_CLOCK_UART0_ENABLE` + `GLB_Set_UART_CLK(XCLK, div=0)`.
fn enable_uart0_clock(p: &Peripherals, c: &mut Clocks) {
    unsafe {
        p.glb.clock_config_1.modify(|v| v.enable_uart::<0>());
        p.glb.uart_config.modify(|v| v.disable_clock());
        p.hbn
            .global
            .modify(|v| v.set_uart_clock_source(UartClockSource::Xclk));
        p.glb
            .uart_config
            .modify(|v| v.set_clock_divide(0).enable_clock());
    }
    c.uart_source = UartClockSource::Xclk;
    c.uart_divide = 0;
}

/// C `GLB_Set_DSP_UART0_CLK(XCLK, div=0)`. HAL has no DSP UART clock API.
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

/// Same sequence as C `start_d0_core`.
fn start_d0_core() {
    unsafe {
        write_volatile(
            TZC_MM_BMX_TZMID as *mut u32,
            read_volatile(TZC_MM_BMX_TZMID as *const u32) | 1,
        );
        write_volatile(
            TZC_MM_BMX_TZMID_LOCK as *mut u32,
            read_volatile(TZC_MM_BMX_TZMID_LOCK as *const u32) | 1,
        );
        write_volatile(MM_MISC_CPU0_BOOT as *mut u32, D0_XIP_ENTRY);
        write_volatile(SF_CTRL_ID1_OFFSET as *mut u32, D0_FLASH_ADDR + 0x1000);
        // Write flags before releasing D0, then clean D-cache. C does
        // L1C_DCache_Clean_By_Addr; without it D0 can spin on stale zeros.
        write_volatile(IPC_SYNC_ADDR1 as *mut u32, IPC_SYNC_FLAG);
        write_volatile(IPC_SYNC_ADDR2 as *mut u32, IPC_SYNC_FLAG);
        dcache_clean(IPC_SYNC_ADDR1);

        write_volatile(
            MM_CLK_CTRL_CPU as *mut u32,
            read_volatile(MM_CLK_CTRL_CPU as *const u32) | MMCPU0_CLK_EN,
        );
        riscv::asm::delay(MCU_HZ / 1_000_000);
        write_volatile(
            MM_SW_SYS_RESET as *mut u32,
            read_volatile(MM_SW_SYS_RESET as *const u32) & !MMCPU0_RESET,
        );
    }
}

/// C `csi_icache_enable`: invalidate, then set MHCR.IE.
fn enable_icache() {
    unsafe {
        core::arch::asm!("fence", options(nostack));
        core::arch::asm!("fence.i", options(nostack));
        core::arch::asm!(".word 0x0100000b", options(nostack));
        let mut mhcr: usize;
        core::arch::asm!("csrr {mhcr}, 0x7C1", mhcr = out(reg) mhcr, options(nostack));
        mhcr |= 1;
        core::arch::asm!("csrw 0x7C1, {mhcr}", mhcr = in(reg) mhcr, options(nostack));
        core::arch::asm!("fence", options(nostack));
        core::arch::asm!("fence.i", options(nostack));
    }
}

/// T-Head `dcache.cpa` (same encoding as C `__CUSTOM_DCACHE_CPA`).
fn dcache_clean(addr: usize) {
    unsafe {
        core::arch::asm!(
            ".insn i 0x0b, 0x0, x0, {addr}, 0x29",
            addr = in(reg) addr,
            options(nostack)
        );
        core::arch::asm!("fence", options(nostack));
    }
}

fn rdcycle() -> u64 {
    let mut hi: u32;
    let mut lo: u32;
    let mut hi2: u32;
    unsafe {
        loop {
            core::arch::asm!(
                "rdcycleh {hi}",
                "rdcycle {lo}",
                "rdcycleh {hi2}",
                hi = out(reg) hi,
                lo = out(reg) lo,
                hi2 = out(reg) hi2,
                options(nomem, nostack)
            );
            if hi == hi2 {
                break;
            }
        }
    }
    ((hi as u64) << 32) | lo as u64
}

fn delay_ms(ms: u32) {
    let start = rdcycle();
    let need = MCU_HZ as u64 * ms as u64 / 1000;
    while rdcycle().wrapping_sub(start) < need {
        core::hint::spin_loop();
    }
}
