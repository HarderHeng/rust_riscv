#![no_std]
#![no_main]

use bouffalo_hal::{
    prelude::*,
    psram::init_psram,
    uart::{Config, RegisterBlock as UartRegs},
};
use bouffalo_rt::{Clocks, Peripherals, entry};
use core::ptr::{read_volatile, write_volatile};
use embedded_time::rate::*;
use panic_halt as _;
use riscv::{
    asm,
    register::{
        mcounteren, medeleg, mepc, mideleg, mie, mip, mstatus, mtvec, pmpaddr0, pmpcfg0, satp,
        sie, stvec, time,
    },
};
use xuantie_riscv::{asm as xt_asm, register::mhcr};

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

const MM_MISC_BASE: usize = 0x3000_0000;
const MM_MISC_CPU_RTC: usize = MM_MISC_BASE + 0x18;
const C906_RTC_DIV_MASK: u32 = 0x3FF;
const C906_RTC_EN: u32 = 1 << 31;

const MTIMECMPL: usize = 0xE400_4000;
const MTIMECMPH: usize = 0xE400_4004;
const PSRAM_BASE: usize = 0x5000_0000;
const PSRAM_PROBE: usize = PSRAM_BASE + 0x1000;
const PAGE_TABLE: usize = PSRAM_BASE;
const M_PSRAM_MAGIC: u64 = 0x5A5A_5A5A_5A5A_5A5A;
const S_PSRAM_MAGIC: u64 = 0xA5A5_A5A5_A5A5_A5A5;
const TZC_SEC_BASE: usize = 0x2000_5000;
const TZC_PSRAMA_TZSRG_CTRL: usize = TZC_SEC_BASE + 0x380;
const TZC_PSRAMA_R0_EN: u32 = 1 << 16;
const PTE_LEAF: u64 = 0xC7;
const CACHE_LINE: usize = 64;

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_SHUTDOWN: usize = 8;
const CAUSE_IRQ: usize = 1 << (usize::BITS - 1);
const CAUSE_ECALL_S: usize = 9;
const CAUSE_M_TIMER: usize = CAUSE_IRQ | 7;
const CAUSE_S_TIMER: usize = CAUSE_IRQ | 5;

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

    init_psram(&p.psram, &p.glb);
    tzc_release_psrama();
    if !psram_probe(M_PSRAM_MAGIC) {
        write!(serial, "[M] psram fail\r\n").ok();
        serial.flush().ok();
        hang();
    }
    write!(serial, "[M] psram ok\r\n").ok();
    serial.flush().ok();

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
        asm::fence();
        asm::fence_i();
        xt_asm::icache_iall();
        mhcr::set_ie();
        asm::fence();
        asm::fence_i();
    }
}

fn dcache_invalidate(addr: usize) {
    unsafe {
        xt_asm::dcache_ipa(addr);
        asm::fence();
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

/// C `Tzc_Sec_PSRAMA_Access_Release`: clear region0 EN (bit 16).
fn tzc_release_psrama() {
    unsafe {
        let v = read_volatile(TZC_PSRAMA_TZSRG_CTRL as *const u32);
        write_volatile(TZC_PSRAMA_TZSRG_CTRL as *mut u32, v & !TZC_PSRAMA_R0_EN);
    }
}

fn psram_probe(magic: u64) -> bool {
    unsafe {
        write_volatile(PSRAM_PROBE as *mut u64, magic);
        read_volatile(PSRAM_PROBE as *const u64) == magic
    }
}

fn dcache_clean_range(addr: usize, len: usize) {
    let mut a = addr & !(CACHE_LINE - 1);
    let end = addr + len;
    unsafe {
        while a < end {
            xt_asm::dcache_cpal1(a);
            a += CACHE_LINE;
        }
        asm::fence();
    }
}

fn pte_1g(pa: u64) -> u64 {
    ((pa >> 12) << 10) | PTE_LEAF
}

fn build_identity_root() {
    let table = PAGE_TABLE as *mut u64;
    unsafe {
        for i in 0..512 {
            write_volatile(table.add(i), 0);
        }
        write_volatile(table.add(0), pte_1g(0x0000_0000));
        write_volatile(table.add(1), pte_1g(0x4000_0000));
    }
    dcache_clean_range(PAGE_TABLE, 4096);
}

fn enable_sv39() {
    asm::sfence_vma_all();
    unsafe {
        satp::set(satp::Mode::Sv39, 0, PAGE_TABLE >> 12);
    }
    asm::sfence_vma_all();
}

fn pmp_allow_all() {
    unsafe {
        pmpaddr0::write(usize::MAX);
        pmpcfg0::write(0x1F);
    }
}

fn install_trap() {
    let mut vec = mtvec::Mtvec::from_bits(0);
    vec.set_address(trap_m as *const () as usize);
    vec.set_trap_mode(mtvec::TrapMode::Direct);
    unsafe {
        mtvec::write(vec);
        medeleg::write(medeleg::Medeleg::from_bits(0));
        mideleg::write(mideleg::Mideleg::from_bits(1 << 5));
        mie::write(mie::Mie::from_bits(0));
        sie::write(sie::Sie::from_bits(0));
        mcounteren::set_tm();
    }
}

fn enter_s_mode(dest: extern "C" fn() -> !) -> ! {
    unsafe {
        satp::write(satp::Satp::from_bits(0));
        mstatus::set_mpp(mstatus::MPP::Supervisor);
        mstatus::set_mpie();
        mepc::write(dest as usize);
        core::arch::asm!("mret", options(nostack));
        core::hint::unreachable_unchecked();
    }
}

fn uart3_regs() -> &'static UartRegs {
    unsafe { &*(UART3_BASE as *const UartRegs) }
}

fn uart3_putb(b: u8) {
    let uart = uart3_regs();
    while uart.fifo_config_1.read().transmit_available_bytes() == 0 {
        core::hint::spin_loop();
    }
    unsafe {
        uart.fifo_write.write(b);
    }
}

fn uart3_puts(s: &str) {
    for b in s.as_bytes() {
        uart3_putb(*b);
    }
}

fn uart3_put_hex(v: usize) {
    for i in (0..16).rev() {
        let n = (v >> (i * 4)) & 0xf;
        uart3_putb(if n < 10 {
            b'0' + n as u8
        } else {
            b'a' + (n as u8 - 10)
        });
    }
}

fn hang() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

fn bump_mepc() {
    unsafe {
        mepc::write(mepc::read().wrapping_add(4));
    }
}

/// C906 compares `mtime` to the live 64-bit `mtimecmp`. `THeadClint::write_mtimecmp`
/// writes lo then hi, which can match too early. Keep the three 32-bit stores.
fn write_mtimecmp(next: u64) {
    unsafe {
        write_volatile(MTIMECMPH as *mut u32, u32::MAX);
        write_volatile(MTIMECMPL as *mut u32, next as u32);
        write_volatile(MTIMECMPH as *mut u32, (next >> 32) as u32);
    }
}

fn sbi_set_timer_m(next: u64) {
    write_mtimecmp(next);
    unsafe {
        mip::clear_stimer();
        mie::set_mtimer();
    }
}

fn handle_m_timer() {
    unsafe {
        mip::set_stimer();
        mie::clear_mtimer();
    }
}

#[unsafe(no_mangle)]
extern "C" fn trap_handle(a0: usize, a7: usize, cause: usize) -> usize {
    if cause == CAUSE_M_TIMER {
        handle_m_timer();
        return a0;
    }
    if cause != CAUSE_ECALL_S {
        uart3_puts("[M] trap ");
        uart3_put_hex(cause);
        uart3_puts("\r\n");
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
    time::read64()
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
extern "C" fn trap_s_handle(cause: usize) {
    if cause != CAUSE_S_TIMER {
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
    let mut vec = stvec::Stvec::from_bits(0);
    vec.set_address(trap_s as *const () as usize);
    vec.set_trap_mode(stvec::TrapMode::Direct);
    unsafe {
        stvec::write(vec);
    }
    build_identity_root();
    enable_sv39();
    s_puts("[S] satp on\r\n");
    if psram_probe(S_PSRAM_MAGIC) {
        s_puts("[S] psram ok\r\n");
    } else {
        s_puts("[S] psram fail\r\n");
    }
    sbi_shutdown();
}
