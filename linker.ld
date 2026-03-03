/* QEMU virt machine — riscv32 linker script
 *
 * Memory map (relevant subset):
 *   0x0000_0000  ..  0x0FFF_FFFF   MMIO (UART0 @ 0x1000_0000)
 *   0x8000_0000  ..  0x8800_0000   DRAM 128 MiB  (QEMU -kernel loads here)
 *
 * Address types used below:
 *   VMA  Virtual Memory Address  — address the code uses at runtime
 *   LMA  Load Memory Address     — address the data sits in the ELF image
 *
 * For this bare-metal setup VMA == LMA (everything lives in RAM), but the
 * .data AT> idiom is kept so adding a ROM/flash region later is trivial.
 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY {
    RAM : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS {
    /* ------------------------------------------------------------------ */
    /* .text  — executable code                                            */
    /* ------------------------------------------------------------------ */
    .text ORIGIN(RAM) : {
        KEEP(*(.text.start))    /* _start must be the very first word     */
        *(.text .text.*)
    } > RAM

    /* ------------------------------------------------------------------ */
    /* .rodata  — read-only data (string literals, vtables, …)            */
    /* ------------------------------------------------------------------ */
    .rodata ALIGN(8) : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } > RAM

    /* Mark the end of the read-only image (used if you ever copy .data). */
    _sidata = .;

    /* ------------------------------------------------------------------ */
    /* .data  — initialised read-write data                                */
    /*   LMA = right after .rodata in the flash/RAM image (_sidata)       */
    /*   VMA = same (we are running directly from RAM)                    */
    /* ------------------------------------------------------------------ */
    .data ALIGN(4) : AT(_sidata) {
        _sdata = .;             /* VMA start — copy destination           */
        *(.data .data.*)
        *(.sdata .sdata.*)      /* RISC-V small-data section              */
        . = ALIGN(4);
        _edata = .;             /* VMA end                                */
    } > RAM

    /* ------------------------------------------------------------------ */
    /* .bss  — zero-initialised data (zeroed by _start before kernel_main)*/
    /* ------------------------------------------------------------------ */
    .bss ALIGN(4) (NOLOAD) : {
        _sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(COMMON)
        . = ALIGN(4);
        _ebss = .;
    } > RAM

    /* ------------------------------------------------------------------ */
    /* Stack  — grows downward; placed right after BSS                    */
    /* ------------------------------------------------------------------ */
    .stack ALIGN(16) (NOLOAD) : {
        _stack_bottom = .;
        . += 0x10000;           /* 64 KiB                                 */
        _stack_top = .;
    } > RAM

    /* ------------------------------------------------------------------ */
    /* Heap   — everything that remains is available to an allocator      */
    /* ------------------------------------------------------------------ */
    _heap_start = _stack_top;
    _heap_end   = ORIGIN(RAM) + LENGTH(RAM);

    /* ------------------------------------------------------------------ */
    /* Discard sections that bloat the image and are unused at runtime    */
    /* ------------------------------------------------------------------ */
    /DISCARD/ : {
        *(.eh_frame .eh_frame_hdr)
        *(.note .note.*)
        *(.comment)
    }
}
