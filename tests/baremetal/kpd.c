/*
 * Baremetal Keypad and Flip Sensor driver for N2660Emu
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "kpd.h"

static volatile uint32_t *const kpd_regs = (volatile uint32_t *)SPRD_KPD_BASE;
static volatile uint32_t *const gpio_regs = (volatile uint32_t *)SPRD_GPIO_BASE;

void kpd_init(void)
{
    /* 1. Configure Flip Sensor GPIO pin (Bank 0 Pin 0 as Input) */
    gpio_regs[GPIO_DIR / 4] &= ~GPIO_FLIP_SENSOR_PIN;

    /* 2. Enable Keypad Matrix Controller (Bit 0 = Enable) */
    kpd_regs[SPRD_KPD_CTRL / 4] = 0x00000001;

    /* 3. Enable Keypad Interrupt */
    kpd_regs[SPRD_KPD_INT_EN / 4] = 0x00000001;

    /* 4. Clear any pending interrupts */
    kpd_clear_interrupt();
}

int kpd_read_flip_sensor(void)
{
    /* Read GPIO Bank 0 Data: Pin 0 high indicates Flip is Open, low indicates Closed */
    uint32_t data = gpio_regs[GPIO_DATA / 4];
    return (data & GPIO_FLIP_SENSOR_PIN) ? 1 : 0;
}

void kpd_read_status(uint32_t *status0, uint32_t *status1)
{
    if (status0) {
        *status0 = kpd_regs[SPRD_KPD_STATUS0 / 4];
    }
    if (status1) {
        *status1 = kpd_regs[SPRD_KPD_STATUS1 / 4];
    }
}

int kpd_is_key_pressed(KeypadKeyCode key)
{
    if (key >= KEY_COUNT) {
        return 0;
    }

    if (key < 16) {
        uint32_t s0 = kpd_regs[SPRD_KPD_STATUS0 / 4];
        return (s0 & (1U << key)) ? 1 : 0;
    } else {
        uint32_t s1 = kpd_regs[SPRD_KPD_STATUS1 / 4];
        return (s1 & (1U << (key - 16))) ? 1 : 0;
    }
}

void kpd_clear_interrupt(void)
{
    kpd_regs[SPRD_KPD_INT_CLR / 4] = 0x00000001;
}

void kpd_enable_interrupt(void)
{
    kpd_regs[SPRD_KPD_INT_EN / 4] = 0x00000001;
}

void kpd_disable_interrupt(void)
{
    kpd_regs[SPRD_KPD_INT_EN / 4] = 0x00000000;
}
