/*
 * Unisoc T107 (UMS9117) SPRD Interrupt Controller (INTC) Baremetal Driver
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "intc.h"

static irq_handler_t irq_handlers[32];
static volatile uint32_t irq_counter[32];

void intc_init(void)
{
    REG_INTC_IRQ_EN_CLR = 0xFFFFFFFF;
    REG_INTC_FIQ_EN_CLR = 0xFFFFFFFF;
    REG_INTC_IRQ_SOFT_CLR = 0xFFFFFFFF;
    REG_INTC_FIQ_SOFT_CLR = 0xFFFFFFFF;
    REG_INTC_IRQ_CLR = 0xFFFFFFFF;
    REG_INTC_TYPE = 0x00000000;

    for (int i = 0; i < 32; i++) {
        irq_handlers[i] = NULL;
        irq_counter[i] = 0;
    }
}

void intc_enable_irq(uint32_t irq_num)
{
    if (irq_num < 32) {
        REG_INTC_IRQ_EN = (1U << irq_num);
    }
}

void intc_disable_irq(uint32_t irq_num)
{
    if (irq_num < 32) {
        REG_INTC_IRQ_EN_CLR = (1U << irq_num);
    }
}

uint32_t intc_get_raw_status(void)
{
    return REG_INTC_IRQ_RAW_STS;
}

uint32_t intc_get_masked_status(void)
{
    return REG_INTC_IRQ_MASK_STS;
}

void intc_trigger_soft_irq(uint32_t irq_num)
{
    if (irq_num < 32) {
        REG_INTC_IRQ_SOFT = (1U << irq_num);
    }
}

void intc_clear_soft_irq(uint32_t irq_num)
{
    if (irq_num < 32) {
        REG_INTC_IRQ_SOFT_CLR = (1U << irq_num);
    }
}

void intc_clear_irq(uint32_t irq_num)
{
    if (irq_num < 32) {
        REG_INTC_IRQ_CLR = (1U << irq_num);
    }
}

int intc_register_handler(uint32_t irq_num, irq_handler_t handler)
{
    if (irq_num >= 32) {
        return -1;
    }
    irq_handlers[irq_num] = handler;
    return 0;
}

uint32_t intc_get_irq_count(uint32_t irq_num)
{
    if (irq_num < 32) {
        return irq_counter[irq_num];
    }
    return 0;
}

void cpu_enable_irqs(void)
{
    __asm__ volatile ("cpsie i" ::: "memory");
}

void cpu_disable_irqs(void)
{
    __asm__ volatile ("cpsid i" ::: "memory");
}

uint32_t cpu_get_cpsr(void)
{
    uint32_t cpsr;
    __asm__ volatile ("mrs %0, cpsr" : "=r" (cpsr));
    return cpsr;
}

void c_irq_handler(void)
{
    uint32_t status = REG_INTC_IRQ_MASK_STS;

    for (int i = 0; i < 32; i++) {
        if (status & (1U << i)) {
            irq_counter[i]++;
            if (irq_handlers[i]) {
                irq_handlers[i]();
            }
            REG_INTC_IRQ_SOFT_CLR = (1U << i);
            REG_INTC_IRQ_CLR = (1U << i);
        }
    }
}
