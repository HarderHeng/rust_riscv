#![no_std]
#![no_main]

use bouffalo_hal::{prelude::*, uart::Config};
use bouffalo_rt::{Clocks, Peripherals, entry};
use core::ptr::{read_volatile, write_volatile};
use embedded_time::rate::*;
use panic_halt as _;

const XTAL_HZ: u32 = 40_000_000;
const DSP_HZ: u32 = 320_000_000;
const MTIMER_HZ: u32 = 1_000_000;
const MTIMER_DIV: u32 = DSP_HZ / MTIMER_HZ - 1;
const TICK_INTERVAL: u64 = 200_000;
const TICK_COUNT: u8 = 5;

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

const MM_MISC_BASE: usize = 0x3000_0000;
const MM_MISC_CPU_RTC: usize = MM_MISC_BASE + 0x18;
const C906_RTC_DIV_MASK: u32 = 0x3FF;
const C906_RTC_EN: u32 = 1 << 31;

const MTIMECMPL: usize = 0xE400_4000;
const MTIMECMPH: usize = 0xE400_4004;

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_SHUTDOWN: usize = 8;
const CAUSE_ECALL_S: usize = 9;
const CAUSE_IRQ: usize = 1 << (usize::BITS - 1);
const CAUSE_M_TIMER: usize = CAUSE_IRQ | 7;
const CAUSE_S_TIMER: usize = CAUSE_IRQ | 5;
const MIP_STIP: usize = 1 << 5;
const MIE_MTIE: usize = 1 << 7;
const MIDELEG_STI: usize = 1 << 5;
const MSTATUS_MPIE: usize = 1 << 7;
const SIE_STIE: usize = 1 << 5;
const SSTATUS_SIE: usize = 1 << 1;

struct Uart3Xclk;

impl bouffalo_hal::uart::Clock for Uart3Xclk {
    fn uart_clock<const I: usize>(self) -> Hertz {
        Hertz(XTAL_HZ)
    }
}

unsafe extern "C" {
    fn trap_m();
    fn trap_s();
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
    ".pushsection .text.trap_s,\"ax\",@progbits",
    ".align 2",
    ".globl trap_s",
    ".type trap_s, @function",
    "trap_s:",
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
    "    csrr a0, scause",
    "    call trap_s_handle",
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
    "    sret",
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

    enable_mtimer_clock();
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

/// C `CPU_Set_MTimer_CLK(ENABLE, DSP_HZ/1e6 - 1)` on D0: `MM_MISC_CPU_RTC`.
fn enable_mtimer_clock() {
    unsafe {
        let mut v = read_volatile(MM_MISC_CPU_RTC as *const u32);
        v &= !C906_RTC_EN;
        write_volatile(MM_MISC_CPU_RTC as *mut u32, v);
        v = read_volatile(MM_MISC_CPU_RTC as *const u32);
        v = (v & !C906_RTC_DIV_MASK) | MTIMER_DIV;
        write_volatile(MM_MISC_CPU_RTC as *mut u32, v);
        v = read_volatile(MM_MISC_CPU_RTC as *const u32);
        v |= C906_RTC_EN;
        write_volatile(MM_MISC_CPU_RTC as *mut u32, v);
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
    let addr = trap_m as *const () as usize;
    unsafe {
        core::arch::asm!(
            "csrw mtvec, {addr}",
            "csrw medeleg, zero",
            "csrw mideleg, {sti}",
            "csrw mie, zero",
            "csrw sie, zero",
            addr = in(reg) addr,
            sti = in(reg) MIDELEG_STI,
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
            "li {tmp}, {mpp_s_mpie}",
            "or {mstatus}, {mstatus}, {tmp}",
            "csrw mstatus, {mstatus}",
            "csrw mepc, {dest}",
            "mret",
            dest = in(reg) dest,
            mstatus = out(reg) _,
            tmp = out(reg) _,
            mpp_mask = const !(0b11 << 11),
            mpp_s_mpie = const (0b01 << 11) | MSTATUS_MPIE,
            options(nostack)
        );
        core::hint::unreachable_unchecked();
    }
}

fn uart3_putb(b: u8) {
    unsafe {
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

fn bump_mepc() {
    unsafe {
        core::arch::asm!(
            "csrr {p}, mepc",
            "addi {p}, {p}, 4",
            "csrw mepc, {p}",
            p = out(reg) _,
            options(nostack)
        );
    }
}

fn csr_set(csr: u32, bits: usize) {
    unsafe {
        match csr {
            0x304 => core::arch::asm!("csrs mie, {b}", b = in(reg) bits, options(nostack)),
            0x344 => core::arch::asm!("csrs mip, {b}", b = in(reg) bits, options(nostack)),
            0x104 => core::arch::asm!("csrs sie, {b}", b = in(reg) bits, options(nostack)),
            0x100 => core::arch::asm!("csrs sstatus, {b}", b = in(reg) bits, options(nostack)),
            _ => {}
        }
    }
}

fn csr_clear(csr: u32, bits: usize) {
    unsafe {
        match csr {
            0x304 => core::arch::asm!("csrc mie, {b}", b = in(reg) bits, options(nostack)),
            0x344 => core::arch::asm!("csrc mip, {b}", b = in(reg) bits, options(nostack)),
            _ => {}
        }
    }
}

fn write_mtimecmp(next: u64) {
    unsafe {
        write_volatile(MTIMECMPH as *mut u32, u32::MAX);
        write_volatile(MTIMECMPL as *mut u32, next as u32);
        write_volatile(MTIMECMPH as *mut u32, (next >> 32) as u32);
    }
}

fn sbi_set_timer_m(next: u64) {
    write_mtimecmp(next);
    csr_clear(0x344, MIP_STIP);
    csr_set(0x304, MIE_MTIE);
}

fn handle_m_timer() {
    csr_set(0x344, MIP_STIP);
    csr_clear(0x304, MIE_MTIE);
}

#[unsafe(no_mangle)]
extern "C" fn trap_handle(a0: usize, a7: usize, mcause: usize) -> usize {
    if mcause == CAUSE_M_TIMER {
        handle_m_timer();
        return a0;
    }
    if mcause != CAUSE_ECALL_S {
        uart3_puts("[M] trap\r\n");
        hang();
    }
    bump_mepc();
    match a7 {
        SBI_SET_TIMER => {
            sbi_set_timer_m(a0 as u64);
            0
        }
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

fn rdtime() -> u64 {
    let t: u64;
    unsafe {
        core::arch::asm!("rdtime {t}", t = out(reg) t, options(nomem, nostack));
    }
    t
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

fn sbi_set_timer(next: u64) {
    let _ret: usize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") next as usize => _ret,
            in("a1") 0usize,
            in("a2") 0usize,
            in("a7") SBI_SET_TIMER,
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

fn s_puts(s: &str) {
    for b in s.as_bytes() {
        sbi_putchar(*b);
    }
}

fn s_print_tick(n: u8) {
    s_puts("[S] tick ");
    sbi_putchar(b'0' + n);
    sbi_putchar(b'\r');
    sbi_putchar(b'\n');
}

static mut TICKS: u8 = 0;

#[unsafe(no_mangle)]
extern "C" fn trap_s_handle(scause: usize) {
    if scause != CAUSE_S_TIMER {
        s_puts("[S] trap\r\n");
        hang();
    }
    let n = unsafe {
        TICKS = TICKS.saturating_add(1);
        TICKS
    };
    s_print_tick(n);
    if n >= TICK_COUNT {
        sbi_shutdown();
    }
    sbi_set_timer(rdtime() + TICK_INTERVAL);
}

#[unsafe(no_mangle)]
extern "C" fn s_main() -> ! {
    s_puts("[S] hello via sbi\r\n");
    let stvec = trap_s as *const () as usize;
    unsafe {
        core::arch::asm!("csrw stvec, {v}", v = in(reg) stvec, options(nostack));
    }
    csr_set(0x104, SIE_STIE);
    sbi_set_timer(rdtime() + TICK_INTERVAL);
    csr_set(0x100, SSTATUS_SIE);
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nostack));
        }
    }
}
