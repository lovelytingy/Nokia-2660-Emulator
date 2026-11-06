/*
 * Unisoc T107 (UMS9117) SoC Model
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "qapi/error.h"
#include "hw/sysbus.h"
#include "hw/arm/boot.h"
#include "hw/arm/t107.h"
#include "sysemu/sysemu.h"
#include "qemu/log.h"
#include "qemu/module.h"

static const hwaddr t107_uart_addrs[T107_NUM_UARTS] = {
    T107_UART0_BASE,
    T107_UART1_BASE,
    T107_UART2_BASE,
};

static uint64_t t107_mmio_trap_read(void *opaque, hwaddr offset, unsigned size)
{
    hwaddr addr = T107_PERIPH_BASE + offset;
    qemu_log_mask(LOG_UNIMP,
                  "[N2660Emu MMIO TRAP] Unknown peripheral READ at 0x%08" HWADDR_PRIx
                  " (size %u)\n",
                  addr, size);
    return 0;
}

static void t107_mmio_trap_write(void *opaque, hwaddr offset, uint64_t val, unsigned size)
{
    hwaddr addr = T107_PERIPH_BASE + offset;
    qemu_log_mask(LOG_UNIMP,
                  "[N2660Emu MMIO TRAP] Unknown peripheral WRITE at 0x%08" HWADDR_PRIx
                  " val 0x%" PRIx64 " (size %u)\n",
                  addr, val, size);
}

static const MemoryRegionOps t107_mmio_trap_ops = {
    .read = t107_mmio_trap_read,
    .write = t107_mmio_trap_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = {
        .min_access_size = 1,
        .max_access_size = 4,
    },
};

static void t107_soc_init(Object *obj)
{
    T107SocState *s = T107_SOC(obj);

    object_initialize_child(obj, "cpu", &s->cpu, ARM_CPU_TYPE_NAME("cortex-a7"));

    for (int i = 0; i < T107_NUM_UARTS; i++) {
        object_initialize_child(obj, "uart[*]", &s->uart[i], TYPE_SPRD_UART);
    }
}

static void t107_soc_realize(DeviceState *dev, Error **errp)
{
    T107SocState *s = T107_SOC(dev);
    MemoryRegion *sysmem = get_system_memory();

    /* Realize Cortex-A7 Application CPU */
    if (!qdev_realize(DEVICE(&s->cpu), NULL, errp)) {
        return;
    }

    /* 1. Internal SRAM (AP on-chip RAM at 0x00000000, 64 KB) */
    memory_region_init_ram(&s->sram, OBJECT(dev), "t107.sram", T107_SRAM_SIZE, &error_fatal);
    memory_region_add_subregion(sysmem, T107_SRAM_BASE, &s->sram);

    /* 2. DRAM Region (mapped at 0x80000000) */
    if (s->dram_mr) {
        memory_region_add_subregion(sysmem, T107_DRAM_BASE, s->dram_mr);
    }

    /* 3. Catch-all MMIO Trap for unmapped peripherals in 0x40000000 - 0x5FFFFFFF */
    memory_region_init_io(&s->mmio_trap, OBJECT(dev), &t107_mmio_trap_ops, s,
                          "t107.unmapped_mmio", T107_PERIPH_SIZE);
    /* Lowest priority subregion (-1) so mapped peripherals override it */
    memory_region_add_subregion_overlap(sysmem, T107_PERIPH_BASE, &s->mmio_trap, -1);

    /* 4. Peripheral UARTs */
    for (int i = 0; i < T107_NUM_UARTS; i++) {
        SysBusDevice *sbd = SYS_BUS_DEVICE(&s->uart[i]);
        if (i < serial_max_chrs()) {
            qdev_prop_set_chr(DEVICE(sbd), "chardev", serial_hd(i));
        }
        if (!sysbus_realize(sbd, errp)) {
            return;
        }
        /* Mapped at priority 1 to override catch-all MMIO */
        memory_region_add_subregion_overlap(sysmem, t107_uart_addrs[i],
                                            sysbus_mmio_get_region(sbd, 0), 1);
    }
}

static Property t107_soc_properties[] = {
    DEFINE_PROP_LINK("dram-mr", T107SocState, dram_mr, TYPE_MEMORY_REGION, MemoryRegion *),
    DEFINE_PROP_END_OF_LIST(),
};

static void t107_soc_class_init(ObjectClass *oc, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(oc);

    dc->realize = t107_soc_realize;
    device_class_set_props(dc, t107_soc_properties);
}

static const TypeInfo t107_soc_info = {
    .name = TYPE_T107_SOC,
    .parent = TYPE_DEVICE,
    .instance_size = sizeof(T107SocState),
    .instance_init = t107_soc_init,
    .class_init = t107_soc_class_init,
};

static void t107_soc_register_types(void)
{
    type_register_static(&t107_soc_info);
}

type_init(t107_soc_register_types)
