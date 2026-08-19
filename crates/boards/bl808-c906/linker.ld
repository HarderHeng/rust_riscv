/* BL808 C906/D0 linker layout derived from SDK bl808_flash_d0.ld. */

OUTPUT_ARCH(riscv)
ENTRY(_start)

/* The SDK image header/boot ROM wrapper is not part of this ELF yet. */
MEMORY {
    XIP  (rx)  : ORIGIN = 0x58000000, LENGTH = 32M
    ITCM (rx)  : ORIGIN = 0x3EFF0000, LENGTH = 28K
    DTCM (rwx) : ORIGIN = 0x3EFF7000, LENGTH = 4K
    RAM  (rwx) : ORIGIN = 0x3EFF8000, LENGTH = 64K
    XRAM (rwx) : ORIGIN = 0x40004000, LENGTH = 16K
}

SECTIONS {
    .text : {
        _stext = .;
        KEEP(*(.text.start))
        *(.text .text.*)
        _etext = .;
    } > XIP

    .rodata ALIGN(8) : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } > XIP

    _sidata = ALIGN(., 8);
    .data ORIGIN(RAM) : AT(_sidata) {
        . = ALIGN(8);
        _sdata = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        _edata = ALIGN(8);
    } > RAM

    .bss ALIGN(8) (NOLOAD) : {
        _sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(COMMON)
        _ebss = ALIGN(8);
    } > RAM

    .stack ORIGIN(DTCM) (NOLOAD) : {
        _stack_bottom = .;
        . += 0x400;
        _stack_top = .;
    } > DTCM

    _heap_start = ALIGN(ADDR(.bss) + SIZEOF(.bss), 8);
    _heap_end = ORIGIN(RAM) + LENGTH(RAM);

    /DISCARD/ : {
        *(.eh_frame .eh_frame_hdr)
        *(.comment)
    }
}
