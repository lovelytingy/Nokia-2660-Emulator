/*
 * Baremetal GalaxyCore GC9307 Display Driver Implementation
 * Interfaces via Unisoc SPRD SPI0 (0x50080000).
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "gc9307.h"

#define SPRD_SPI0_BASE   0x50080000
#define SPI_TXD_REG      0x0000
#define SPI_CTL4_REG     0x0018

#define REG32(addr) (*(volatile uint32_t *)(addr))

static void spi_write_cmd(uint8_t cmd)
{
    /* Set D/C low (Command Mode) */
    REG32(SPRD_SPI0_BASE + SPI_CTL4_REG) = 0x0;
    REG32(SPRD_SPI0_BASE + SPI_TXD_REG) = (uint32_t)cmd;
}

static void spi_write_data(uint8_t data)
{
    /* Set D/C high (Data Mode) */
    REG32(SPRD_SPI0_BASE + SPI_CTL4_REG) = 0x1;
    REG32(SPRD_SPI0_BASE + SPI_TXD_REG) = (uint32_t)data;
}

void gc9307_init(void)
{
    /* 1. Software Reset */
    spi_write_cmd(0x01);
    for (volatile int i = 0; i < 100; i++);

    /* 2. Sleep Out */
    spi_write_cmd(0x11);
    for (volatile int i = 0; i < 100; i++);

    /* 3. Pixel Format: 16-bit RGB565 */
    spi_write_cmd(0x3A);
    spi_write_data(0x55);

    /* 4. Display On */
    spi_write_cmd(0x29);
}

void gc9307_set_window(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1)
{
    /* Column Address Set (0x2A) */
    spi_write_cmd(0x2A);
    spi_write_data((uint8_t)(x0 >> 8));
    spi_write_data((uint8_t)(x0 & 0xFF));
    spi_write_data((uint8_t)(x1 >> 8));
    spi_write_data((uint8_t)(x1 & 0xFF));

    /* Row Address Set (0x2B) */
    spi_write_cmd(0x2B);
    spi_write_data((uint8_t)(y0 >> 8));
    spi_write_data((uint8_t)(y0 & 0xFF));
    spi_write_data((uint8_t)(y1 >> 8));
    spi_write_data((uint8_t)(y1 & 0xFF));

    /* Memory Write (0x2C) */
    spi_write_cmd(0x2C);
}

void gc9307_fill_rect(uint16_t x0, uint16_t y0, uint16_t w, uint16_t h, uint16_t color)
{
    gc9307_set_window(x0, y0, (uint16_t)(x0 + w - 1), (uint16_t)(y0 + h - 1));

    uint8_t hi = (uint8_t)(color >> 8);
    uint8_t lo = (uint8_t)(color & 0xFF);

    uint32_t total_pixels = (uint32_t)w * (uint32_t)h;
    for (uint32_t i = 0; i < total_pixels; i++) {
        spi_write_data(hi);
        spi_write_data(lo);
    }
}

void gc9307_draw_test_screen(void)
{
    /* 8 Classic Test Color Bars across 240x240 upper screen */
    /* 8 bars x 30 pixels width each = 240 pixels width */
    uint16_t colors[8] = {
        COLOR_WHITE,
        COLOR_YELLOW,
        COLOR_CYAN,
        COLOR_GREEN,
        COLOR_MAGENTA,
        COLOR_RED,
        COLOR_BLUE,
        COLOR_BLACK
    };

    for (int i = 0; i < 8; i++) {
        gc9307_fill_rect((uint16_t)(i * 30), 0, 30, 240, colors[i]);
    }

    /* Bottom 80 pixels: Deep Nokia Blue footer banner */
    gc9307_fill_rect(0, 240, 240, 80, COLOR_NOKIA_BLUE);

    /* Draw small white accent box in the footer */
    gc9307_fill_rect(80, 270, 80, 20, COLOR_WHITE);
}
