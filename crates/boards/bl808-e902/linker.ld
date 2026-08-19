/* BL808 E902 linker layout derived from SDK bl808_flash_m0.ld. */

OUTPUT_ARCH(riscv)
ENTRY(_start)

/* The SDK image header/boot ROM wrapper is not part of this ELF yet. */
MEMORY {
    XIP  (rx)  : ORIGIN = 0x58000000, LENGTH = 32M
    ITCM (rx)  : ORIGIN = 0x62020000, LENGTH = 20K
    DTCM (rwx) : ORIGIN = 0x62025000, LENGTH = 4K
    RAM  (rwx) : ORIGIN = 0x6202A000, LENGTH = 24K
    WRAM (rwx) : ORIGIN = 0x22030000, LENGTH = 160K
    XRAM (rwx) : ORIGIN = 0x40000000, LENGTH = 16K
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

    _sidata = ALIGN(., 4);
    .data ORIGIN(RAM) : AT(_sidata) {
        . = ALIGN(4);
        _sdata = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        _edata = ALIGN(4);
    } > RAM

    .bss ALIGN(4) (NOLOAD) : {
        _sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(COMMON)
        _ebss = ALIGN(4);
    } > RAM

    .stack ORIGIN(DTCM) (NOLOAD) : {
        _stack_bottom = .;
        . += 0x400;
        _stack_top = .;
    } > DTCM

    _heap_start = ALIGN(ADDR(.bss) + SIZEOF(.bss), 4);
    _heap_end = ORIGIN(RAM) + LENGTH(RAM);

    /DISCARD/ : {
        *(.eh_frame .eh_frame_hdr)
        *(.comment)
    }
}
