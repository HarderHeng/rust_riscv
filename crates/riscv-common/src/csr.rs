//! RISC-V Control and Status Register (CSR) utilities.
//!
//! Provides safe wrappers and macros for accessing RISC-V CSRs.
//! All CSR access is done via inline assembly to prevent compiler reordering.

/// Read a CSR value.
///
/// # Safety
///
/// Reading some CSRs may have side effects or require specific privilege levels.
/// Caller must ensure the CSR access is valid in the current context.
///
/// # Example
///
/// ```no_run
/// let mstatus = unsafe { csr_read!(mstatus) };
/// ```
#[macro_export]
macro_rules! csr_read {
    ($csr:ident) => {{
        let value: usize;
        core::arch::asm!(
            concat!("csrr {}, ", stringify!($csr)),
            out(reg) value,
            options(nomem, nostack)
        );
        value
    }};
}

/// Write a value to a CSR.
///
/// # Safety
///
/// Writing to CSRs can affect CPU behavior and requires appropriate privilege level.
/// Caller must ensure the write is safe and valid.
///
/// # Example
///
/// ```no_run
/// unsafe { csr_write!(mstatus, 0x1800) };
/// ```
#[macro_export]
macro_rules! csr_write {
    ($csr:ident, $value:expr) => {{
        core::arch::asm!(
            concat!("csrw ", stringify!($csr), ", {}"),
            in(reg) $value,
            options(nomem, nostack)
        );
    }};
}

/// Set specific bits in a CSR (read-modify-write with OR).
///
/// # Safety
///
/// Modifying CSRs can affect CPU behavior. Caller must ensure the modification is safe.
///
/// # Example
///
/// ```no_run
/// // Enable machine interrupts
/// unsafe { csr_set!(mstatus, 0x8) };
/// ```
#[macro_export]
macro_rules! csr_set {
    ($csr:ident, $bits:expr) => {{
        core::arch::asm!(
            concat!("csrs ", stringify!($csr), ", {}"),
            in(reg) $bits,
            options(nomem, nostack)
        );
    }};
}

/// Clear specific bits in a CSR (read-modify-write with AND-NOT).
///
/// # Safety
///
/// Modifying CSRs can affect CPU behavior. Caller must ensure the modification is safe.
///
/// # Example
///
/// ```no_run
/// // Disable machine interrupts
/// unsafe { csr_clear!(mstatus, 0x8) };
/// ```
#[macro_export]
macro_rules! csr_clear {
    ($csr:ident, $bits:expr) => {{
        core::arch::asm!(
            concat!("csrc ", stringify!($csr), ", {}"),
            in(reg) $bits,
            options(nomem, nostack)
        );
    }};
}

/// Read and then write a CSR, returning the old value.
///
/// # Safety
///
/// Writing to CSRs can affect CPU behavior. Caller must ensure the operation is safe.
#[macro_export]
macro_rules! csr_swap {
    ($csr:ident, $value:expr) => {{
        let old_value: usize;
        core::arch::asm!(
            concat!("csrrw {}, ", stringify!($csr), ", {}"),
            out(reg) old_value,
            in(reg) $value,
            options(nomem, nostack)
        );
        old_value
    }};
}

// ---------------------------------------------------------------------------
// Common CSR bit field definitions
// ---------------------------------------------------------------------------

/// Machine Status Register (mstatus) bit fields
pub mod mstatus {
    /// Machine Interrupt Enable (MIE) bit position
    pub const MIE: usize = 1 << 3;

    /// Machine Previous Interrupt Enable (MPIE) bit position
    pub const MPIE: usize = 1 << 7;

    /// Machine Previous Privilege mode (MPP) field mask
    pub const MPP_MASK: usize = 0b11 << 11;

    /// Machine Previous Privilege: User mode
    pub const MPP_USER: usize = 0b00 << 11;

    /// Machine Previous Privilege: Supervisor mode
    pub const MPP_SUPERVISOR: usize = 0b01 << 11;

    /// Machine Previous Privilege: Machine mode
    pub const MPP_MACHINE: usize = 0b11 << 11;
}

/// Machine Interrupt Enable (mie) bit fields
pub mod mie {
    /// Machine Software Interrupt Enable
    pub const MSIE: usize = 1 << 3;

    /// Machine Timer Interrupt Enable
    pub const MTIE: usize = 1 << 7;

    /// Machine External Interrupt Enable
    pub const MEIE: usize = 1 << 11;
}

/// Machine Interrupt Pending (mip) bit fields
pub mod mip {
    /// Machine Software Interrupt Pending
    pub const MSIP: usize = 1 << 3;

    /// Machine Timer Interrupt Pending
    pub const MTIP: usize = 1 << 7;

    /// Machine External Interrupt Pending
    pub const MEIP: usize = 1 << 11;
}

/// Machine Cause Register (mcause) values
pub mod mcause {
    /// Instruction address misaligned
    pub const INSTRUCTION_MISALIGNED: usize = 0;

    /// Instruction access fault
    pub const INSTRUCTION_ACCESS_FAULT: usize = 1;

    /// Illegal instruction
    pub const ILLEGAL_INSTRUCTION: usize = 2;

    /// Breakpoint
    pub const BREAKPOINT: usize = 3;

    /// Load address misaligned
    pub const LOAD_MISALIGNED: usize = 4;

    /// Load access fault
    pub const LOAD_ACCESS_FAULT: usize = 5;

    /// Store/AMO address misaligned
    pub const STORE_MISALIGNED: usize = 6;

    /// Store/AMO access fault
    pub const STORE_ACCESS_FAULT: usize = 7;

    /// Environment call from U-mode
    pub const ECALL_USER: usize = 8;

    /// Environment call from S-mode
    pub const ECALL_SUPERVISOR: usize = 9;

    /// Environment call from M-mode
    pub const ECALL_MACHINE: usize = 11;

    /// Instruction page fault
    pub const INSTRUCTION_PAGE_FAULT: usize = 12;

    /// Load page fault
    pub const LOAD_PAGE_FAULT: usize = 13;

    /// Store/AMO page fault
    pub const STORE_PAGE_FAULT: usize = 15;

    /// Interrupt bit (high bit set indicates interrupt, not exception)
    #[cfg(target_arch = "riscv32")]
    pub const INTERRUPT_BIT: usize = 1 << 31;

    /// Interrupt bit for RV64 (high bit set indicates an interrupt).
    #[cfg(target_arch = "riscv64")]
    pub const INTERRUPT_BIT: usize = 1 << 63;

    /// Machine software interrupt
    pub const MACHINE_SOFTWARE_INTERRUPT: usize = INTERRUPT_BIT | 3;

    /// Machine timer interrupt
    pub const MACHINE_TIMER_INTERRUPT: usize = INTERRUPT_BIT | 7;

    /// Machine external interrupt
    pub const MACHINE_EXTERNAL_INTERRUPT: usize = INTERRUPT_BIT | 11;
}

// ---------------------------------------------------------------------------
// Helper functions for common CSR operations
// ---------------------------------------------------------------------------

/// Enable machine interrupts globally.
///
/// # Safety
///
/// Enabling interrupts can cause interrupt handlers to run. Caller must ensure
/// interrupt handlers are properly configured before calling this.
#[inline]
pub unsafe fn enable_interrupts() {
    csr_set!(mstatus, mstatus::MIE);
}

/// Disable machine interrupts globally.
///
/// # Safety
///
/// Caller must ensure it is safe to mask interrupts at this point (for example,
/// not holding a lock that an interrupt handler needs to make progress).
#[inline]
pub unsafe fn disable_interrupts() {
    csr_clear!(mstatus, mstatus::MIE);
}

/// Check if machine interrupts are enabled.
#[inline]
pub fn interrupts_enabled() -> bool {
    unsafe { csr_read!(mstatus) & mstatus::MIE != 0 }
}

/// Set the machine trap vector address.
///
/// # Safety
///
/// The trap vector must point to a valid trap handler function.
/// The mode bits (lowest 2 bits) determine vectoring:
/// - 0b00: Direct mode (all traps jump to base address)
/// - 0b01: Vectored mode (interrupts jump to base + 4*cause)
#[inline]
pub unsafe fn set_trap_vector(addr: usize, vectored: bool) {
    let mtvec = if vectored {
        (addr & !0b11) | 0b01
    } else {
        addr & !0b11
    };
    csr_write!(mtvec, mtvec);
}

/// Read the current machine trap vector.
#[inline]
pub fn get_trap_vector() -> (usize, bool) {
    let mtvec = unsafe { csr_read!(mtvec) };
    let addr = mtvec & !0b11;
    let vectored = (mtvec & 0b11) == 0b01;
    (addr, vectored)
}
