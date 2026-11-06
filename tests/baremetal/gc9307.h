/*
 * Baremetal GalaxyCore GC9307 Display Driver
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef TEST_GC9307_H
#define TEST_GC9307_H

#include <stdint.h>

#define GC9307_WIDTH   240
#define GC9307_HEIGHT  320

/* Standard RGB565 Colors */
#define COLOR_BLACK       0x0000
#define COLOR_WHITE       0xFFFF
#define COLOR_RED         0xF800
#define COLOR_GREEN       0x07E0
#define COLOR_BLUE        0x001F
#define COLOR_YELLOW      0xFFE0
#define COLOR_CYAN        0x07FF
#define COLOR_MAGENTA     0xF81F
#define COLOR_NOKIA_BLUE  0x0215  /* Classic deep Nokia brand blue */

void gc9307_init(void);
void gc9307_set_window(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1);
void gc9307_fill_rect(uint16_t x0, uint16_t y0, uint16_t w, uint16_t h, uint16_t color);
void gc9307_draw_test_screen(void);

#endif /* TEST_GC9307_H */
