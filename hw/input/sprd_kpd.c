/*
 * Unisoc / Spreadtrum Keypad Controller (KPD) Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qemu/log.h"
#include "qemu/module.h"
#include "migration/vmstate.h"
#include "hw/input/sprd_kpd.h"

static void sprd_kpd_update_irq(SprdKpdState *s)
{
    bool level = false;
    if ((s->int_en & 0x1) && (s->int_raw & 0x1)) {
        level = true;
    }
    qemu_set_irq(s->irq, level);
}

void sprd_kpd_inject_key(SprdKpdState *s, NokiaKeyCode key, bool pressed)
{
    if (key >= NOKIA_KEY_COUNT) {
        return;
    }

    s->keys_pressed[key] = pressed;

    /* Update matrix status bits */
    uint32_t bit = (1 << (key % 16));
    if (key < 16) {
        if (pressed) {
            s->status0 |= bit;
        } else {
            s->status0 &= ~bit;
        }
    } else {
        if (pressed) {
            s->status1 |= bit;
        } else {
            s->status1 &= ~bit;
        }
    }

    if (pressed) {
        s->int_raw |= 0x1;
    }
    sprd_kpd_update_irq(s);
}

static uint64_t sprd_kpd_read(void *opaque, hwaddr offset, unsigned size)
{
    SprdKpdState *s = opaque;
    uint32_t val = 0;

    switch (offset) {
    case SPRD_KPD_CTRL:
        val = s->ctrl;
        break;

    case SPRD_KPD_INT_EN:
        val = s->int_en;
        break;

    case SPRD_KPD_INT_RAW_STS:
        val = s->int_raw;
        break;

    case SPRD_KPD_INT_MASK_STS:
        val = s->int_raw & s->int_en;
        break;

    case SPRD_KPD_POLARITY:
        val = s->polarity;
        break;

    case SPRD_KPD_CLK_DIV:
        val = s->clk_div;
        break;

    case SPRD_KPD_STATUS0:
        val = s->status0;
        break;

    case SPRD_KPD_STATUS1:
        val = s->status1;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_kpd: read at unhandled offset 0x%" HWADDR_PRIx "\n",
                      offset);
        break;
    }

    return val;
}

static void sprd_kpd_write(void *opaque, hwaddr offset, uint64_t val, unsigned size)
{
    SprdKpdState *s = opaque;

    switch (offset) {
    case SPRD_KPD_CTRL:
        s->ctrl = (uint32_t)val;
        break;

    case SPRD_KPD_INT_EN:
        s->int_en = (uint32_t)val;
        sprd_kpd_update_irq(s);
        break;

    case SPRD_KPD_INT_CLR:
        /* Writing 1 clears raw interrupt */
        if (val & 0x1) {
            s->int_raw &= ~0x1;
        }
        sprd_kpd_update_irq(s);
        break;

    case SPRD_KPD_POLARITY:
        s->polarity = (uint32_t)val;
        break;

    case SPRD_KPD_CLK_DIV:
        s->clk_div = (uint32_t)val;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_kpd: write at unhandled offset 0x%" HWADDR_PRIx
                      " val 0x%" PRIx64 "\n", offset, val);
        break;
    }
}

static const MemoryRegionOps sprd_kpd_ops = {
    .read = sprd_kpd_read,
    .write = sprd_kpd_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = {
        .min_access_size = 4,
        .max_access_size = 4,
    },
};

static void sprd_kpd_reset(DeviceState *dev)
{
    SprdKpdState *s = SPRD_KPD(dev);

    s->ctrl = 0;
    s->int_en = 0;
    s->int_raw = 0;
    s->polarity = 0;
    s->clk_div = 0;
    s->status0 = 0;
    s->status1 = 0;
    memset(s->keys_pressed, 0, sizeof(s->keys_pressed));

    sprd_kpd_update_irq(s);
}

static void sprd_kpd_realize(DeviceState *dev, Error **errp)
{
    SprdKpdState *s = SPRD_KPD(dev);

    memory_region_init_io(&s->iomem, OBJECT(dev), &sprd_kpd_ops, s,
                          "sprd-kpd", SPRD_KPD_REGS_SIZE);
    sysbus_init_mmio(SYS_BUS_DEVICE(dev), &s->iomem);
    sysbus_init_irq(SYS_BUS_DEVICE(dev), &s->irq);
}

static void sprd_kpd_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->realize = sprd_kpd_realize;
    dc->reset = sprd_kpd_reset;
}

static const TypeInfo sprd_kpd_info = {
    .name = TYPE_SPRD_KPD,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(SprdKpdState),
    .class_init = sprd_kpd_class_init,
};

static void sprd_kpd_register_types(void)
{
    type_register_static(&sprd_kpd_info);
}

type_init(sprd_kpd_register_types)

DeviceState *sprd_kpd_create(hwaddr addr, qemu_irq irq)
{
    DeviceState *dev;
    SysBusDevice *sbd;

    dev = qdev_new(TYPE_SPRD_KPD);
    sbd = SYS_BUS_DEVICE(dev);
    sysbus_realize_and_unref(sbd, &error_fatal);
    sysbus_mmio_map(sbd, 0, addr);
    sysbus_connect_irq(sbd, 0, irq);

    return dev;
}
