//! Trap (interrupt and exception) handler for RISC-V M-mode.
//!
//! This module provides the low-level trap entry point (`trap_handler`) in
//! assembly, which saves all general-purpose registers to the stack, calls
//! the Rust dispatcher (`trap_handler_rust`), and restores registers before
//! returning via `mret`.
//!
//! # Callback Registration System
//!
//! This module provides a flexible callback registration system for handling
//! interrupts. Instead of hardcoding interrupt handlers, users can register
//! custom callback functions at runtime.
//!
//! ## Example
//!
//! ```rust
//! use kernel::trap::{register_irq_handler, register_timer_handler};
//!
//! // Register a custom UART interrupt handler
//! fn my_uart_handler(irq: u32) {
//!     // Handle UART interrupt
//! }
//! register_irq_handler(10, my_uart_handler).unwrap();
//!
//! // Register a timer interrupt handler
//! fn my_timer_handler() {
//!     // Handle timer tick
//! }
//! register_timer_handler(my_timer_handler);
//! ```
//!
//! # Platform Integration
//!
//! This module is platform-agnostic. The actual interrupt controller
//! interaction (claim/complete for external interrupts) must be handled
//! by the board-specific code that registers the IRQ handlers.
//!
//! # CSR Registers
//!
//! - `mtvec`: Trap vector base address, points to `trap_handler`.
//! - `mcause`: Trap cause (interrupt bit + exception code).
//! - `mepc`: Machine exception program counter (return address).
//! - `mstatus`: Machine status (MIE = global interrupt enable).
//! - `mie`: Machine interrupt enable (MTIE, MSIE, MEIE).

use spin::Mutex;

// ---------------------------------------------------------------------------
// Callback types and storage
// ---------------------------------------------------------------------------

/// Handler function type for core interrupts (software, timer).
///
/// These handlers receive no parameters and are called directly from the
/// trap dispatcher when the corresponding interrupt occurs.
pub type CoreHandler = fn();

/// Handler function type for external interrupts.
///
/// The handler receives the IRQ number that triggered the interrupt.
/// This allows a single handler to potentially serve multiple IRQ sources.
pub type IrqHandler = fn(irq: u32);

/// Handler function type for interrupt claim operation.
///
/// This function is called by the trap handler to claim the pending
/// interrupt from the platform's interrupt controller. It should return
/// the IRQ number (or 0 if no interrupt is pending).
///
/// To work with the static trap handler, this must be a function pointer
/// that doesn't capture any variables. Board code should use a static
/// interrupt controller reference.
pub type ClaimHandler = fn() -> u32;

/// Handler function type for interrupt completion operation.
///
/// This function is called by the trap handler after the IRQ handler
/// completes to signal to the interrupt controller that handling is done.
///
/// To work with the static trap handler, this must be a function pointer
/// that doesn't capture any variables. Board code should use a static
/// interrupt controller reference.
pub type CompleteHandler = fn(irq: u32);

/// Maximum number of registered external IRQ handlers.
///
/// Sparse storage: boards typically register only a handful of IRQs
/// (console, virtio, …), so a small linear table beats a 128-slot dense array.
const MAX_IRQ_ENTRIES: usize = 32;

/// Storage for the software interrupt handler.
static SOFTWARE_HANDLER: Mutex<Option<CoreHandler>> = Mutex::new(None);

/// Storage for the timer interrupt handler.
static TIMER_HANDLER: Mutex<Option<CoreHandler>> = Mutex::new(None);

/// Sparse table of external interrupt handlers: `(irq, handler)` pairs.
/// IRQ 0 is reserved and should not be used.
static IRQ_HANDLERS: Mutex<heapless::Vec<(u32, IrqHandler), MAX_IRQ_ENTRIES>> =
    Mutex::new(heapless::Vec::new());

/// Storage for the interrupt claim handler.
/// This must be set by the platform before enabling external interrupts.
static CLAIM_HANDLER: Mutex<Option<ClaimHandler>> = Mutex::new(None);

/// Storage for the interrupt complete handler.
/// This must be set by the platform before enabling external interrupts.
static COMPLETE_HANDLER: Mutex<Option<CompleteHandler>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Callback registration API
// ---------------------------------------------------------------------------

/// Registers a handler for machine software interrupts (MSI, code 3).
///
/// # Example
/// ```rust
/// fn my_software_handler() {
///     // Handle software interrupt
/// }
/// kernel::trap::register_software_handler(my_software_handler);
/// ```
#[allow(dead_code)]
pub fn register_software_handler(handler: CoreHandler) {
    *SOFTWARE_HANDLER.lock() = Some(handler);
}

/// Unregisters the software interrupt handler.
#[allow(dead_code)]
pub fn unregister_software_handler() {
    *SOFTWARE_HANDLER.lock() = None;
}

/// Registers a handler for machine timer interrupts (MTI, code 7).
///
/// # Example
/// ```rust
/// fn my_timer_handler() {
///     // Handle timer interrupt
/// }
/// kernel::trap::register_timer_handler(my_timer_handler);
/// ```
#[allow(dead_code)]
pub fn register_timer_handler(handler: CoreHandler) {
    *TIMER_HANDLER.lock() = Some(handler);
}

/// Unregisters the timer interrupt handler.
#[allow(dead_code)]
pub fn unregister_timer_handler() {
    *TIMER_HANDLER.lock() = None;
}

/// Registers a handler for external interrupts (MEI, code 11).
///
/// # Arguments
/// * `irq` - The IRQ number (non-zero). IRQ 0 is reserved and will return an error.
///   Fails if the sparse handler table is full.
/// * `handler` - The callback function to invoke when this IRQ fires.
///
/// # Returns
/// * `Ok(())` if registration succeeded
/// * `Err(&str)` if the IRQ number is invalid
///
/// # Example
/// ```rust
/// fn uart_handler(irq: u32) {
///     // Handle UART interrupt
/// }
/// kernel::trap::register_irq_handler(10, uart_handler).unwrap();
/// ```
pub fn register_irq_handler(irq: u32, handler: IrqHandler) -> Result<(), &'static str> {
    if irq == 0 {
        return Err("IRQ 0 is reserved");
    }

    let mut handlers = IRQ_HANDLERS.lock();
    if let Some(entry) = handlers.iter_mut().find(|(n, _)| *n == irq) {
        entry.1 = handler;
        return Ok(());
    }
    handlers
        .push((irq, handler))
        .map_err(|_| "IRQ handler table full")
}

/// Unregisters an external interrupt handler.
///
/// # Arguments
/// * `irq` - The IRQ number to unregister
///
/// # Returns
/// * `Ok(())` if unregistration succeeded
/// * `Err(&str)` if the IRQ number is invalid
#[allow(dead_code)]
pub fn unregister_irq_handler(irq: u32) -> Result<(), &'static str> {
    if irq == 0 {
        return Err("IRQ 0 is reserved");
    }

    let mut handlers = IRQ_HANDLERS.lock();
    if let Some(idx) = handlers.iter().position(|(n, _)| *n == irq) {
        handlers.swap_remove(idx);
        Ok(())
    } else {
        Ok(())
    }
}

/// Registers the platform's interrupt claim handler.
///
/// This function must be called by the platform initialization code
/// before enabling external interrupts. The claim handler is responsible
/// for querying the platform's interrupt controller (e.g., PLIC) and
/// returning the pending IRQ number.
///
/// # Arguments
/// * `handler` - Function that claims interrupts from the platform
pub fn register_claim_handler(handler: ClaimHandler) {
    *CLAIM_HANDLER.lock() = Some(handler);
}

/// Registers the platform's interrupt completion handler.
///
/// This function must be called by the platform initialization code
/// before enabling external interrupts. The complete handler is responsible
/// for signaling to the platform's interrupt controller that interrupt
/// handling is finished.
///
/// # Arguments
/// * `handler` - Function that signals interrupt completion to the platform
pub fn register_complete_handler(handler: CompleteHandler) {
    *COMPLETE_HANDLER.lock() = Some(handler);
}

// ---------------------------------------------------------------------------
// TrapFrame structure
// ---------------------------------------------------------------------------

/// Saved register context for trap handling.
///
/// Layout matches the assembly code in `trap_handler`:
/// - regs[0..31] = x1-x31 (x0 is hardwired to 0, not saved)
/// - mepc = return address
#[repr(C)]
pub struct TrapFrame {
    /// General-purpose registers x1-x31
    pub regs: [usize; 31],
    /// Machine exception program counter
    pub mepc: usize,
}

// ---------------------------------------------------------------------------
// Assembly trap entry point
// ---------------------------------------------------------------------------

#[cfg(all(target_arch = "riscv32", not(target_feature = "e")))]
core::arch::global_asm!(
    ".align 4",
    ".global trap_handler",
    "trap_handler:",
    // Save all registers to stack (32 words = 128 bytes)
    "    addi sp, sp, -128",
    "    sw x1,   0(sp)",  // ra
    "    sw x2,   4(sp)",  // sp AFTER addi (frame pointer); not the pre-trap SP
    "    sw x3,   8(sp)",  // gp
    "    sw x4,  12(sp)",  // tp
    "    sw x5,  16(sp)",  // t0
    "    sw x6,  20(sp)",  // t1
    "    sw x7,  24(sp)",  // t2
    "    sw x8,  28(sp)",  // s0/fp
    "    sw x9,  32(sp)",  // s1
    "    sw x10, 36(sp)",  // a0
    "    sw x11, 40(sp)",  // a1
    "    sw x12, 44(sp)",  // a2
    "    sw x13, 48(sp)",  // a3
    "    sw x14, 52(sp)",  // a4
    "    sw x15, 56(sp)",  // a5
    "    sw x16, 60(sp)",  // a6
    "    sw x17, 64(sp)",  // a7
    "    sw x18, 68(sp)",  // s2
    "    sw x19, 72(sp)",  // s3
    "    sw x20, 76(sp)",  // s4
    "    sw x21, 80(sp)",  // s5
    "    sw x22, 84(sp)",  // s6
    "    sw x23, 88(sp)",  // s7
    "    sw x24, 92(sp)",  // s8
    "    sw x25, 96(sp)",  // s9
    "    sw x26, 100(sp)", // s10
    "    sw x27, 104(sp)", // s11
    "    sw x28, 108(sp)", // t3
    "    sw x29, 112(sp)", // t4
    "    sw x30, 116(sp)", // t5
    "    sw x31, 120(sp)", // t6
    // Save mepc
    "    csrr t0, mepc",
    "    sw t0, 124(sp)",
    // Call Rust handler with TrapFrame pointer
    "    mv a0, sp",
    "    call trap_handler_rust",
    // Restore mepc
    "    lw t0, 124(sp)",
    "    csrw mepc, t0",
    // Restore all registers
    "    lw x1,   0(sp)",
    "    lw x3,   8(sp)",
    "    lw x4,  12(sp)",
    "    lw x5,  16(sp)",
    "    lw x6,  20(sp)",
    "    lw x7,  24(sp)",
    "    lw x8,  28(sp)",
    "    lw x9,  32(sp)",
    "    lw x10, 36(sp)",
    "    lw x11, 40(sp)",
    "    lw x12, 44(sp)",
    "    lw x13, 48(sp)",
    "    lw x14, 52(sp)",
    "    lw x15, 56(sp)",
    "    lw x16, 60(sp)",
    "    lw x17, 64(sp)",
    "    lw x18, 68(sp)",
    "    lw x19, 72(sp)",
    "    lw x20, 76(sp)",
    "    lw x21, 80(sp)",
    "    lw x22, 84(sp)",
    "    lw x23, 88(sp)",
    "    lw x24, 92(sp)",
    "    lw x25, 96(sp)",
    "    lw x26, 100(sp)",
    "    lw x27, 104(sp)",
    "    lw x28, 108(sp)",
    "    lw x29, 112(sp)",
    "    lw x30, 116(sp)",
    "    lw x31, 120(sp)",
    "    lw x2,   4(sp)", // reload post-addi sp (no-op vs current); then addi below
    "    addi sp, sp, 128",
    // Return from trap
    "    mret",
);

// ---------------------------------------------------------------------------
// CSR register access
// ---------------------------------------------------------------------------

/// Machine cause register (read-only view of trap cause).
#[inline]
fn read_mcause() -> usize {
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    {
        let cause: usize;
        unsafe {
            core::arch::asm!("csrr {}, mcause", out(reg) cause);
        }
        cause
    }

    #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
    {
        0
    }
}

/// Checks if the trap was an interrupt (vs. exception).
#[inline]
fn is_interrupt(mcause: usize) -> bool {
    (mcause & (1usize << (usize::BITS - 1))) != 0
}

/// Extracts the exception/interrupt code from mcause.
#[inline]
fn cause_code(mcause: usize) -> usize {
    mcause & !(1usize << (usize::BITS - 1))
}

/// Halts the CPU after an unrecoverable trap.
#[inline]
fn halt() -> ! {
    loop {
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        unsafe {
            core::arch::asm!("wfi");
        }

        #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
        core::hint::spin_loop();
    }
}

/// Dispatches a trap using the current `mcause` value.
///
/// Board-specific trap entries use this when they provide their own register
/// save/restore assembly (for example the BL808 C906 RV64 entry).
#[no_mangle]
pub extern "C" fn dispatch_current_trap() {
    let mcause = read_mcause();

    if is_interrupt(mcause) {
        match cause_code(mcause) {
            3 => handle_software_interrupt(),
            7 => handle_timer_interrupt(),
            11 => handle_external_interrupt(),
            _ => halt(),
        }
    } else {
        halt();
    }
}

/// Dispatches an interrupt delivered directly by a CLIC vector entry.
///
/// CLIC external interrupts carry the device IRQ number in the vector cause;
/// unlike PLIC they do not require claim/complete MMIO operations.
#[no_mangle]
pub extern "C" fn dispatch_clic_irq(irq: usize) {
    match irq {
        3 => handle_software_interrupt(),
        7 => handle_timer_interrupt(),
        _ => dispatch_registered_irq(irq as u32),
    }
}

fn dispatch_registered_irq(irq: u32) {
    if irq == 0 {
        return;
    }

    let handlers = IRQ_HANDLERS.lock();
    if let Some((_, handler)) = handlers.iter().find(|(n, _)| *n == irq) {
        let handler = *handler;
        drop(handlers);
        handler(irq);
    }
}

// ---------------------------------------------------------------------------
// Rust trap dispatcher
// ---------------------------------------------------------------------------

/// Rust entry point for trap handling.
///
/// Called from assembly `trap_handler` with a pointer to the saved TrapFrame.
/// Dispatches to specific handlers based on mcause.
#[no_mangle]
extern "C" fn trap_handler_rust(_frame: &mut TrapFrame) {
    dispatch_current_trap();
}

// ---------------------------------------------------------------------------
// Interrupt handlers
// ---------------------------------------------------------------------------

/// Handles machine software interrupt (MSI, code 3).
fn handle_software_interrupt() {
    // Call registered handler if available
    if let Some(handler) = *SOFTWARE_HANDLER.lock() {
        handler();
    }
    // If no handler is registered, just return
    // (Caller must clear the interrupt source manually)
}

/// Handles machine timer interrupt (MTI, code 7).
fn handle_timer_interrupt() {
    // Call registered handler if available
    if let Some(handler) = *TIMER_HANDLER.lock() {
        handler();
    }
    // If no handler is registered, just return
    // (Caller must clear the interrupt source manually)
}

/// Handles machine external interrupt (MEI, code 11).
fn handle_external_interrupt() {
    // Claim the interrupt from the platform
    let claim_fn = CLAIM_HANDLER.lock();
    let irq = if let Some(claim) = *claim_fn {
        drop(claim_fn);
        claim()
    } else {
        // No claim handler registered - can't proceed
        return;
    };

    if irq == 0 {
        // Spurious interrupt
        return;
    }

    // Call registered handler if available
    dispatch_registered_irq(irq);

    // Signal completion to the platform
    let complete_fn = COMPLETE_HANDLER.lock();
    if let Some(complete) = *complete_fn {
        drop(complete_fn);
        complete(irq);
    }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Initializes the trap vector to point to the assembly `trap_handler`.
///
/// This must be called before enabling any interrupts.
pub fn init() {
    #[cfg(all(target_arch = "riscv32", not(target_feature = "e")))]
    unsafe {
        core::arch::asm!("la t0, trap_handler", "csrw mtvec, t0",);
    }
}

/// Enables machine external interrupts (MEI) from PLIC.
pub fn enable_external_interrupts() {
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        // Set MEIE bit (bit 11) in mie register.
        let mask: usize = 1 << 11;
        core::arch::asm!("csrs mie, {}", in(reg) mask);
    }
}

/// Enables machine timer interrupts (MTI, `mie.MTIE`).
pub fn enable_timer_interrupt() {
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        // Set MTIE bit (bit 7) in mie register.
        let mask: usize = 1 << 7;
        core::arch::asm!("csrs mie, {}", in(reg) mask);
    }
}

/// Enables global interrupts by setting MIE bit in mstatus.
pub fn enable_global_interrupts() {
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        // Set MIE bit (bit 3) in mstatus register.
        let mask: usize = 1 << 3;
        core::arch::asm!("csrs mstatus, {}", in(reg) mask);
    }
}

#[cfg(test)]
mod tests {
    use super::{cause_code, is_interrupt};

    #[test]
    fn decodes_the_architecture_width_interrupt_bit() {
        let interrupt_bit = 1usize << (usize::BITS - 1);
        let mcause = interrupt_bit | 11;

        assert!(is_interrupt(mcause));
        assert_eq!(cause_code(mcause), 11);
        assert!(!is_interrupt(2));
        assert_eq!(cause_code(2), 2);
    }
}
