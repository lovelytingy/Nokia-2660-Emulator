/*
 * Unisoc SPRD Interrupt Controller (INTC) Model
 * Base Address: 0x50050000
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_INTC_SPRD_INTC_H
#define HW_INTC_SPRD_INTC_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qom/object.h"

#define TYPE_SPRD_INTC "sprd-intc"
OBJECT_DECLARE_SIMPLE_TYPE(SprdIntcState, SPRD_INTC)

#define SPRD_INTC_NUM_IRQS 32

/* Register Offsets (Base: 0x50050000) */
#define SPRD_INTC_IRQ_RAW_STS   0x0000  /* Raw interrupt status (RO) */
#define SPRD_INTC_IRQ_MASK_STS  0x0004  /* Masked interrupt status (RO) */
#define SPRD_INTC_IRQ_EN        0x0008  /* Interrupt enable / set (RW) */
#define SPRD_INTC_IRQ_EN_CLR    0x000C  /* Interrupt enable clear (WO) */
#define SPRD_INTC_IRQ_SOFT      0x0010  /* Software interrupt trigger (RW) */
#define SPRD_INTC_IRQ_SOFT_CLR  0x0014  /* Software interrupt clear (WO) */
#define SPRD_INTC_IRQ_CLR       0x0018  /* Edge interrupt clear (WO) */
#define SPRD_INTC_FIQ_RAW_STS   0x001C  /* FIQ raw status (RO) */
#define SPRD_INTC_FIQ_MASK_STS  0x0020  /* FIQ masked status (RO) */
#define SPRD_INTC_FIQ_EN        0x0024  /* FIQ enable / set (RW) */
#define SPRD_INTC_FIQ_EN_CLR    0x0028  /* FIQ enable clear (WO) */
#define SPRD_INTC_FIQ_SOFT      0x002C  /* FIQ soft trigger (RW) */
#define SPRD_INTC_FIQ_SOFT_CLR  0x0030  /* FIQ soft clear (WO) */
#define SPRD_INTC_TYPE          0x0034  /* Interrupt type: 0=IRQ, 1=FIQ (RW) */

/* Hardware IRQ Channel Assignments for Unisoc T107 */
#define SPRD_IRQ_TIMER          0   /* System Timer Tick */
#define SPRD_IRQ_UART0          1   /* Primary Debug/BSL UART */
#define SPRD_IRQ_UART1          2   /* Secondary UART */
#define SPRD_IRQ_UART2          3   /* Modem/BT UART */
#define SPRD_IRQ_GPIO           4   /* GPIO Controller & Flip Sensor */
#define SPRD_IRQ_KPD            5   /* Keypad Matrix Controller */
#define SPRD_IRQ_SPI0           6   /* SPI0 / Primary GC9307 LCD */
#define SPRD_IRQ_SPI1           7   /* SPI1 / Secondary ST7735 Sub-LCD */
#define SPRD_IRQ_SFC            8   /* Serial Flash Controller (SPI NAND) */
#define SPRD_IRQ_AUDIO          9   /* SPRD Audio Subsystem & Tone Buzzer */
#define SPRD_IRQ_WDG            10  /* Watchdog Timer */
#define SPRD_IRQ_DMA            11  /* DMA Engine */

struct SprdIntcState {
    /*< private >*/
    SysBusDevice parent_obj;

    /*< public >*/
    MemoryRegion iomem;
    qemu_irq parent_irq;
    qemu_irq parent_fiq;

    uint32_t irq_raw_sts;
    uint32_t irq_en;
    uint32_t irq_soft;
    uint32_t fiq_raw_sts;
    uint32_t fiq_en;
    uint32_t fiq_soft;
    uint32_t int_type;
};

#endif /* HW_INTC_SPRD_INTC_H */
