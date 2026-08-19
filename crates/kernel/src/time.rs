//! Timekeeping facade for the kernel.
//!
//! The kernel itself owns no clock hardware. Instead, the board registers a
//! monotonic uptime source (a function returning seconds) via
//! [`register_uptime_source`]; the shell's `uptime` command consumes it
//! through [`uptime_seconds`]. This keeps the layering: platforms keep their
//! timer/CLINT access inside `boards/*`, the kernel only reads a plain value.

use spin::Mutex;

/// Registered uptime source (returns seconds since boot).
///
/// `None` means no platform has provided a clock yet; `uptime` reports that
/// the timer is unimplemented.
static UPTIME_SOURCE: Mutex<Option<fn() -> u64>> = Mutex::new(None);

/// Registers the platform's monotonic uptime source.
///
/// # Arguments
/// * `source` - Function returning seconds elapsed since kernel start.
///
/// This must be called once at boot, before the shell starts using `uptime`.
pub fn register_uptime_source(source: fn() -> u64) {
    *UPTIME_SOURCE.lock() = Some(source);
}

/// Returns the uptime in seconds, if a source has been registered.
pub fn uptime_seconds() -> Option<u64> {
    let source = *UPTIME_SOURCE.lock();
    source.map(|f| f())
}
