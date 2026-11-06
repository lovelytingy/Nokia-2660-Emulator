/*
 * Linker script for N2660Emu baremetal verification payload
 *
 * Targets Unisoc T107 DRAM (0x80000000) with stack in SRAM / DRAM.
 */

ENTRY(Reset_Handler)

MEMORY
{
    SRAM (rwx) : ORIGIN = 0x00000000, LENGTH = 64K
    DRAM (rwx) : ORIGIN = 0x80000000, LENGTH = 48M
}

SECTIONS
{
    . = 0x80000000;

    .text : {
        *(.vectors)
        *(.text*)
        *(.rodata*)
        . = ALIGN(4);
    } > DRAM

    .data : {
        _sdata = .;
        *(.data*)
        . = ALIGN(4);
        _edata = .;
    } > DRAM

    .bss (NOLOAD) : {
        _sbss = .;
        *(.bss*)
        *(COMMON)
        . = ALIGN(4);
        _ebss = .;
    } > DRAM

    /* Stack region (16 KB) at top of initial 1MB */
    .stack (NOLOAD) : {
        . = ALIGN(8);
        _stack_bottom = .;
        . += 0x4000;
        _stack_top = .;
    } > DRAM

    /* IRQ Stack region (4 KB) */
    .irq_stack (NOLOAD) : {
        . = ALIGN(8);
        _irq_stack_bottom = .;
        . += 0x1000;
        _irq_stack_top = .;
    } > DRAM

    _end = .;
}
