/*
 * Baremetal Keypad and Flip Sensor driver for N2660Emu
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef TEST_KPD_H
#define TEST_KPD_H

#include <stdint.h>

#define SPRD_KPD_BASE         0x50060000
#define SPRD_KPD_CTRL         0x0000
#define SPRD_KPD_INT_EN       0x0004
#define SPRD_KPD_INT_RAW_STS  0x0008
#define SPRD_KPD_INT_MASK_STS 0x000C
#define SPRD_KPD_INT_CLR      0x0010
#define SPRD_KPD_POLARITY     0x0014
#define SPRD_KPD_CLK_DIV      0x0018
#define SPRD_KPD_STATUS0      0x001C
#define SPRD_KPD_STATUS1      0x0020

#define SPRD_GPIO_BASE        0x50040000
#define GPIO_DATA             0x0000
#define GPIO_DIR              0x0008
#define GPIO_FLIP_SENSOR_PIN  (1U << 0) /* Pin 0: 1 = Flip Open, 0 = Flip Closed */

/* Nokia 2660 Key Codes */
typedef enum {
    KEY_0 = 0,
    KEY_1,
    KEY_2,
    KEY_3,
    KEY_4,
    KEY_5,
    KEY_6,
    KEY_7,
    KEY_8,
    KEY_9,
    KEY_STAR,
    KEY_HASH,
    KEY_UP,
    KEY_DOWN,
    KEY_LEFT,
    KEY_RIGHT,
    KEY_OK,
    KEY_LSK,
    KEY_RSK,
    KEY_CALL,
    KEY_END,
    KEY_SOS,
    KEY_VOL_UP,
    KEY_VOL_DOWN,
    KEY_COUNT
} KeypadKeyCode;

void kpd_init(void);
int kpd_read_flip_sensor(void);
void kpd_read_status(uint32_t *status0, uint32_t *status1);
int kpd_is_key_pressed(KeypadKeyCode key);
void kpd_clear_interrupt(void);

#endif /* TEST_KPD_H */
