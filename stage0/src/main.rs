#![no_std]
#![no_main]

use bouffalo_hal::{prelude::*, uart::Config};
use bouffalo_rt::{Clocks, Peripherals, entry};
use core::ptr::{read_volatile, write_volatile};
use embedded_time::rate::*;
use panic_halt as _;

const XTAL_HZ: u32 = 40_000_000;
const MM_GLB_BASE: usize = 0x3000_7000;
const MM_CLK_CTRL_CPU: usize = MM_GLB_BASE;
const MM_CLK_CTRL_PERI: usize = MM_GLB_BASE + 0x10;
const DSP_UART_CLK_XCLK: u32 = 2;
const IPC_SYNC_ADDR1: usize = 0x4000_0000;
const IPC_SYNC_ADDR2: usize = 0x4000_0004;
const IPC_SYNC_FLAG: u32 = 0x1234_5678;

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

    writeln!(serial, "rust helloworld from D0/C906, uart up").ok();
    serial.flush().ok();

    wait_for_m0();
    writeln!(serial, "rust helloworld from D0/C906, ipc synced").ok();
    serial.flush().ok();

    enable_icache();
    writeln!(serial, "rust helloworld from D0/C906, icache on").ok();
    serial.flush().ok();

    writeln!(serial, "[M] stage0 entering S-mode").ok();
    serial.flush().ok();
    pmp_allow_all();
    enter_s_mode(s_main);
}

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

fn wait_for_m0() {
    unsafe {
        while read_volatile(IPC_SYNC_ADDR1 as *const u32) != IPC_SYNC_FLAG
            || read_volatile(IPC_SYNC_ADDR2 as *const u32) != IPC_SYNC_FLAG
        {
            dcache_invalidate(IPC_SYNC_ADDR1);
            core::hint::spin_loop();
        }
        write_volatile(IPC_SYNC_ADDR1 as *mut u32, 0);
        write_volatile(IPC_SYNC_ADDR2 as *mut u32, 0);
    }
}

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

const UART3_BASE: usize = 0x3000_2000;
const UART_FIFO_CONFIG_1: usize = UART3_BASE + 0x84;
const UART_FIFO_WDATA: usize = UART3_BASE + 0x88;

fn pmp_allow_all() {
    unsafe {
        let addr: usize = usize::MAX;
        let cfg: usize = 0x1F;
        core::arch::asm!(
            "csrw pmpaddr0, {addr}",
            "csrw pmpcfg0, {cfg}",
            addr = in(reg) addr,
            cfg = in(reg) cfg,
            options(nostack)
        );
    }
}

fn enter_s_mode(dest: extern "C" fn() -> !) -> ! {
    let dest = dest as usize;
    unsafe {
        core::arch::asm!(
            "csrw satp, zero",
            "csrr {mstatus}, mstatus",
            "li {tmp}, {mpp_mask}",
            "and {mstatus}, {mstatus}, {tmp}",
            "li {tmp}, {mpp_s}",
            "or {mstatus}, {mstatus}, {tmp}",
            "csrw mstatus, {mstatus}",
            "csrw mepc, {dest}",
            "mret",
            dest = in(reg) dest,
            mstatus = out(reg) _,
            tmp = out(reg) _,
            mpp_mask = const !(0b11 << 11),
            mpp_s = const 0b01 << 11,
            options(nostack)
        );
        core::hint::unreachable_unchecked();
    }
}

fn uart3_putb(b: u8) {
    unsafe {
        // UART_FIFO_CONFIG_1[5:0] is TX free space (empty == 32). Wait while full.
        while (read_volatile(UART_FIFO_CONFIG_1 as *const u32) & 0x3F) == 0 {
            core::hint::spin_loop();
        }
        write_volatile(UART_FIFO_WDATA as *mut u32, b as u32);
    }
}

fn uart3_puts(s: &str) {
    for b in s.as_bytes() {
        uart3_putb(*b);
    }
}

#[unsafe(no_mangle)]
extern "C" fn s_main() -> ! {
    uart3_puts("[S] hello from supervisor on C906\r\n");
    loop {
        core::hint::spin_loop();
    }
}
