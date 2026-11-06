/*
 * Unisoc / Spreadtrum SPI Controller Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qemu/log.h"
#include "qemu/module.h"
#include "migration/vmstate.h"
#include "hw/ssi/sprd_spi.h"

static uint64_t sprd_spi_read(void *opaque, hwaddr offset, unsigned size)
{
    SprdSPIState *s = opaque;
    uint32_t val = 0;

    switch (offset) {
    case SPRD_SPI_CLKD:
        val = s->clkd;
        break;

    case SPRD_SPI_CTL0:
        val = s->ctl0;
        break;

    case SPRD_SPI_CTL1:
        val = s->ctl1;
        break;

    case SPRD_SPI_CTL4:
        val = s->ctl4;
        break;

    case SPRD_SPI_STS1:
        /* Always ready: TX FIFO empty, not busy */
        val = SPRD_SPI_STS1_TX_EMPTY;
        break;

    case SPRD_SPI_STS2:
        val = 0;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_spi: read at unhandled offset 0x%" HWADDR_PRIx "\n",
                      offset);
        break;
    }

    return val;
}

static void sprd_spi_write(void *opaque, hwaddr offset, uint64_t val, unsigned size)
{
    SprdSPIState *s = opaque;

    switch (offset) {
    case SPRD_SPI_TXD:
        /* Transmit byte or word to attached slave */
        if (s->lcd) {
            uint8_t byte = (uint8_t)(val & 0xFF);
            if (s->dc_data_mode) {
                gc9307_write_data(s->lcd, byte);
            } else {
                gc9307_write_cmd(s->lcd, byte);
            }
        }
        break;

    case SPRD_SPI_CLKD:
        s->clkd = (uint32_t)val;
        break;

    case SPRD_SPI_CTL0:
        s->ctl0 = (uint32_t)val;
        break;

    case SPRD_SPI_CTL1:
        s->ctl1 = (uint32_t)val;
        break;

    case SPRD_SPI_CTL4:
        s->ctl4 = (uint32_t)val;
        /* Bit 0 in CTL4 can be used to set D/C line directly */
        s->dc_data_mode = (val & 0x1) != 0;
        break;

    case SPRD_SPI_INT_EN:
        s->int_en = (uint32_t)val;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_spi: write at unhandled offset 0x%" HWADDR_PRIx
                      " val 0x%" PRIx64 "\n", offset, val);
        break;
    }
}

static const MemoryRegionOps sprd_spi_ops = {
    .read = sprd_spi_read,
    .write = sprd_spi_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = {
        .min_access_size = 1,
        .max_access_size = 4,
    },
};

static void sprd_spi_reset(DeviceState *dev)
{
    SprdSPIState *s = SPRD_SPI(dev);

    s->clkd = 0;
    s->ctl0 = 0;
    s->ctl1 = 0;
    s->ctl4 = 0;
    s->int_en = 0;
    s->sts1 = SPRD_SPI_STS1_TX_EMPTY;
    s->dc_data_mode = false;
}

static void sprd_spi_realize(DeviceState *dev, Error **errp)
{
    SprdSPIState *s = SPRD_SPI(dev);

    memory_region_init_io(&s->iomem, OBJECT(dev), &sprd_spi_ops, s,
                          "sprd-spi", SPRD_SPI_REGS_SIZE);
    sysbus_init_mmio(SYS_BUS_DEVICE(dev), &s->iomem);
    sysbus_init_irq(SYS_BUS_DEVICE(dev), &s->irq);
}

static void sprd_spi_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->realize = sprd_spi_realize;
    dc->reset = sprd_spi_reset;
}

static const TypeInfo sprd_spi_info = {
    .name = TYPE_SPRD_SPI,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(SprdSPIState),
    .class_init = sprd_spi_class_init,
};

static void sprd_spi_register_types(void)
{
    type_register_static(&sprd_spi_info);
}

type_init(sprd_spi_register_types)

DeviceState *sprd_spi_create(hwaddr addr, qemu_irq irq)
{
    DeviceState *dev;
    SysBusDevice *sbd;

    dev = qdev_new(TYPE_SPRD_SPI);
    sbd = SYS_BUS_DEVICE(dev);
    sysbus_realize_and_unref(sbd, &error_fatal);
    sysbus_mmio_map(sbd, 0, addr);
    sysbus_connect_irq(sbd, 0, irq);

    return dev;
}
