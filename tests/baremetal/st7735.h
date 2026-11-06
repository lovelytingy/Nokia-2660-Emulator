/*
 * Baremetal Sitronix ST7735 / GC9106 Outer Sub-Display Driver
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef TEST_ST7735_H
#define TEST_ST7735_H

#include <stdint.h>

#define ST7735_WIDTH   120
#define ST7735_HEIGHT  160

/* Standard RGB565 Colors */
#define COLOR_SUB_BLACK       0x0000
#define COLOR_SUB_WHITE       0xFFFF
#define COLOR_SUB_RED         0xF800
#define COLOR_SUB_GREEN       0x07E0
#define COLOR_SUB_BLUE        0x001F
#define COLOR_SUB_YELLOW      0xFFE0
#define COLOR_SUB_CYAN        0x07FF
#define COLOR_SUB_NOKIA_BLUE  0x0215
#define COLOR_SUB_DARK_GREY   0x2104

void st7735_init(void);
void st7735_set_window(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1);
void st7735_fill_rect(uint16_t x0, uint16_t y0, uint16_t w, uint16_t h, uint16_t color);
void st7735_draw_test_screen(void);

#endif /* TEST_ST7735_H */
