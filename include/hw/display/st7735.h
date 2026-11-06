/*
 * Sitronix ST7735 / GalaxyCore GC9106 Secondary Sub-Display (120x160) Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_DISPLAY_ST7735_H
#define HW_DISPLAY_ST7735_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qom/object.h"

#define TYPE_ST7735 "st7735"
OBJECT_DECLARE_SIMPLE_TYPE(ST7735State, ST7735)

#define ST7735_WIDTH   120
#define ST7735_HEIGHT  160

/* Standard MIPI-DCS / ST7735 Commands */
#define ST7735_CMD_NOP       0x00
#define ST7735_CMD_SWRESET   0x01
#define ST7735_CMD_SLPIN     0x10
#define ST7735_CMD_SLPOUT    0x11
#define ST7735_CMD_DISPOFF   0x28
#define ST7735_CMD_DISPON    0x29
#define ST7735_CMD_CASET     0x2A  /* Column Address Set */
#define ST7735_CMD_RASET     0x2B  /* Row Address Set */
#define ST7735_CMD_RAMWR     0x2C  /* Memory Write */
#define ST7735_CMD_MADCTL    0x36  /* Memory Access Control */
#define ST7735_CMD_COLMOD    0x3A  /* Pixel Format Set */

typedef enum {
    ST7735_STATE_CMD = 0,
    ST7735_STATE_PARAM,
    ST7735_STATE_PIXELS
} ST7735InputState;

struct ST7735State {
    /*< private >*/
    DeviceState parent_obj;

    /*< public >*/
    uint16_t framebuffer[ST7735_HEIGHT][ST7735_WIDTH]; /* RGB565 */

    bool sleep_mode;
    bool display_on;
    uint8_t colmod;
    uint8_t madctl;

    /* Window coordinates */
    uint16_t col_start;
    uint16_t col_end;
    uint16_t row_start;
    uint16_t row_end;

    /* Current write pointer */
    uint16_t cur_x;
    uint16_t cur_y;

    /* Protocol parser state */
    ST7735InputState state;
    uint8_t current_cmd;
    uint8_t param_idx;
    uint8_t param_expected;
    uint8_t params[4];

    bool pixel_byte_ready;
    uint8_t pixel_high_byte;
};

void st7735_reset(DeviceState *dev);
void st7735_write_cmd(ST7735State *s, uint8_t cmd);
void st7735_write_data(ST7735State *s, uint8_t data);

#endif /* HW_DISPLAY_ST7735_H */
