/*
 * Unisoc / Spreadtrum GPIO Controller Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qemu/log.h"
#include "qemu/module.h"
#include "migration/vmstate.h"
#include "hw/gpio/sprd_gpio.h"

static void sprd_gpio_update_irq(SprdGpioState *s, int bank_idx)
{
    SprdGpioBank *bank = &s->banks[bank_idx];
    bool level = false;

    if (bank->ris & bank->ie) {
        level = true;
    }
    qemu_set_irq(s->irq, level);
}

static uint64_t sprd_gpio_read(void *opaque, hwaddr offset, unsigned size)
{
    SprdGpioState *s = opaque;
    int bank_idx = offset / SPRD_GPIO_BANK_SIZE;
    hwaddr reg_off = offset % SPRD_GPIO_BANK_SIZE;

    if (bank_idx >= SPRD_GPIO_NUM_BANKS) {
        return 0;
    }

    SprdGpioBank *b = &s->banks[bank_idx];
    uint32_t val = 0;

    switch (reg_off) {
    case SPRD_GPIO_DATA:
        /* Pins configured as output return data_out, inputs return data_in */
        val = (b->data_out & b->dir) | (b->data_in & ~b->dir);
        break;

    case SPRD_GPIO_DMSK:
        val = b->dmsk;
        break;

    case SPRD_GPIO_DIR:
        val = b->dir;
        break;

    case SPRD_GPIO_IS:
        val = b->is;
        break;

    case SPRD_GPIO_IBE:
        val = b->ibe;
        break;

    case SPRD_GPIO_IEV:
        val = b->iev;
        break;

    case SPRD_GPIO_IE:
        val = b->ie;
        break;

    case SPRD_GPIO_RIS:
        val = b->ris;
        break;

    case SPRD_GPIO_MIS:
        val = b->ris & b->ie;
        break;

    case SPRD_GPIO_INEN:
        val = b->inen;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_gpio: read at unhandled bank %d offset 0x%" HWADDR_PRIx "\n",
                      bank_idx, reg_off);
        break;
    }

    return val;
}

static void sprd_gpio_write(void *opaque, hwaddr offset, uint64_t val, unsigned size)
{
    SprdGpioState *s = opaque;
    int bank_idx = offset / SPRD_GPIO_BANK_SIZE;
    hwaddr reg_off = offset % SPRD_GPIO_BANK_SIZE;

    if (bank_idx >= SPRD_GPIO_NUM_BANKS) {
        return;
    }

    SprdGpioBank *b = &s->banks[bank_idx];

    switch (reg_off) {
    case SPRD_GPIO_DATA:
        b->data_out = (uint32_t)val;
        break;

    case SPRD_GPIO_DMSK:
        b->dmsk = (uint32_t)val;
        break;

    case SPRD_GPIO_DIR:
        b->dir = (uint32_t)val;
        break;

    case SPRD_GPIO_IS:
        b->is = (uint32_t)val;
        break;

    case SPRD_GPIO_IBE:
        b->ibe = (uint32_t)val;
        break;

    case SPRD_GPIO_IEV:
        b->iev = (uint32_t)val;
        break;

    case SPRD_GPIO_IE:
        b->ie = (uint32_t)val;
        sprd_gpio_update_irq(s, bank_idx);
        break;

    case SPRD_GPIO_IC:
        /* Clear corresponding interrupt bits */
        b->ris &= ~(uint32_t)val;
        sprd_gpio_update_irq(s, bank_idx);
        break;

    case SPRD_GPIO_INEN:
        b->inen = (uint32_t)val;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_gpio: write at unhandled bank %d offset 0x%" HWADDR_PRIx
                      " val 0x%" PRIx64 "\n", bank_idx, reg_off, val);
        break;
    }
}

static const MemoryRegionOps sprd_gpio_ops = {
    .read = sprd_gpio_read,
    .write = sprd_gpio_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = {
        .min_access_size = 4,
        .max_access_size = 4,
    },
};

static void sprd_gpio_reset(DeviceState *dev)
{
    SprdGpioState *s = SPRD_GPIO(dev);

    memset(s->banks, 0, sizeof(s->banks));
    qemu_set_irq(s->irq, 0);
}

static void sprd_gpio_realize(DeviceState *dev, Error **errp)
{
    SprdGpioState *s = SPRD_GPIO(dev);

    memory_region_init_io(&s->iomem, OBJECT(dev), &sprd_gpio_ops, s,
                          "sprd-gpio", SPRD_GPIO_TOTAL_SIZE);
    sysbus_init_mmio(SYS_BUS_DEVICE(dev), &s->iomem);
    sysbus_init_irq(SYS_BUS_DEVICE(dev), &s->irq);
}

static void sprd_gpio_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->realize = sprd_gpio_realize;
    dc->reset = sprd_gpio_reset;
}

static const TypeInfo sprd_gpio_info = {
    .name = TYPE_SPRD_GPIO,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(SprdGpioState),
    .class_init = sprd_gpio_class_init,
};

static void sprd_gpio_register_types(void)
{
    type_register_static(&sprd_gpio_info);
}

type_init(sprd_gpio_register_types)

DeviceState *sprd_gpio_create(hwaddr addr, qemu_irq irq)
{
    DeviceState *dev;
    SysBusDevice *sbd;

    dev = qdev_new(TYPE_SPRD_GPIO);
    sbd = SYS_BUS_DEVICE(dev);
    sysbus_realize_and_unref(sbd, &error_fatal);
    sysbus_mmio_map(sbd, 0, addr);
    sysbus_connect_irq(sbd, 0, irq);

    return dev;
}
