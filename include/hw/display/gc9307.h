/*
 * GalaxyCore GC9307 QVGA (240x320) LCD Controller Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_DISPLAY_GC9307_H
#define HW_DISPLAY_GC9307_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qom/object.h"

#define TYPE_GC9307 "gc9307"
OBJECT_DECLARE_SIMPLE_TYPE(GC9307State, GC9307)

#define GC9307_WIDTH   240
#define GC9307_HEIGHT  320

/* GC9307 / Standard MIPI-DCS Commands */
#define GC9307_CMD_NOP       0x00
#define GC9307_CMD_SWRESET   0x01
#define GC9307_CMD_SLPIN     0x10
#define GC9307_CMD_SLPOUT    0x11
#define GC9307_CMD_DISPOFF   0x28
#define GC9307_CMD_DISPON    0x29
#define GC9307_CMD_CASET     0x2A  /* Column Address Set */
#define GC9307_CMD_RASET     0x2B  /* Row Address Set */
#define GC9307_CMD_RAMWR     0x2C  /* Memory Write */
#define GC9307_CMD_MADCTL    0x36  /* Memory Access Control */
#define GC9307_CMD_COLMOD    0x3A  /* Pixel Format Set (RGB565) */

typedef enum {
    GC9307_STATE_CMD = 0,
    GC9307_STATE_PARAM,
    GC9307_STATE_PIXELS
} GC9307InputState;

struct GC9307State {
    /*< private >*/
    DeviceState parent_obj;

    /*< public >*/
    uint16_t framebuffer[GC9307_HEIGHT][GC9307_WIDTH]; /* RGB565 */

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
    GC9307InputState state;
    uint8_t current_cmd;
    uint8_t param_buf[8];
    uint8_t param_idx;
    uint8_t param_expected;

    /* 16-bit pixel byte assembly */
    uint8_t pixel_byte_high;
    bool pixel_byte_ready;
};

void gc9307_write_cmd(GC9307State *s, uint8_t cmd);
void gc9307_write_data(GC9307State *s, uint8_t data);
void gc9307_reset(DeviceState *dev);

#endif /* HW_DISPLAY_GC9307_H */
