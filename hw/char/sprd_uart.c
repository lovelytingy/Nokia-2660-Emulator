/*
 * Unisoc / Spreadtrum UART Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "chardev/char-fe.h"
#include "qemu/log.h"
#include "qemu/module.h"
#include "migration/vmstate.h"
#include "hw/char/sprd_uart.h"

static void sprd_uart_update_irq(SprdUartState *s)
{
    bool level = false;
    if ((s->ien & 0x1) && (s->rx_count > 0)) {
        level = true;
    }
    qemu_set_irq(s->irq, level);
}

static uint64_t sprd_uart_read(void *opaque, hwaddr offset, unsigned size)
{
    SprdUartState *s = opaque;
    uint32_t val = 0;

    switch (offset) {
    case SPRD_RXD:
        if (s->rx_count > 0) {
            val = s->rx_fifo[s->rx_tail];
            s->rx_tail = (s->rx_tail + 1) % sizeof(s->rx_fifo);
            s->rx_count--;
            sprd_uart_update_irq(s);
        }
        break;

    case SPRD_STS0:
        val = s->sts0;
        break;

    case SPRD_STS1:
        /* Bit 8 indicates TX FIFO is empty (ready to transmit) */
        val = SPRD_STS1_TX_FIFO_EMPTY;
        if (s->rx_count == 0) {
            val |= SPRD_STS1_RX_FIFO_EMPTY;
        } else if (s->rx_count >= sizeof(s->rx_fifo)) {
            val |= SPRD_STS1_RX_FIFO_FULL;
        }
        /* Lower bits typically hold current FIFO byte counts */
        val |= (s->rx_count & 0x7F);
        break;

    case SPRD_IEN:
        val = s->ien;
        break;

    case SPRD_CTL0:
        val = s->ctl0;
        break;

    case SPRD_CTL1:
        val = s->ctl1;
        break;

    case SPRD_CTL2:
        val = s->ctl2;
        break;

    case SPRD_CLKD0:
        val = s->clkd0;
        break;

    case SPRD_CLKD1:
        val = s->clkd1;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_uart: read at unhandled offset 0x%" HWADDR_PRIx "\n",
                      offset);
        break;
    }

    return val;
}

static void sprd_uart_write(void *opaque, hwaddr offset, uint64_t val, unsigned size)
{
    SprdUartState *s = opaque;
    uint8_t ch;

    switch (offset) {
    case SPRD_TXD:
        ch = (uint8_t)val;
        if (qemu_chr_fe_backend_connected(&s->chr)) {
            /* Synchronously send byte to connected backend (e.g. stdout / socket) */
            qemu_chr_fe_write_all(&s->chr, &ch, 1);
        }
        break;

    case SPRD_IEN:
        s->ien = val;
        sprd_uart_update_irq(s);
        break;

    case SPRD_ICLR:
        /* Interrupt clear */
        sprd_uart_update_irq(s);
        break;

    case SPRD_CTL0:
        s->ctl0 = val;
        break;

    case SPRD_CTL1:
        s->ctl1 = val;
        break;

    case SPRD_CTL2:
        s->ctl2 = val;
        break;

    case SPRD_CLKD0:
        s->clkd0 = val;
        break;

    case SPRD_CLKD1:
        s->clkd1 = val;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_uart: write at unhandled offset 0x%" HWADDR_PRIx
                      " val 0x%" PRIx64 "\n", offset, val);
        break;
    }
}

static const MemoryRegionOps sprd_uart_ops = {
    .read = sprd_uart_read,
    .write = sprd_uart_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = {
        .min_access_size = 1,
        .max_access_size = 4,
    },
};

static int sprd_uart_can_receive(void *opaque)
{
    SprdUartState *s = opaque;
    return sizeof(s->rx_fifo) - s->rx_count;
}

static void sprd_uart_receive(void *opaque, const uint8_t *buf, int size)
{
    SprdUartState *s = opaque;

    for (int i = 0; i < size; i++) {
        if (s->rx_count < sizeof(s->rx_fifo)) {
            s->rx_fifo[s->rx_head] = buf[i];
            s->rx_head = (s->rx_head + 1) % sizeof(s->rx_fifo);
            s->rx_count++;
        }
    }
    sprd_uart_update_irq(s);
}

static void sprd_uart_reset(DeviceState *dev)
{
    SprdUartState *s = SPRD_UART(dev);

    s->sts0 = 0;
    s->sts1 = SPRD_STS1_TX_FIFO_EMPTY | SPRD_STS1_RX_FIFO_EMPTY;
    s->ien = 0;
    s->ctl0 = 0;
    s->ctl1 = 0;
    s->ctl2 = 0;
    s->clkd0 = 0;
    s->clkd1 = 0;
    s->rx_count = 0;
    s->rx_head = 0;
    s->rx_tail = 0;

    sprd_uart_update_irq(s);
}

static void sprd_uart_realize(DeviceState *dev, Error **errp)
{
    SprdUartState *s = SPRD_UART(dev);

    memory_region_init_io(&s->iomem, OBJECT(dev), &sprd_uart_ops, s,
                          "sprd-uart", SPRD_UART_REGS_SIZE);
    sysbus_init_mmio(SYS_BUS_DEVICE(dev), &s->iomem);
    sysbus_init_irq(SYS_BUS_DEVICE(dev), &s->irq);

    qemu_chr_fe_set_handlers(&s->chr, sprd_uart_can_receive,
                             sprd_uart_receive, NULL, NULL,
                             s, NULL, true);
}

static const VMStateDescription vmstate_sprd_uart = {
    .name = "sprd-uart",
    .version_id = 1,
    .minimum_version_id = 1,
    .fields = (VMStateField[]) {
        VMSTATE_UINT32(sts0, SprdUartState),
        VMSTATE_UINT32(sts1, SprdUartState),
        VMSTATE_UINT32(ien, SprdUartState),
        VMSTATE_UINT32(ctl0, SprdUartState),
        VMSTATE_UINT32(ctl1, SprdUartState),
        VMSTATE_UINT32(ctl2, SprdUartState),
        VMSTATE_UINT32(clkd0, SprdUartState),
        VMSTATE_UINT32(clkd1, SprdUartState),
        VMSTATE_UINT32(rx_count, SprdUartState),
        VMSTATE_UINT32(rx_head, SprdUartState),
        VMSTATE_UINT32(rx_tail, SprdUartState),
        VMSTATE_BUFFER(rx_fifo, SprdUartState),
        VMSTATE_END_OF_LIST()
    }
};

static Property sprd_uart_properties[] = {
    DEFINE_PROP_CHR("chardev", SprdUartState, chr),
    DEFINE_PROP_END_OF_LIST(),
};

static void sprd_uart_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->realize = sprd_uart_realize;
    dc->reset = sprd_uart_reset;
    dc->vmsd = &vmstate_sprd_uart;
    device_class_set_props(dc, sprd_uart_properties);
}

static const TypeInfo sprd_uart_info = {
    .name = TYPE_SPRD_UART,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(SprdUartState),
    .class_init = sprd_uart_class_init,
};

static void sprd_uart_register_types(void)
{
    type_register_static(&sprd_uart_info);
}

type_init(sprd_uart_register_types)

DeviceState *sprd_uart_create(hwaddr addr, qemu_irq irq, Chardev *chr)
{
    DeviceState *dev;
    SysBusDevice *sbd;

    dev = qdev_new(TYPE_SPRD_UART);
    sbd = SYS_BUS_DEVICE(dev);
    qdev_prop_set_chr(dev, "chardev", chr);
    sysbus_realize_and_unref(sbd, &error_fatal);
    sysbus_mmio_map(sbd, 0, addr);
    sysbus_connect_irq(sbd, 0, irq);

    return dev;
}
