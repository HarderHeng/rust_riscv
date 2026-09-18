//! Board-supplied platform identity for shell commands.
//!
//! Boards (or `kernel_main`) register a human-readable name and architecture
//! string so commands like `version` do not hardcode a single RISC-V ISA.

use spin::Mutex;

static PLATFORM_NAME: Mutex<Option<&'static str>> = Mutex::new(None);
static PLATFORM_ARCH: Mutex<Option<&'static str>> = Mutex::new(None);

/// Registers the platform display name and architecture string.
pub fn register_platform_identity(name: &'static str, arch: &'static str) {
    *PLATFORM_NAME.lock() = Some(name);
    *PLATFORM_ARCH.lock() = Some(arch);
}

/// Returns the registered platform name, if any.
pub fn platform_name() -> Option<&'static str> {
    *PLATFORM_NAME.lock()
}

/// Returns the registered architecture string, if any.
pub fn platform_arch() -> Option<&'static str> {
    *PLATFORM_ARCH.lock()
}
