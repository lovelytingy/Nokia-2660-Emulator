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

    object_initialize_child(obj, "timer", &s->timer, TYPE_SPRD_TIMER);
    object_initialize_child(obj, "gpio", &s->gpio, TYPE_SPRD_GPIO);
    object_initialize_child(obj, "intc", &s->intc, TYPE_SPRD_INTC);
    object_initialize_child(obj, "kpd", &s->kpd, TYPE_SPRD_KPD);
    object_initialize_child(obj, "spi", &s->spi, TYPE_SPRD_SPI);
    object_initialize_child(obj, "lcd", &s->lcd, TYPE_GC9307);
    object_initialize_child(obj, "spi1", &s->spi1, TYPE_SPRD_SPI);
    object_initialize_child(obj, "sub_lcd", &s->sub_lcd, TYPE_ST7735);
    object_initialize_child(obj, "sfc", &s->sfc, TYPE_SPRD_SFC);
    object_initialize_child(obj, "audio", &s->audio, TYPE_SPRD_AUDIO);
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

    /* BootROM High Vector Region at 0xFFFF0000 (64 KB) */
    memory_region_init_rom(&s->bootrom, OBJECT(dev), "t107.bootrom", T107_BOOTROM_SIZE, &error_fatal);
    memory_region_add_subregion(sysmem, T107_BOOTROM_BASE, &s->bootrom);

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

    /* 5. System Timer (mapped at 0x50030000) */
    SysBusDevice *timer_sbd = SYS_BUS_DEVICE(&s->timer);
    if (!sysbus_realize(timer_sbd, errp)) {
        return;
    }
    memory_region_add_subregion_overlap(sysmem, T107_TIMERS_BASE,
                                        sysbus_mmio_get_region(timer_sbd, 0), 1);

    /* 6. GPIO Controller (mapped at 0x50040000) */
    SysBusDevice *gpio_sbd = SYS_BUS_DEVICE(&s->gpio);
    if (!sysbus_realize(gpio_sbd, errp)) {
        return;
    }
    memory_region_add_subregion_overlap(sysmem, T107_GPIO_BASE,
                                        sysbus_mmio_get_region(gpio_sbd, 0), 1);

    /* 6b. Interrupt Controller (mapped at 0x50050000) */
    SysBusDevice *intc_sbd = SYS_BUS_DEVICE(&s->intc);
    if (!sysbus_realize(intc_sbd, errp)) {
        return;
    }
    memory_region_add_subregion_overlap(sysmem, T107_INTC_BASE,
                                        sysbus_mmio_get_region(intc_sbd, 0), 1);
    sysbus_connect_irq(intc_sbd, 0, qdev_get_gpio_in(DEVICE(&s->cpu), ARM_CPU_IRQ));
    sysbus_connect_irq(intc_sbd, 1, qdev_get_gpio_in(DEVICE(&s->cpu), ARM_CPU_FIQ));

    /* 7. Keypad Matrix Controller (mapped at 0x50060000) */
    SysBusDevice *kpd_sbd = SYS_BUS_DEVICE(&s->kpd);
    if (!sysbus_realize(kpd_sbd, errp)) {
        return;
    }
    memory_region_add_subregion_overlap(sysmem, T107_KPD_BASE,
                                        sysbus_mmio_get_region(kpd_sbd, 0), 1);

    /* 8. GC9307 Display Panel & SPRD SPI0 Controller (mapped at 0x50080000) */
    if (!qdev_realize(DEVICE(&s->lcd), NULL, errp)) {
        return;
    }
    s->spi.lcd = &s->lcd;
    SysBusDevice *spi_sbd = SYS_BUS_DEVICE(&s->spi);
    if (!sysbus_realize(spi_sbd, errp)) {
        return;
    }
    memory_region_add_subregion_overlap(sysmem, T107_SPI0_BASE,
                                        sysbus_mmio_get_region(spi_sbd, 0), 1);

    /* 9. ST7735 Secondary Sub-Display Panel & SPRD SPI1 Controller (mapped at 0x50090000) */
    if (!qdev_realize(DEVICE(&s->sub_lcd), NULL, errp)) {
        return;
    }
    s->spi1.sub_lcd = &s->sub_lcd;
    SysBusDevice *spi1_sbd = SYS_BUS_DEVICE(&s->spi1);
    if (!sysbus_realize(spi1_sbd, errp)) {
        return;
    }
    memory_region_add_subregion_overlap(sysmem, T107_SPI1_BASE,
                                        sysbus_mmio_get_region(spi1_sbd, 0), 1);

    /* 10. Serial Flash Controller & SPI NAND Direct Window (0x500A0000 & 0x60000000) */
    SysBusDevice *sfc_sbd = SYS_BUS_DEVICE(&s->sfc);
    if (!sysbus_realize(sfc_sbd, errp)) {
        return;
    }
    memory_region_add_subregion_overlap(sysmem, T107_SFC_BASE,
                                        sysbus_mmio_get_region(sfc_sbd, 0), 1);
    memory_region_add_subregion_overlap(sysmem, T107_FLASH_BASE,
                                        sysbus_mmio_get_region(sfc_sbd, 1), 1);

    /* 11. SPRD Audio Subsystem & Tone Generator (0x500C0000) */
    SysBusDevice *audio_sbd = SYS_BUS_DEVICE(&s->audio);
    if (!sysbus_realize(audio_sbd, errp)) {
        return;
    }
    memory_region_add_subregion_overlap(sysmem, T107_AUDIO_BASE,
                                        sysbus_mmio_get_region(audio_sbd, 0), 1);
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
