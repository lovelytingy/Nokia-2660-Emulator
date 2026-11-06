/*
 * Unisoc T107 (UMS9117) Clean-Room BootROM Assembly
 * Target: ARM Cortex-A7 (ARMv7-A)
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

    .syntax unified
    .cpu cortex-a7
    .arm

    .section .vectors, "ax"
    .global _start
    .global reset_vector

_start:
reset_vector:
    ldr pc, =reset_handler
    ldr pc, =hang
    ldr pc, =hang
    ldr pc, =hang
    ldr pc, =hang
    .word 0
    ldr pc, =hang
    ldr pc, =hang

    .text
    .align 4
    .global reset_handler

reset_handler:
    /* Set CPU mode to Supervisor (SVC), disable IRQ and FIQ */
    cpsid if, #0x13

    /* Set up stack pointer in internal SRAM (IRAM top at 0x0000FFF0) */
    ldr sp, =0x0000FFF0

    /* Initialize UART0 for BootROM status prints */
    ldr r0, =str_banner
    bl uart_print_str

    /* Check Boot Strap 1: Keypad Matrix Controller STATUS0 for '*' key (Bit 10) */
    ldr r1, =0x5006001C     /* KPD_STATUS0 */
    ldr r2, [r1]
    tst r2, #(1 << 10)      /* Bit 10 = Nokia '*' key */
    bne enter_bsl_mode

    /* Check Boot Strap 2: GPIO Bank 0 Pin 1 (Strap / USB VBUS) */
    ldr r1, =0x50040000     /* GPIO_DATA */
    ldr r2, [r1]
    tst r2, #0x02           /* Pin 1 */
    bne enter_bsl_mode

normal_boot_mode:
    ldr r0, =str_normal_boot
    bl uart_print_str

    /* Probe SPI NAND Flash Block 0 at memory-mapped window 0x60000000 */
    ldr r4, =0x60000000     /* Flash direct read window base */
    ldr r5, [r4]            /* Read first 4 bytes (Magic) */
    ldr r6, =0x53505244     /* "SPRD" Magic */
    cmp r5, r6
    beq boot0_valid

    /* Magic check failed, try secondary magic "BIMG" (0x42494D47) */
    ldr r6, =0x42494D47
    cmp r5, r6
    beq boot0_valid

    /* No valid header in SPI NAND Block 0, fall back to BSL recovery */
    ldr r0, =str_nand_invalid
    bl uart_print_str
    b enter_bsl_mode

boot0_valid:
    /* Valid SPRD BOOT0 found */
    /* Header layout:
     * offset 0x00: magic (0x53505244)
     * offset 0x04: version
     * offset 0x08: load_addr (e.g. 0x00006200)
     * offset 0x0C: image_size (bytes)
     * offset 0x10: entry_point
     * offset 0x14: checksum
     * offset 0x20: payload start
     */
    ldr r7, [r4, #0x08]     /* Target SRAM load address */
    ldr r8, [r4, #0x0C]     /* Payload size */
    ldr r9, [r4, #0x10]     /* Entry point */

    /* Basic sanity check: size between 1 and 40KB */
    cmp r8, #0
    beq enter_bsl_mode
    ldr r10, =0x0000A000    /* 40 KB */
    cmp r8, r10
    bhi enter_bsl_mode

    ldr r0, =str_boot0_loading
    bl uart_print_str

    /* Copy BOOT0 payload from Flash (0x60000020) to SRAM load_addr */
    add r1, r4, #0x20       /* Source pointer */
    mov r2, r7              /* Destination pointer */
    mov r3, r8              /* Byte count */

copy_loop:
    cmp r3, #0
    beq copy_done
    ldrb r0, [r1], #1
    strb r0, [r2], #1
    subs r3, r3, #1
    bne copy_loop

copy_done:
    ldr r0, =str_boot0_jump
    bl uart_print_str

    /* Branch to BOOT0 entry point in SRAM */
    mov r0, #0
    mov r1, #0
    mov lr, pc
    bx r9

    /* If BOOT0 returns, hang cleanly */
    b hang

enter_bsl_mode:
    ldr r0, =str_bsl_mode
    bl uart_print_str

    /* In BSL mode:
     * BootROM signals readiness and checks if FDL1 is loaded at 0x00006200.
     * If valid code exists at 0x00006200, branch to it.
     */
    ldr r1, =0x00006200
    ldr r2, [r1]
    cmp r2, #0
    beq bsl_wait_loop
    cmp r2, #0xFFFFFFFF
    beq bsl_wait_loop

    /* Branch to FDL1 in SRAM */
    ldr r0, =str_fdl1_jump
    bl uart_print_str
    mov r0, #0
    mov lr, pc
    bx r1

bsl_wait_loop:
    /* Loop awaiting external BSL host packets over UART0 */
    nop
    b bsl_wait_loop

hang:
    nop
    b hang

/* Helper: Print null-terminated string to SPRD UART0 (0x50000000) */
uart_print_str:
    push {r1, r2, lr}
    ldr r1, =0x50000000     /* SPRD_TXD */
uart_next_char:
    ldrb r2, [r0], #1
    cmp r2, #0
    beq uart_done
    str r2, [r1]
    b uart_next_char
uart_done:
    pop {r1, r2, pc}

    .section .rodata
str_banner:
    .asciz "\r\n=== Unisoc T107 (UMS9117) BootROM v1.0 ===\r\n"
str_normal_boot:
    .asciz "[BootROM] Normal cold boot. Probing SPI NAND Flash Block 0...\r\n"
str_boot0_loading:
    .asciz "[BootROM] Valid SPRD BOOT0 header found! Copying payload to SRAM...\r\n"
str_boot0_jump:
    .asciz "[BootROM] BOOT0 staged successfully. Handing off execution...\r\n"
str_nand_invalid:
    .asciz "[BootROM] Warning: No valid BOOT0 in SPI NAND Block 0. Fallback to BSL mode...\r\n"
str_bsl_mode:
    .asciz "[BootROM] Entering Spreadtrum BSL download protocol on UART0...\r\n"
str_fdl1_jump:
    .asciz "[BootROM] FDL1 payload detected at 0x00006200. Branching to FDL1...\r\n"
