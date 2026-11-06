/*
 * Linker script for Unisoc T107 Clean-Room BootROM
 * Maps directly to 0x00000000 (SRAM / BootROM mapping).
 */

ENTRY(_start)

MEMORY
{
    ROM (rx) : ORIGIN = 0x00000000, LENGTH = 16K
}

SECTIONS
{
    . = 0x00000000;

    .text : {
        *(.vectors)
        *(.text*)
        *(.rodata*)
        . = ALIGN(4);
    } > ROM

    _end = .;
}
