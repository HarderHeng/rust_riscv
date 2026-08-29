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
const UART3_BASE: usize = 0x3000_2000;
const UART_FIFO_CONFIG_1: usize = UART3_BASE + 0x84;
const UART_FIFO_WDATA: usize = UART3_BASE + 0x88;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_SHUTDOWN: usize = 8;
const CAUSE_ECALL_S: usize = 9;

struct Uart3Xclk;

impl bouffalo_hal::uart::Clock for Uart3Xclk {
    fn uart_clock<const I: usize>(self) -> Hertz {
        Hertz(XTAL_HZ)
    }
}

unsafe extern "C" {
    fn trap_m();
}

core::arch::global_asm!(
    ".pushsection .text.trap_m,\"ax\",@progbits",
    ".align 2",
    ".globl trap_m",
    ".type trap_m, @function",
    "trap_m:",
    "    addi sp, sp, -256",
    "    sd ra, 8(sp)",
    "    sd gp, 24(sp)",
    "    sd tp, 32(sp)",
    "    sd t0, 40(sp)",
    "    sd t1, 48(sp)",
    "    sd t2, 56(sp)",
    "    sd s0, 64(sp)",
    "    sd s1, 72(sp)",
    "    sd a0, 80(sp)",
    "    sd a1, 88(sp)",
    "    sd a2, 96(sp)",
    "    sd a3, 104(sp)",
    "    sd a4, 112(sp)",
    "    sd a5, 120(sp)",
    "    sd a6, 128(sp)",
    "    sd a7, 136(sp)",
    "    sd s2, 144(sp)",
    "    sd s3, 152(sp)",
    "    sd s4, 160(sp)",
    "    sd s5, 168(sp)",
    "    sd s6, 176(sp)",
    "    sd s7, 184(sp)",
    "    sd s8, 192(sp)",
    "    sd s9, 200(sp)",
    "    sd s10, 208(sp)",
    "    sd s11, 216(sp)",
    "    sd t3, 224(sp)",
    "    sd t4, 232(sp)",
    "    sd t5, 240(sp)",
    "    sd t6, 248(sp)",
    "    addi t0, sp, 256",
    "    sd t0, 16(sp)",
    "    ld a0, 80(sp)",
    "    ld a1, 136(sp)",
    "    csrr a2, mcause",
    "    call trap_handle",
    "    sd a0, 80(sp)",
    "    csrr t0, mepc",
    "    addi t0, t0, 4",
    "    csrw mepc, t0",
    "    ld ra, 8(sp)",
    "    ld gp, 24(sp)",
    "    ld tp, 32(sp)",
    "    ld t0, 40(sp)",
    "    ld t1, 48(sp)",
    "    ld t2, 56(sp)",
    "    ld s0, 64(sp)",
    "    ld s1, 72(sp)",
    "    ld a0, 80(sp)",
    "    ld a1, 88(sp)",
    "    ld a2, 96(sp)",
    "    ld a3, 104(sp)",
    "    ld a4, 112(sp)",
    "    ld a5, 120(sp)",
    "    ld a6, 128(sp)",
    "    ld a7, 136(sp)",
    "    ld s2, 144(sp)",
    "    ld s3, 152(sp)",
    "    ld s4, 160(sp)",
    "    ld s5, 168(sp)",
    "    ld s6, 176(sp)",
    "    ld s7, 184(sp)",
    "    ld s8, 192(sp)",
    "    ld s9, 200(sp)",
    "    ld s10, 208(sp)",
    "    ld s11, 216(sp)",
    "    ld t3, 224(sp)",
    "    ld t4, 232(sp)",
    "    ld t5, 240(sp)",
    "    ld t6, 248(sp)",
    "    ld sp, 16(sp)",
    "    mret",
    ".popsection",
);

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

    enable_icache();
    write!(serial, "rust helloworld from D0/C906, icache on\r\n").ok();
    serial.flush().ok();

    pmp_allow_all();
    install_trap();
    write!(serial, "[M] sbi0 entering S-mode\r\n").ok();
    serial.flush().ok();
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

fn install_trap() {
    let addr = trap_m as usize;
    unsafe {
        core::arch::asm!(
            "csrw mtvec, {addr}",
            "csrw medeleg, zero",
            "csrw mideleg, zero",
            "csrw mie, zero",
            "csrw sie, zero",
            "csrr {m}, mstatus",
            "andi {m}, {m}, {mie_off}",
            "csrw mstatus, {m}",
            addr = in(reg) addr,
            m = out(reg) _,
            mie_off = const !(1usize << 3),
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

fn hang() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
extern "C" fn trap_handle(a0: usize, a7: usize, mcause: usize) -> usize {
    if mcause != CAUSE_ECALL_S {
        uart3_puts("[M] trap\r\n");
        hang();
    }
    match a7 {
        SBI_CONSOLE_PUTCHAR => {
            uart3_putb(a0 as u8);
            0
        }
        SBI_SHUTDOWN => {
            uart3_puts("[M] shutdown\r\n");
            hang();
        }
        _ => {
            uart3_puts("[M] bad sbi\r\n");
            hang();
        }
    }
}

fn sbi_putchar(c: u8) {
    let _ret: usize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") c as usize => _ret,
            in("a1") 0usize,
            in("a2") 0usize,
            in("a7") SBI_CONSOLE_PUTCHAR,
            options(nostack)
        );
    }
}

fn sbi_shutdown() -> ! {
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") 0usize,
            in("a1") 0usize,
            in("a2") 0usize,
            in("a7") SBI_SHUTDOWN,
            options(nostack)
        );
        core::hint::unreachable_unchecked();
    }
}

#[unsafe(no_mangle)]
extern "C" fn s_main() -> ! {
    for b in b"[S] hello via sbi\r\n" {
        sbi_putchar(*b);
    }
    sbi_shutdown();
}
