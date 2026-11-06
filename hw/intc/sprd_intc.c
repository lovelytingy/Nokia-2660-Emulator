/*
 * Unisoc SPRD Interrupt Controller (INTC) Model
 * Base Address: 0x50050000
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "hw/irq.h"
#include "hw/intc/sprd_intc.h"
#include "qemu/log.h"
#include "qemu/module.h"

static void sprd_intc_update(SprdIntcState *s)
{
    uint32_t irq_active = (s->irq_raw_sts | s->irq_soft) & ~s->int_type;
    uint32_t irq_pending = irq_active & s->irq_en;
    qemu_set_irq(s->parent_irq, !!irq_pending);

    uint32_t fiq_active = (s->irq_raw_sts | s->fiq_soft) & s->int_type;
    uint32_t fiq_pending = fiq_active & s->fiq_en;
    qemu_set_irq(s->parent_fiq, !!fiq_pending);
}

static void sprd_intc_set_irq(void *opaque, int irq, int level)
{
    SprdIntcState *s = SPRD_INTC(opaque);

    if (irq < 0 || irq >= SPRD_INTC_NUM_IRQS) {
        return;
    }

    if (level) {
        s->irq_raw_sts |= (1U << irq);
    } else {
        s->irq_raw_sts &= ~(1U << irq);
    }

    sprd_intc_update(s);
}

static uint64_t sprd_intc_read(void *opaque, hwaddr offset, unsigned size)
{
    SprdIntcState *s = SPRD_INTC(opaque);
    uint32_t val = 0;

    switch (offset) {
    case SPRD_INTC_IRQ_RAW_STS:
        val = (s->irq_raw_sts | s->irq_soft) & ~s->int_type;
        break;
    case SPRD_INTC_IRQ_MASK_STS:
        val = ((s->irq_raw_sts | s->irq_soft) & ~s->int_type) & s->irq_en;
        break;
    case SPRD_INTC_IRQ_EN:
        val = s->irq_en;
        break;
    case SPRD_INTC_IRQ_SOFT:
        val = s->irq_soft;
        break;
    case SPRD_INTC_FIQ_RAW_STS:
        val = (s->irq_raw_sts | s->fiq_soft) & s->int_type;
        break;
    case SPRD_INTC_FIQ_MASK_STS:
        val = ((s->irq_raw_sts | s->fiq_soft) & s->int_type) & s->fiq_en;
        break;
    case SPRD_INTC_FIQ_EN:
        val = s->fiq_en;
        break;
    case SPRD_INTC_FIQ_SOFT:
        val = s->fiq_soft;
        break;
    case SPRD_INTC_TYPE:
        val = s->int_type;
        break;
    default:
        qemu_log_mask(LOG_GUEST_ERROR, "[SPRD INTC] Unhandled read at offset 0x%04" HWADDR_PRIx "\n", offset);
        break;
    }

    return val;
}

static void sprd_intc_write(void *opaque, hwaddr offset, uint64_t val64, unsigned size)
{
    SprdIntcState *s = SPRD_INTC(opaque);
    uint32_t val = (uint32_t)val64;

    switch (offset) {
    case SPRD_INTC_IRQ_EN:
        s->irq_en |= val;
        sprd_intc_update(s);
        break;
    case SPRD_INTC_IRQ_EN_CLR:
        s->irq_en &= ~val;
        sprd_intc_update(s);
        break;
    case SPRD_INTC_IRQ_SOFT:
        s->irq_soft |= val;
        sprd_intc_update(s);
        break;
    case SPRD_INTC_IRQ_SOFT_CLR:
        s->irq_soft &= ~val;
        sprd_intc_update(s);
        break;
    case SPRD_INTC_IRQ_CLR:
        s->irq_raw_sts &= ~val;
        sprd_intc_update(s);
        break;
    case SPRD_INTC_FIQ_EN:
        s->fiq_en |= val;
        sprd_intc_update(s);
        break;
    case SPRD_INTC_FIQ_EN_CLR:
        s->fiq_en &= ~val;
        sprd_intc_update(s);
        break;
    case SPRD_INTC_FIQ_SOFT:
        s->fiq_soft |= val;
        sprd_intc_update(s);
        break;
    case SPRD_INTC_FIQ_SOFT_CLR:
        s->fiq_soft &= ~val;
        sprd_intc_update(s);
        break;
    case SPRD_INTC_TYPE:
        s->int_type = val;
        sprd_intc_update(s);
        break;
    default:
        qemu_log_mask(LOG_GUEST_ERROR, "[SPRD INTC] Unhandled write at offset 0x%04" HWADDR_PRIx " val 0x%08x\n", offset, val);
        break;
    }
}

static const MemoryRegionOps sprd_intc_ops = {
    .read = sprd_intc_read,
    .write = sprd_intc_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = {
        .min_access_size = 4,
        .max_access_size = 4,
    },
};

static void sprd_intc_reset(DeviceState *dev)
{
    SprdIntcState *s = SPRD_INTC(dev);

    s->irq_raw_sts = 0;
    s->irq_en = 0;
    s->irq_soft = 0;
    s->fiq_raw_sts = 0;
    s->fiq_en = 0;
    s->fiq_soft = 0;
    s->int_type = 0;
    sprd_intc_update(s);
}

static void sprd_intc_init(Object *obj)
{
    SprdIntcState *s = SPRD_INTC(obj);
    SysBusDevice *sbd = SYS_BUS_DEVICE(obj);

    memory_region_init_io(&s->iomem, obj, &sprd_intc_ops, s, "sprd-intc", 0x1000);
    sysbus_init_mmio(sbd, &s->iomem);

    sysbus_init_irq(sbd, &s->parent_irq);
    sysbus_init_irq(sbd, &s->parent_fiq);

    qdev_init_gpio_in(DEVICE(s), sprd_intc_set_irq, SPRD_INTC_NUM_IRQS);
}

static void sprd_intc_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->reset = sprd_intc_reset;
    dc->desc = "Unisoc SPRD Interrupt Controller (INTC)";
}

static const TypeInfo sprd_intc_info = {
    .name          = TYPE_SPRD_INTC,
    .parent        = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(SprdIntcState),
    .instance_init = sprd_intc_init,
    .class_init    = sprd_intc_class_init,
};

static void sprd_intc_register_types(void)
{
    type_register_static(&sprd_intc_info);
}

type_init(sprd_intc_register_types)
