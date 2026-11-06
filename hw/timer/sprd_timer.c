/*
 * Unisoc / Spreadtrum System Timer Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "hw/ptimer.h"
#include "qemu/log.h"
#include "qemu/module.h"
#include "migration/vmstate.h"
#include "hw/timer/sprd_timer.h"

static void sprd_timer_update_irq(SprdTimerState *s)
{
    bool level = false;
    if ((s->int_reg & SPRD_TIMER_INT_EN) && (s->int_reg & SPRD_TIMER_INT_RAW_STS)) {
        s->int_reg |= SPRD_TIMER_INT_MASK_STS;
        level = true;
    } else {
        s->int_reg &= ~SPRD_TIMER_INT_MASK_STS;
    }
    qemu_set_irq(s->irq, level);
}

static void sprd_timer_tick(void *opaque)
{
    SprdTimerState *s = opaque;
    s->int_reg |= SPRD_TIMER_INT_RAW_STS;
    sprd_timer_update_irq(s);
}

static uint64_t sprd_timer_read(void *opaque, hwaddr offset, unsigned size)
{
    SprdTimerState *s = opaque;
    uint32_t val = 0;

    switch (offset) {
    case SPRD_TIMER_LOAD_LO:
        val = s->load_lo;
        break;

    case SPRD_TIMER_LOAD_HI:
        val = s->load_hi;
        break;

    case SPRD_TIMER_VALUE_LO:
    case SPRD_TIMER_SHDW_LO:
        if (s->ctl & SPRD_TIMER_CTL_ENABLE) {
            val = (uint32_t)ptimer_get_count(s->ptimer);
        } else {
            val = s->load_lo;
        }
        break;

    case SPRD_TIMER_VALUE_HI:
    case SPRD_TIMER_SHDW_HI:
        val = s->value_hi;
        break;

    case SPRD_TIMER_CTL:
        val = s->ctl;
        break;

    case SPRD_TIMER_INT:
        val = s->int_reg;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_timer: read at unhandled offset 0x%" HWADDR_PRIx "\n",
                      offset);
        break;
    }

    return val;
}

static void sprd_timer_write(void *opaque, hwaddr offset, uint64_t val, unsigned size)
{
    SprdTimerState *s = opaque;

    switch (offset) {
    case SPRD_TIMER_LOAD_LO:
        s->load_lo = (uint32_t)val;
        ptimer_transaction_begin(s->ptimer);
        ptimer_set_limit(s->ptimer, s->load_lo, 1);
        ptimer_transaction_commit(s->ptimer);
        break;

    case SPRD_TIMER_LOAD_HI:
        s->load_hi = (uint32_t)val;
        break;

    case SPRD_TIMER_CTL:
        s->ctl = (uint32_t)val;
        ptimer_transaction_begin(s->ptimer);
        if (s->ctl & SPRD_TIMER_CTL_ENABLE) {
            ptimer_run(s->ptimer, !(s->ctl & SPRD_TIMER_CTL_PERIOD));
        } else {
            ptimer_stop(s->ptimer);
        }
        ptimer_transaction_commit(s->ptimer);
        break;

    case SPRD_TIMER_INT:
        if (val & SPRD_TIMER_INT_CLR) {
            s->int_reg &= ~SPRD_TIMER_INT_RAW_STS;
        }
        s->int_reg = (s->int_reg & ~SPRD_TIMER_INT_EN) | (val & SPRD_TIMER_INT_EN);
        sprd_timer_update_irq(s);
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_timer: write at unhandled offset 0x%" HWADDR_PRIx
                      " val 0x%" PRIx64 "\n", offset, val);
        break;
    }
}

static const MemoryRegionOps sprd_timer_ops = {
    .read = sprd_timer_read,
    .write = sprd_timer_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = {
        .min_access_size = 4,
        .max_access_size = 4,
    },
};

static void sprd_timer_reset(DeviceState *dev)
{
    SprdTimerState *s = SPRD_TIMER(dev);

    s->load_lo = 0;
    s->load_hi = 0;
    s->value_lo = 0;
    s->value_hi = 0;
    s->ctl = 0;
    s->int_reg = 0;

    ptimer_transaction_begin(s->ptimer);
    ptimer_stop(s->ptimer);
    ptimer_set_limit(s->ptimer, 0, 1);
    ptimer_transaction_commit(s->ptimer);

    sprd_timer_update_irq(s);
}

static void sprd_timer_realize(DeviceState *dev, Error **errp)
{
    SprdTimerState *s = SPRD_TIMER(dev);

    s->ptimer = ptimer_init(sprd_timer_tick, s, PTIMER_POLICY_DEFAULT);
    /* Standard 32.768 kHz RTC clock frequency or 26 MHz system clock */
    ptimer_transaction_begin(s->ptimer);
    ptimer_set_freq(s->ptimer, 32768);
    ptimer_transaction_commit(s->ptimer);

    memory_region_init_io(&s->iomem, OBJECT(dev), &sprd_timer_ops, s,
                          "sprd-timer", SPRD_TIMER_REGS_SIZE);
    sysbus_init_mmio(SYS_BUS_DEVICE(dev), &s->iomem);
    sysbus_init_irq(SYS_BUS_DEVICE(dev), &s->irq);
}

static const VMStateDescription vmstate_sprd_timer = {
    .name = "sprd-timer",
    .version_id = 1,
    .minimum_version_id = 1,
    .fields = (VMStateField[]) {
        VMSTATE_UINT32(load_lo, SprdTimerState),
        VMSTATE_UINT32(load_hi, SprdTimerState),
        VMSTATE_UINT32(value_lo, SprdTimerState),
        VMSTATE_UINT32(value_hi, SprdTimerState),
        VMSTATE_UINT32(ctl, SprdTimerState),
        VMSTATE_UINT32(int_reg, SprdTimerState),
        VMSTATE_PTIMER(ptimer, SprdTimerState),
        VMSTATE_END_OF_LIST()
    }
};

static void sprd_timer_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->realize = sprd_timer_realize;
    dc->reset = sprd_timer_reset;
    dc->vmsd = &vmstate_sprd_timer;
}

static const TypeInfo sprd_timer_info = {
    .name = TYPE_SPRD_TIMER,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(SprdTimerState),
    .class_init = sprd_timer_class_init,
};

static void sprd_timer_register_types(void)
{
    type_register_static(&sprd_timer_info);
}

type_init(sprd_timer_register_types)

DeviceState *sprd_timer_create(hwaddr addr, qemu_irq irq)
{
    DeviceState *dev;
    SysBusDevice *sbd;

    dev = qdev_new(TYPE_SPRD_TIMER);
    sbd = SYS_BUS_DEVICE(dev);
    sysbus_realize_and_unref(sbd, &error_fatal);
    sysbus_mmio_map(sbd, 0, addr);
    sysbus_connect_irq(sbd, 0, irq);

    return dev;
}
