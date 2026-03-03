/* BL808 E902 (riscv32emc) Linker Script - TEMPLATE */
/*
 * TODO: This is a PLACEHOLDER memory map!
 *
 * Required Information:
 * 1. ITCM (Instruction Tightly-Coupled Memory) base address and size
 * 2. DTCM (Data Tightly-Coupled Memory) base address and size
 * 3. RAM base address and size
 * 4. MMIO peripheral base addresses (UART, GPIO, timers, etc.)
 * 5. Boot ROM location (if applicable)
 *
 * Resources needed:
 * - BL808 datasheet
 * - E902 core technical reference manual
 * - Bouffalo Lab SDK linker scripts
 */

MEMORY
{
    /* TODO: Replace with actual memory map */
    ITCM (rx)  : ORIGIN = 0x22010000, LENGTH = 48K   /* Instruction TCM - VERIFY! */
    DTCM (rw)  : ORIGIN = 0x2201C000, LENGTH = 16K   /* Data TCM - VERIFY! */
    RAM (rwx)  : ORIGIN = 0x62020000, LENGTH = 64K   /* Main RAM - VERIFY! */
    /* MMIO regions not defined yet - need peripheral addresses */
}

/* Entry point */
ENTRY(_start)

SECTIONS
{
    /* Code section in ITCM for fast execution */
    .text : {
        *(.text.start)    /* _start entry point */
        *(.text .text.*)
    } > ITCM

    /* Read-only data can go in ITCM or RAM */
    .rodata : {
        *(.rodata .rodata.*)
    } > ITCM

    /* Initialized data in DTCM */
    .data : {
        _sdata = .;
        *(.data .data.*)
        _edata = .;
    } > DTCM

    /* Zero-initialized data in DTCM */
    .bss : {
        _sbss = .;
        *(.bss .bss.*)
        *(COMMON)
        _ebss = .;
    } > DTCM

    /* Stack in DTCM (grows downward) */
    .stack (NOLOAD) : {
        _stack_bottom = .;
        . = . + 8K;  /* 8K stack - adjust as needed */
        _stack_top = .;
    } > DTCM

    /* Heap in main RAM if needed */
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
