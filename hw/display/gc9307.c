/*
 * GalaxyCore GC9307 QVGA (240x320) LCD Controller Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qemu/log.h"
#include "qemu/module.h"
#include "hw/display/gc9307.h"

void gc9307_reset(DeviceState *dev)
{
    GC9307State *s = GC9307(dev);

    memset(s->framebuffer, 0, sizeof(s->framebuffer));

    s->sleep_mode = true;
    s->display_on = false;
    s->colmod = 0x55; /* RGB565 */
    s->madctl = 0x00;

    s->col_start = 0;
    s->col_end = GC9307_WIDTH - 1;
    s->row_start = 0;
    s->row_end = GC9307_HEIGHT - 1;

    s->cur_x = 0;
    s->cur_y = 0;

    s->state = GC9307_STATE_CMD;
    s->current_cmd = GC9307_CMD_NOP;
    s->param_idx = 0;
    s->param_expected = 0;
    s->pixel_byte_ready = false;
}

void gc9307_write_cmd(GC9307State *s, uint8_t cmd)
{
    s->current_cmd = cmd;
    s->param_idx = 0;
    s->pixel_byte_ready = false;

    switch (cmd) {
    case GC9307_CMD_NOP:
        break;

    case GC9307_CMD_SWRESET:
        gc9307_reset(DEVICE(s));
        break;

    case GC9307_CMD_SLPIN:
        s->sleep_mode = true;
        break;

    case GC9307_CMD_SLPOUT:
        s->sleep_mode = false;
        break;

    case GC9307_CMD_DISPOFF:
        s->display_on = false;
        break;

    case GC9307_CMD_DISPON:
        s->display_on = true;
        break;

    case GC9307_CMD_CASET:
        s->state = GC9307_STATE_PARAM;
        s->param_expected = 4;
        break;

    case GC9307_CMD_RASET:
        s->state = GC9307_STATE_PARAM;
        s->param_expected = 4;
        break;

    case GC9307_CMD_RAMWR:
        s->state = GC9307_STATE_PIXELS;
        s->cur_x = s->col_start;
        s->cur_y = s->row_start;
        break;

    case GC9307_CMD_MADCTL:
        s->state = GC9307_STATE_PARAM;
        s->param_expected = 1;
        break;

    case GC9307_CMD_COLMOD:
        s->state = GC9307_STATE_PARAM;
        s->param_expected = 1;
        break;

    default:
        qemu_log_mask(LOG_UNIMP, "gc9307: Unimplemented command 0x%02X\n", cmd);
        s->state = GC9307_STATE_CMD;
        break;
    }
}

void gc9307_write_data(GC9307State *s, uint8_t data)
{
    if (s->state == GC9307_STATE_PARAM) {
        if (s->param_idx < sizeof(s->param_buf)) {
            s->param_buf[s->param_idx++] = data;
        }

        if (s->param_idx >= s->param_expected) {
            /* Execute parameterized command */
            switch (s->current_cmd) {
            case GC9307_CMD_CASET:
                s->col_start = (s->param_buf[0] << 8) | s->param_buf[1];
                s->col_end   = (s->param_buf[2] << 8) | s->param_buf[3];
                if (s->col_end >= GC9307_WIDTH) {
                    s->col_end = GC9307_WIDTH - 1;
                }
                break;

            case GC9307_CMD_RASET:
                s->row_start = (s->param_buf[0] << 8) | s->param_buf[1];
                s->row_end   = (s->param_buf[2] << 8) | s->param_buf[3];
                if (s->row_end >= GC9307_HEIGHT) {
                    s->row_end = GC9307_HEIGHT - 1;
                }
                break;

            case GC9307_CMD_MADCTL:
                s->madctl = s->param_buf[0];
                break;

            case GC9307_CMD_COLMOD:
                s->colmod = s->param_buf[0];
                break;
            }
            s->state = GC9307_STATE_CMD;
        }
    } else if (s->state == GC9307_STATE_PIXELS) {
        if (!s->pixel_byte_ready) {
            s->pixel_byte_high = data;
            s->pixel_byte_ready = true;
        } else {
            uint16_t color = (s->pixel_byte_high << 8) | data;
            s->pixel_byte_ready = false;

            if (s->cur_y < GC9307_HEIGHT && s->cur_x < GC9307_WIDTH) {
                s->framebuffer[s->cur_y][s->cur_x] = color;
            }

            s->cur_x++;
            if (s->cur_x > s->col_end) {
                s->cur_x = s->col_start;
                s->cur_y++;
                if (s->cur_y > s->row_end) {
                    s->cur_y = s->row_start;
                }
            }
        }
    }
}

static void gc9307_realize(DeviceState *dev, Error **errp)
{
    gc9307_reset(dev);
}

static void gc9307_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);
    dc->realize = gc9307_realize;
    dc->reset = gc9307_reset;
}

static const TypeInfo gc9307_info = {
    .name = TYPE_GC9307,
    .parent = TYPE_DEVICE,
    .instance_size = sizeof(GC9307State),
    .class_init = gc9307_class_init,
};

static void gc9307_register_types(void)
{
    type_register_static(&gc9307_info);
}

type_init(gc9307_register_types)
