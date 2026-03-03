//! Memory layout types for hardware abstraction.
//!
//! This module defines types for describing the memory layout of the system.

use core::ops::Range;

/// Describes a region of memory.
///
/// This type represents a contiguous region of memory with a start and end address.
pub type MemoryRegion = Range<usize>;

/// Complete memory layout of the system.
///
/// This structure describes all major memory regions used by the kernel,
/// typically populated from linker script symbols.
#[derive(Debug, Clone)]
pub struct MemoryLayout {
    /// Heap region (for dynamic allocation)
    pub heap: MemoryRegion,

    /// Stack region (for function call frames)
    pub stack: MemoryRegion,

    /// Text region (executable code)
    pub text: MemoryRegion,

    /// Data region (initialized read-write data)
    pub data: MemoryRegion,

    /// BSS region (zero-initialized data)
    pub bss: MemoryRegion,
}

impl MemoryLayout {
    /// Create a new memory layout.
    ///
    /// # Arguments
    /// * `heap` - Heap memory range
    /// * `stack` - Stack memory range
    /// * `text` - Text (code) memory range
    /// * `data` - Data section memory range
    /// * `bss` - BSS section memory range
    pub const fn new(
        heap: MemoryRegion,
        stack: MemoryRegion,
        text: MemoryRegion,
        data: MemoryRegion,
        bss: MemoryRegion,
    ) -> Self {
        Self {
            heap,
            stack,
            text,
            data,
            bss,
        }
    }

    /// Get the total memory range covered by this layout.
    ///
    /// # Returns
    /// A range from the lowest address to the highest address in the layout.
    pub fn total_range(&self) -> MemoryRegion {
        let min = self.text.start
            .min(self.data.start)
            .min(self.bss.start)
            .min(self.stack.start)
            .min(self.heap.start);

        let max = self.text.end
            .max(self.data.end)
            .max(self.bss.end)
            .max(self.stack.end)
            .max(self.heap.end);

        min..max
    }
}
