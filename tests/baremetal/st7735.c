/*
 * Baremetal Sitronix ST7735 / GC9106 Outer Sub-Display Driver Implementation
 * Interfaces via Unisoc SPRD SPI1 (0x50090000).
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "st7735.h"

#define SPRD_SPI1_BASE   0x50090000
#define SPI_TXD_REG      0x0000
#define SPI_CTL4_REG     0x0018

#define REG32(addr) (*(volatile uint32_t *)(addr))

static void sub_spi_write_cmd(uint8_t cmd)
{
    /* Set D/C low (Command Mode) */
    REG32(SPRD_SPI1_BASE + SPI_CTL4_REG) = 0x0;
    REG32(SPRD_SPI1_BASE + SPI_TXD_REG) = (uint32_t)cmd;
}

static void sub_spi_write_data(uint8_t data)
{
    /* Set D/C high (Data Mode) */
    REG32(SPRD_SPI1_BASE + SPI_CTL4_REG) = 0x1;
    REG32(SPRD_SPI1_BASE + SPI_TXD_REG) = (uint32_t)data;
}

void st7735_init(void)
{
    /* 1. Software Reset */
    sub_spi_write_cmd(0x01);
    for (volatile int i = 0; i < 100; i++);

    /* 2. Sleep Out */
    sub_spi_write_cmd(0x11);
    for (volatile int i = 0; i < 100; i++);

    /* 3. Pixel Format: 16-bit RGB565 */
    sub_spi_write_cmd(0x3A);
    sub_spi_write_data(0x55);

    /* 4. Display On */
    sub_spi_write_cmd(0x29);
}

void st7735_set_window(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1)
{
    /* Column Address Set (CASET) */
    sub_spi_write_cmd(0x2A);
    sub_spi_write_data((x0 >> 8) & 0xFF);
    sub_spi_write_data(x0 & 0xFF);
    sub_spi_write_data((x1 >> 8) & 0xFF);
    sub_spi_write_data(x1 & 0xFF);

    /* Row Address Set (RASET) */
    sub_spi_write_cmd(0x2B);
    sub_spi_write_data((y0 >> 8) & 0xFF);
    sub_spi_write_data(y0 & 0xFF);
    sub_spi_write_data((y1 >> 8) & 0xFF);
    sub_spi_write_data(y1 & 0xFF);

    /* Memory Write (RAMWR) */
    sub_spi_write_cmd(0x2C);
}

void st7735_fill_rect(uint16_t x0, uint16_t y0, uint16_t w, uint16_t h, uint16_t color)
{
    if (x0 >= ST7735_WIDTH || y0 >= ST7735_HEIGHT || w == 0 || h == 0) {
        return;
    }

    uint16_t x1 = x0 + w - 1;
    uint16_t y1 = y0 + h - 1;
    if (x1 >= ST7735_WIDTH)  x1 = ST7735_WIDTH - 1;
    if (y1 >= ST7735_HEIGHT) y1 = ST7735_HEIGHT - 1;

    st7735_set_window(x0, y0, x1, y1);

    uint8_t hi = (color >> 8) & 0xFF;
    uint8_t lo = color & 0xFF;
    uint32_t count = (uint32_t)(x1 - x0 + 1) * (uint32_t)(y1 - y0 + 1);

    for (uint32_t i = 0; i < count; i++) {
        sub_spi_write_data(hi);
        sub_spi_write_data(lo);
    }
}

void st7735_draw_test_screen(void)
{
    /* 1. Background fill dark slate */
    st7735_fill_rect(0, 0, ST7735_WIDTH, ST7735_HEIGHT, COLOR_SUB_DARK_GREY);

    /* 2. Top status header (Battery / Signal bar) */
    st7735_fill_rect(0, 0, ST7735_WIDTH, 20, COLOR_SUB_BLUE);
    /* Signal strength indicator green square */
    st7735_fill_rect(8, 5, 10, 10, COLOR_SUB_GREEN);
    /* Battery indicator yellow square */
    st7735_fill_rect(ST7735_WIDTH - 20, 5, 12, 10, COLOR_SUB_YELLOW);

    /* 3. Outer screen Digital Clock frame (simulate 12:45 PM) */
    st7735_fill_rect(15, 45, 90, 50, COLOR_SUB_BLACK);
    st7735_fill_rect(20, 50, 80, 40, COLOR_SUB_CYAN);

    /* 4. Notification banner */
    st7735_fill_rect(10, 110, 100, 24, COLOR_SUB_NOKIA_BLUE);

    /* 5. Bottom accent bar */
    st7735_fill_rect(0, ST7735_HEIGHT - 6, ST7735_WIDTH, 6, COLOR_SUB_WHITE);
}
