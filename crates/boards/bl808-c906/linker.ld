/* BL808 C906 (riscv64imac) Linker Script - TEMPLATE */
/*
 * TODO: This is a PLACEHOLDER memory map!
 *
 * Required Information:
 * 1. Main memory base address and size (typically larger than E902/E907)
 * 2. Cache configuration (I-cache, D-cache sizes and settings)
 * 3. MMIO peripheral base addresses (UART, GPIO, timers, etc.)
 * 4. Shared memory regions with E902 and E907 cores
 * 5. Boot sequence (OpenSBI/M-mode vs S-mode entry)
 *
 * Resources needed:
 * - BL808 datasheet
 * - C906 core technical reference manual (T-Head/Alibaba)
 * - Bouffalo Lab SDK linker scripts
 */

MEMORY
{
    /* TODO: Replace with actual memory map */
    /* C906 typically has more memory and no separate TCM */
    RAM (rwx)  : ORIGIN = 0x50000000, LENGTH = 16M   /* Main RAM - VERIFY! */
    /* MMIO regions not defined yet - need peripheral addresses */
}

/* Entry point */
ENTRY(_start)

SECTIONS
{
    /* Code section */
    .text : {
        *(.text.start)    /* _start entry point */
        *(.text .text.*)
    } > RAM

    /* Read-only data */
    .rodata : {
        *(.rodata .rodata.*)
    } > RAM

    /* Initialized data */
    .data : {
        _sdata = .;
        *(.data .data.*)
        _edata = .;
    } > RAM

    /* Zero-initialized data */
    .bss : {
        _sbss = .;
        *(.bss .bss.*)
        *(COMMON)
        _ebss = .;
    } > RAM

    /* Stack (grows downward) */
    .stack (NOLOAD) : {
        _stack_bottom = .;
        . = . + 64K;  /* 64K stack - larger for 64-bit architecture */
        _stack_top = .;
    } > RAM

    /* Heap */
    .heap (NOLOAD) : {
        _heap_start = .;
        . = ORIGIN(RAM) + LENGTH(RAM);
        _heap_end = .;
    } > RAM

    /* Discard unwanted sections */
    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}
