/*
 * Startup code for N2660Emu baremetal verification payload
 * ARM Cortex-A7 (ARMv7-A)
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

    .syntax unified
    .arch armv7-a
    .arm

    .section .vectors, "ax"
    .global _start
    .global Reset_Handler

_start:
    b Reset_Handler       /* Reset */
    b Undef_Handler       /* Undefined Instruction */
    b SVC_Handler         /* Supervisor Call */
    b Prefetch_Abort      /* Prefetch Abort */
    b Data_Abort          /* Data Abort */
    b .                   /* Reserved */
    b IRQ_Handler         /* IRQ */
    b FIQ_Handler         /* FIQ */

    .section .text
    .type Reset_Handler, %function
Reset_Handler:
    /* Set CPU mode to Supervisor (SVC) and disable IRQ/FIQ */
    cpsid if, #0x13

    /* Set Vector Base Address Register (VBAR) to _start */
    ldr r0, =_start
    mcr p15, 0, r0, c12, c0, 0
    isb

    /* Set up IRQ mode stack pointer */
    cps #0x12
    ldr sp, =_irq_stack_top

    /* Switch back to Supervisor mode and set up SVC stack */
    cps #0x13
    ldr sp, =_stack_top

    /* Preserve boot/kernel handoff registers (r0, r1, r2) */
    mov r4, r0
    mov r5, r1
    mov r6, r2

    /* Zero out .bss */
    ldr r0, =_sbss
    ldr r1, =_ebss
    mov r2, #0
1:
    cmp r0, r1
    strlt r2, [r0], #4
    blt 1b

    /* Enable VFP / NEON (CP10 and CP11 full access) */
    mrc p15, 0, r0, c1, c0, 2
    orr r0, r0, #(0xf << 20)
    mcr p15, 0, r0, c1, c0, 2
    isb
    mov r0, #(1 << 30)      /* Enable VFP in FPEXC */
    vmsr fpexc, r0

    /* Pass boot handoff registers to main(r0, r1, r2) */
    mov r0, r4
    mov r1, r5
    mov r2, r6

    /* Jump to C entry point */
    bl main

    /* If main returns, halt CPU */
halt_loop:
    wfi
    b halt_loop

Undef_Handler:
    b .

SVC_Handler:
    movs pc, lr

Prefetch_Abort:
    b .

Data_Abort:
    b .

    .global IRQ_Handler
    .type IRQ_Handler, %function
IRQ_Handler:
    sub lr, lr, #4
    push {r0-r3, r12, lr}
    bl c_irq_handler
    pop {r0-r3, r12, lr}
    movs pc, lr

FIQ_Handler:
    b .
