/*
 * Unisoc T107 (UMS9117) SoC and Nokia 2660 Machine Definition
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_ARM_T107_H
#define HW_ARM_T107_H

#include "qemu/osdep.h"
#include "target/arm/cpu-qom.h"
#include "hw/sysbus.h"
#include "hw/char/sprd_uart.h"
#include "hw/timer/sprd_timer.h"
#include "hw/gpio/sprd_gpio.h"
#include "hw/ssi/sprd_spi.h"
#include "hw/display/gc9307.h"
#include "hw/display/st7735.h"
#include "hw/input/sprd_kpd.h"
#include "hw/boards.h"

#define TYPE_T107_SOC "t107-soc"
OBJECT_DECLARE_SIMPLE_TYPE(T107SocState, T107_SOC)

#define TYPE_NOKIA_2660_MACHINE MACHINE_TYPE_NAME("nokia-2660")
OBJECT_DECLARE_SIMPLE_TYPE(Nokia2660MachineState, NOKIA_2660_MACHINE)

/* Memory Map Addresses */
#define T107_SRAM_BASE            0x00000000
#define T107_SRAM_SIZE            (64 * 1024)       /* 64 KB Internal SRAM */

#define T107_DRAM_BASE            0x80000000
#define T107_DRAM_DEFAULT_SIZE    (48 * 1024 * 1024) /* 48 MB LPDDR2 */

#define T107_PERIPH_BASE          0x40000000
#define T107_PERIPH_SIZE          0x20000000        /* 512 MB peripheral window */

#define T107_UART0_BASE           0x50000000        /* Primary debug/FDL UART */
#define T107_UART1_BASE           0x50010000
#define T107_UART2_BASE           0x50020000
#define T107_TIMERS_BASE          0x50030000
#define T107_GPIO_BASE            0x50040000
#define T107_KPD_BASE             0x50060000        /* Keypad Matrix Controller */
#define T107_SPI0_BASE            0x50080000        /* GC9307 Display interface */
#define T107_SPI1_BASE            0x50090000        /* ST7735 Sub-Display interface */

#define T107_NUM_UARTS            3

struct T107SocState {
    /*< private >*/
    DeviceState parent_obj;

    /*< public >*/
    ARMCPU cpu;
    MemoryRegion sram;
    MemoryRegion dram_alias;
    MemoryRegion *dram_mr;
    MemoryRegion mmio_trap;

    SprdUartState uart[T107_NUM_UARTS];
    SprdTimerState timer;
    SprdGpioState gpio;
    SprdKpdState kpd;
    SprdSPIState spi;
    GC9307State lcd;
    SprdSPIState spi1;
    ST7735State sub_lcd;
};

struct Nokia2660MachineState {
    /*< private >*/
    MachineState parent_obj;

    /*< public >*/
    T107SocState soc;
};

#endif /* HW_ARM_T107_H */
