/*
 * Unisoc / Spreadtrum GPIO Controller Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_GPIO_SPRD_GPIO_H
#define HW_GPIO_SPRD_GPIO_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qom/object.h"

#define TYPE_SPRD_GPIO "sprd-gpio"
OBJECT_DECLARE_SIMPLE_TYPE(SprdGpioState, SPRD_GPIO)

/* Register Offsets within each bank */
#define SPRD_GPIO_DATA          0x0000  /* Pin Data (Read: in / Write: out) */
#define SPRD_GPIO_DMSK          0x0004  /* Data Mask */
#define SPRD_GPIO_DIR           0x0008  /* Direction (1=Output, 0=Input) */
#define SPRD_GPIO_IS            0x000C  /* Interrupt Sense (1=Level, 0=Edge) */
#define SPRD_GPIO_IBE           0x0010  /* Interrupt Both Edges */
#define SPRD_GPIO_IEV           0x0014  /* Interrupt Event (1=High/Rising, 0=Low/Falling) */
#define SPRD_GPIO_IE            0x0018  /* Interrupt Enable */
#define SPRD_GPIO_RIS           0x001C  /* Raw Interrupt Status */
#define SPRD_GPIO_MIS           0x0020  /* Masked Interrupt Status */
#define SPRD_GPIO_IC            0x0024  /* Interrupt Clear */
#define SPRD_GPIO_INEN          0x0028  /* Input Buffer Enable */

#define SPRD_GPIO_BANK_SIZE     0x0080
#define SPRD_GPIO_NUM_BANKS     8       /* Up to 128 GPIO lines */
#define SPRD_GPIO_TOTAL_SIZE    0x1000

typedef struct SprdGpioBank {
    uint32_t data_out;
    uint32_t data_in;
    uint32_t dmsk;
    uint32_t dir;
    uint32_t is;
    uint32_t ibe;
    uint32_t iev;
    uint32_t ie;
    uint32_t ris;
    uint32_t inen;
} SprdGpioBank;

struct SprdGpioState {
    /*< private >*/
    SysBusDevice parent_obj;

    /*< public >*/
    MemoryRegion iomem;
    qemu_irq irq;

    SprdGpioBank banks[SPRD_GPIO_NUM_BANKS];
};

DeviceState *sprd_gpio_create(hwaddr addr, qemu_irq irq);

#endif /* HW_GPIO_SPRD_GPIO_H */
