/*
 * 5x7 ASCII Bitmap Font for Baremetal Display Rendering
 * Supports ASCII 32 (Space) to 126 (~).
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef TEST_FONT5X7_H
#define TEST_FONT5X7_H

#include <stdint.h>

extern const uint8_t font5x7[96][5];

void gc9307_draw_char(int x, int y, char c, uint16_t color, uint16_t bg, int scale);
void gc9307_draw_string(int x, int y, const char *str, uint16_t color, uint16_t bg, int scale);

void st7735_draw_char(int x, int y, char c, uint16_t color, uint16_t bg, int scale);
void st7735_draw_string(int x, int y, const char *str, uint16_t color, uint16_t bg, int scale);

#endif /* TEST_FONT5X7_H */
