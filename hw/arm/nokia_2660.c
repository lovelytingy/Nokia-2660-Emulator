/*
 * Nokia 2660 Flip (TA-1480) Machine Model
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "qapi/error.h"
#include "hw/boards.h"
#include "hw/arm/boot.h"
#include "hw/arm/t107.h"

static struct arm_boot_info nokia_2660_boot_info;

static void nokia_2660_init(MachineState *machine)
{
    Nokia2660MachineState *ms = NOKIA_2660_MACHINE(machine);

    /* Allocate and initialize T107 SoC */
    object_initialize_child(OBJECT(machine), "soc", &ms->soc, TYPE_T107_SOC);

    /* Connect system RAM to the SoC's DRAM region */
    object_property_set_link(OBJECT(&ms->soc), "dram-mr",
                             OBJECT(machine->ram), &error_abort);

    /* Realize SoC */
    qdev_realize(DEVICE(&ms->soc), NULL, &error_fatal);

    /* Configure ARM bootloader support for -kernel / -bios */
    nokia_2660_boot_info.ram_size = machine->ram_size;
    nokia_2660_boot_info.board_id = 0x9117;
    nokia_2660_boot_info.loader_start = T107_DRAM_BASE;

    if (machine->kernel_filename) {
        arm_load_kernel(ARM_CPU(&ms->soc.cpu), machine, &nokia_2660_boot_info);
    }
}

static void nokia_2660_class_init(ObjectClass *oc, void *data)
{
    MachineClass *mc = MACHINE_CLASS(oc);

    mc->desc = "Nokia 2660 Flip (TA-1480, Unisoc T107 / UMS9117)";
    mc->init = nokia_2660_init;
    mc->default_cpu_type = ARM_CPU_TYPE_NAME("cortex-a7");
    mc->default_ram_size = T107_DRAM_DEFAULT_SIZE;
    mc->default_ram_id = "nokia2660.ram";
    mc->min_cpus = 1;
    mc->max_cpus = 1;
}

static const TypeInfo nokia_2660_info = {
    .name = TYPE_NOKIA_2660_MACHINE,
    .parent = TYPE_MACHINE,
    .instance_size = sizeof(Nokia2660MachineState),
    .class_init = nokia_2660_class_init,
};

static void nokia_2660_register_types(void)
{
    type_register_static(&nokia_2660_info);
}

type_init(nokia_2660_register_types)
