/*
 * Unisoc / Spreadtrum System Timer Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_TIMER_SPRD_TIMER_H
#define HW_TIMER_SPRD_TIMER_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "hw/ptimer.h"
#include "qom/object.h"

#define TYPE_SPRD_TIMER "sprd-timer"
OBJECT_DECLARE_SIMPLE_TYPE(SprdTimerState, SPRD_TIMER)

/* Register Offsets */
#define SPRD_TIMER_LOAD_LO       0x0000
#define SPRD_TIMER_LOAD_HI       0x0004
#define SPRD_TIMER_VALUE_LO      0x0008
#define SPRD_TIMER_VALUE_HI      0x000C
#define SPRD_TIMER_CTL           0x0010
#define SPRD_TIMER_INT           0x0014
#define SPRD_TIMER_SHDW_LO       0x0018
#define SPRD_TIMER_SHDW_HI       0x001C

#define SPRD_TIMER_REGS_SIZE     0x1000

/* Control Register Bits */
#define SPRD_TIMER_CTL_PERIOD    (1 << 0)
#define SPRD_TIMER_CTL_ENABLE    (1 << 1)
#define SPRD_TIMER_CTL_64BIT     (1 << 16)

/* Interrupt Register Bits */
#define SPRD_TIMER_INT_EN        (1 << 0)
#define SPRD_TIMER_INT_RAW_STS   (1 << 1)
#define SPRD_TIMER_INT_MASK_STS  (1 << 2)
#define SPRD_TIMER_INT_CLR       (1 << 3)

struct SprdTimerState {
    /*< private >*/
    SysBusDevice parent_obj;

    /*< public >*/
    MemoryRegion iomem;
    qemu_irq irq;
    ptimer_state *ptimer;

    uint32_t load_lo;
    uint32_t load_hi;
    uint32_t value_lo;
    uint32_t value_hi;
    uint32_t ctl;
    uint32_t int_reg;
};

DeviceState *sprd_timer_create(hwaddr addr, qemu_irq irq);

#endif /* HW_TIMER_SPRD_TIMER_H */
