/*
 * Unisoc / Spreadtrum SPI Controller Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_SSI_SPRD_SPI_H
#define HW_SSI_SPRD_SPI_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "hw/display/gc9307.h"
#include "qom/object.h"

#define TYPE_SPRD_SPI "sprd-spi"
OBJECT_DECLARE_SIMPLE_TYPE(SprdSPIState, SPRD_SPI)

/* Register Offsets */
#define SPRD_SPI_TXD          0x0000
#define SPRD_SPI_CLKD         0x0004
#define SPRD_SPI_CTL0         0x0008
#define SPRD_SPI_CTL1         0x000C
#define SPRD_SPI_CTL2         0x0010
#define SPRD_SPI_CTL3         0x0014
#define SPRD_SPI_CTL4         0x0018
#define SPRD_SPI_INT_EN       0x0020
#define SPRD_SPI_INT_CLR      0x0024
#define SPRD_SPI_STS1         0x0030
#define SPRD_SPI_STS2         0x0034

#define SPRD_SPI_REGS_SIZE    0x1000

/* Status Bits */
#define SPRD_SPI_STS1_TX_EMPTY (1 << 8)
#define SPRD_SPI_STS1_BUSY     (1 << 0)

struct SprdSPIState {
    /*< private >*/
    SysBusDevice parent_obj;

    /*< public >*/
    MemoryRegion iomem;
    qemu_irq irq;

    uint32_t clkd;
    uint32_t ctl0;
    uint32_t ctl1;
    uint32_t ctl4;
    uint32_t int_en;
    uint32_t sts1;

    /* Attached display device */
    GC9307State *lcd;
    bool dc_data_mode; /* Data/Command pin state */
};

DeviceState *sprd_spi_create(hwaddr addr, qemu_irq irq);

#endif /* HW_SSI_SPRD_SPI_H */
