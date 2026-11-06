/*
 * Unisoc / Spreadtrum Keypad Controller (KPD) Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_INPUT_SPRD_KPD_H
#define HW_INPUT_SPRD_KPD_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qom/object.h"

#define TYPE_SPRD_KPD "sprd-kpd"
OBJECT_DECLARE_SIMPLE_TYPE(SprdKpdState, SPRD_KPD)

/* Register Offsets */
#define SPRD_KPD_CTRL          0x0000  /* Control (Enable, Debounce) */
#define SPRD_KPD_INT_EN        0x0004  /* Interrupt Enable */
#define SPRD_KPD_INT_RAW_STS   0x0008  /* Raw Interrupt Status */
#define SPRD_KPD_INT_MASK_STS  0x000C  /* Masked Interrupt Status */
#define SPRD_KPD_INT_CLR       0x0010  /* Interrupt Clear */
#define SPRD_KPD_POLARITY      0x0014  /* Key Polarity */
#define SPRD_KPD_CLK_DIV       0x0018  /* Clock Divider */
#define SPRD_KPD_STATUS0       0x001C  /* Key Status Low (Rows 0-3) */
#define SPRD_KPD_STATUS1       0x0020  /* Key Status High (Rows 4-7) */

#define SPRD_KPD_REGS_SIZE     0x1000

/* Nokia 2660 Key Codes */
typedef enum {
    NOKIA_KEY_0 = 0,
    NOKIA_KEY_1,
    NOKIA_KEY_2,
    NOKIA_KEY_3,
    NOKIA_KEY_4,
    NOKIA_KEY_5,
    NOKIA_KEY_6,
    NOKIA_KEY_7,
    NOKIA_KEY_8,
    NOKIA_KEY_9,
    NOKIA_KEY_STAR,
    NOKIA_KEY_HASH,
    NOKIA_KEY_UP,
    NOKIA_KEY_DOWN,
    NOKIA_KEY_LEFT,
    NOKIA_KEY_RIGHT,
    NOKIA_KEY_OK,
    NOKIA_KEY_LSK,     /* Left Soft Key */
    NOKIA_KEY_RSK,     /* Right Soft Key */
    NOKIA_KEY_CALL,    /* Green Call Key */
    NOKIA_KEY_END,     /* Red End / Power Key */
    NOKIA_KEY_SOS,     /* Emergency Button on Rear */
    NOKIA_KEY_VOL_UP,
    NOKIA_KEY_VOL_DOWN,
    NOKIA_KEY_COUNT
} NokiaKeyCode;

struct SprdKpdState {
    /*< private >*/
    SysBusDevice parent_obj;

    /*< public >*/
    MemoryRegion iomem;
    qemu_irq irq;

    uint32_t ctrl;
    uint32_t int_en;
    uint32_t int_raw;
    uint32_t polarity;
    uint32_t clk_div;
    uint32_t status0;
    uint32_t status1;

    bool keys_pressed[NOKIA_KEY_COUNT];
};

DeviceState *sprd_kpd_create(hwaddr addr, qemu_irq irq);
void sprd_kpd_inject_key(SprdKpdState *s, NokiaKeyCode key, bool pressed);

#endif /* HW_INPUT_SPRD_KPD_H */
