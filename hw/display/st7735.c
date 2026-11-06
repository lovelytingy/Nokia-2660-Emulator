/*
 * Sitronix ST7735 / GalaxyCore GC9106 Secondary Sub-Display (120x160) Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qemu/log.h"
#include "qemu/module.h"
#include "hw/display/st7735.h"

void st7735_reset(DeviceState *dev)
{
    ST7735State *s = ST7735(dev);

    memset(s->framebuffer, 0, sizeof(s->framebuffer));

    s->sleep_mode = true;
    s->display_on = false;
    s->colmod = 0x55; /* RGB565 */
    s->madctl = 0x00;

    s->col_start = 0;
    s->col_end = ST7735_WIDTH - 1;
    s->row_start = 0;
    s->row_end = ST7735_HEIGHT - 1;

    s->cur_x = 0;
    s->cur_y = 0;

    s->state = ST7735_STATE_CMD;
    s->current_cmd = ST7735_CMD_NOP;
    s->param_idx = 0;
    s->param_expected = 0;
    s->pixel_byte_ready = false;
}

void st7735_write_cmd(ST7735State *s, uint8_t cmd)
{
    s->current_cmd = cmd;
    s->param_idx = 0;
    s->pixel_byte_ready = false;

    switch (cmd) {
    case ST7735_CMD_NOP:
        break;

    case ST7735_CMD_SWRESET:
        st7735_reset(DEVICE(s));
        break;

    case ST7735_CMD_SLPIN:
        s->sleep_mode = true;
        break;

    case ST7735_CMD_SLPOUT:
        s->sleep_mode = false;
        break;

    case ST7735_CMD_DISPOFF:
        s->display_on = false;
        break;

    case ST7735_CMD_DISPON:
        s->display_on = true;
        break;

    case ST7735_CMD_CASET:
        s->state = ST7735_STATE_PARAM;
        s->param_expected = 4;
        break;

    case ST7735_CMD_RASET:
        s->state = ST7735_STATE_PARAM;
        s->param_expected = 4;
        break;

    case ST7735_CMD_RAMWR:
        s->state = ST7735_STATE_PIXELS;
        s->cur_x = s->col_start;
        s->cur_y = s->row_start;
        break;

    case ST7735_CMD_MADCTL:
        s->state = ST7735_STATE_PARAM;
        s->param_expected = 1;
        break;

    case ST7735_CMD_COLMOD:
        s->state = ST7735_STATE_PARAM;
        s->param_expected = 1;
        break;

    default:
        qemu_log_mask(LOG_UNIMP, "st7735: Unimplemented command 0x%02X\n", cmd);
        s->state = ST7735_STATE_CMD;
        break;
    }
}

void st7735_write_data(ST7735State *s, uint8_t data)
{
    if (s->state == ST7735_STATE_PARAM) {
        if (s->param_idx < 4) {
            s->params[s->param_idx++] = data;
        }

        if (s->param_idx >= s->param_expected) {
            switch (s->current_cmd) {
            case ST7735_CMD_CASET: {
                uint16_t start = (s->params[0] << 8) | s->params[1];
                uint16_t end   = (s->params[2] << 8) | s->params[3];
                s->col_start = MIN(start, ST7735_WIDTH - 1);
                s->col_end   = MIN(end, ST7735_WIDTH - 1);
                break;
            }
            case ST7735_CMD_RASET: {
                uint16_t start = (s->params[0] << 8) | s->params[1];
                uint16_t end   = (s->params[2] << 8) | s->params[3];
                s->row_start = MIN(start, ST7735_HEIGHT - 1);
                s->row_end   = MIN(end, ST7735_HEIGHT - 1);
                break;
            }
            case ST7735_CMD_COLMOD:
                s->colmod = s->params[0];
                break;
            case ST7735_CMD_MADCTL:
                s->madctl = s->params[0];
                break;
            }
            s->state = ST7735_STATE_CMD;
        }
    } else if (s->state == ST7735_STATE_PIXELS) {
        if (!s->pixel_byte_ready) {
            s->pixel_high_byte = data;
            s->pixel_byte_ready = true;
        } else {
            uint16_t color = (s->pixel_high_byte << 8) | data;
            s->pixel_byte_ready = false;

            if (s->cur_y < ST7735_HEIGHT && s->cur_x < ST7735_WIDTH) {
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

static void st7735_instance_init(Object *obj)
{
    ST7735State *s = ST7735(obj);
    st7735_reset(DEVICE(s));
}

static void st7735_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);
    dc->reset = st7735_reset;
    dc->desc = "Sitronix ST7735 / GC9106 1.77 QQVGA LCD (120x160)";
}

static const TypeInfo st7735_info = {
    .name          = TYPE_ST7735,
    .parent        = TYPE_DEVICE,
    .instance_size = sizeof(ST7735State),
    .instance_init = st7735_instance_init,
    .class_init    = st7735_class_init,
};

static void st7735_register_types(void)
{
    type_register_static(&st7735_info);
}

type_init(st7735_register_types)
