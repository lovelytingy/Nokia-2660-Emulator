/*
 * Unisoc T107 (UMS9117) SPRD Interrupt Controller (INTC) Baremetal Driver
 * Base Address: 0x50050000
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef BAREMETAL_INTC_H
#define BAREMETAL_INTC_H

#include <stdint.h>
#include <stddef.h>

#define SPRD_INTC_BASE          0x50050000

#define REG_INTC_IRQ_RAW_STS    (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0000))
#define REG_INTC_IRQ_MASK_STS   (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0004))
#define REG_INTC_IRQ_EN         (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0008))
#define REG_INTC_IRQ_EN_CLR     (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x000C))
#define REG_INTC_IRQ_SOFT       (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0010))
#define REG_INTC_IRQ_SOFT_CLR   (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0014))
#define REG_INTC_IRQ_CLR        (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0018))
#define REG_INTC_FIQ_RAW_STS    (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x001C))
#define REG_INTC_FIQ_MASK_STS   (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0020))
#define REG_INTC_FIQ_EN         (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0024))
#define REG_INTC_FIQ_EN_CLR     (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0028))
#define REG_INTC_FIQ_SOFT       (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x002C))
#define REG_INTC_FIQ_SOFT_CLR   (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0030))
#define REG_INTC_TYPE           (*(volatile uint32_t *)(SPRD_INTC_BASE + 0x0034))

#define SPRD_IRQ_TIMER          0
#define SPRD_IRQ_UART0          1
#define SPRD_IRQ_UART1          2
#define SPRD_IRQ_UART2          3
#define SPRD_IRQ_GPIO           4
#define SPRD_IRQ_KPD            5
#define SPRD_IRQ_SPI0           6
#define SPRD_IRQ_SPI1           7
#define SPRD_IRQ_SFC            8
#define SPRD_IRQ_AUDIO          9
#define SPRD_IRQ_WDG            10
#define SPRD_IRQ_DMA            11
#define SPRD_IRQ_TEST_SOFT      15

typedef void (*irq_handler_t)(void);

void intc_init(void);
void intc_enable_irq(uint32_t irq_num);
void intc_disable_irq(uint32_t irq_num);
uint32_t intc_get_raw_status(void);
uint32_t intc_get_masked_status(void);
void intc_trigger_soft_irq(uint32_t irq_num);
void intc_clear_soft_irq(uint32_t irq_num);
void intc_clear_irq(uint32_t irq_num);
int intc_register_handler(uint32_t irq_num, irq_handler_t handler);
uint32_t intc_get_irq_count(uint32_t irq_num);

/* CPU Interrupt Controls */
void cpu_enable_irqs(void);
void cpu_disable_irqs(void);
uint32_t cpu_get_cpsr(void);

/* C IRQ dispatcher called from assembly vector */
void c_irq_handler(void);

#endif /* BAREMETAL_INTC_H */
