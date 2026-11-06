/*
 * Unisoc / Spreadtrum Boot Serial Loader (BSL) baremetal protocol driver
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "bsl.h"

uint16_t bsl_crc16(uint16_t crc, const uint8_t *src, size_t len)
{
    while (len--) {
        crc ^= (*src++) << 8;
        for (int i = 0; i < 8; i++) {
            if (crc & 0x8000) {
                crc = ((crc << 1) ^ 0x1021) & 0xFFFF;
            } else {
                crc = (crc << 1) & 0xFFFF;
            }
        }
    }
    return crc;
}

int bsl_encode(uint16_t type, const uint8_t *payload, uint16_t len, uint8_t *out_buf, size_t max_out)
{
    uint8_t raw[512];
    if ((size_t)(4 + len + 2) > sizeof(raw)) {
        return -1;
    }

    /* Type (big-endian) */
    raw[0] = (uint8_t)(type >> 8);
    raw[1] = (uint8_t)(type & 0xFF);

    /* Length (big-endian) */
    raw[2] = (uint8_t)(len >> 8);
    raw[3] = (uint8_t)(len & 0xFF);

    /* Payload */
    for (uint16_t i = 0; i < len; i++) {
        raw[4 + i] = payload[i];
    }

    /* Calculate CRC16 over type, len, and payload */
    uint16_t crc = bsl_crc16(0, raw, 4 + len);
    raw[4 + len] = (uint8_t)(crc >> 8);
    raw[4 + len + 1] = (uint8_t)(crc & 0xFF);

    size_t raw_len = 4 + len + 2;
    size_t out_idx = 0;

    if (out_idx >= max_out) return -1;
    out_buf[out_idx++] = HDLC_HEADER;

    for (size_t i = 0; i < raw_len; i++) {
        uint8_t b = raw[i];
        if (b == HDLC_HEADER || b == HDLC_ESCAPE) {
            if (out_idx + 2 >= max_out) return -1;
            out_buf[out_idx++] = HDLC_ESCAPE;
            out_buf[out_idx++] = b ^ 0x20;
        } else {
            if (out_idx >= max_out) return -1;
            out_buf[out_idx++] = b;
        }
    }

    if (out_idx >= max_out) return -1;
    out_buf[out_idx++] = HDLC_HEADER;

    return (int)out_idx;
}

int bsl_decode(const uint8_t *frame, size_t frame_len, uint16_t *type, uint8_t *payload, uint16_t *len)
{
    if (frame_len < 3) return -1;

    size_t start = 0;
    size_t end = frame_len;

    if (frame[start] == HDLC_HEADER) start++;
    if (end > start && frame[end - 1] == HDLC_HEADER) end--;

    uint8_t unescaped[512];
    size_t u_len = 0;

    for (size_t i = start; i < end; i++) {
        uint8_t b = frame[i];
        if (b == HDLC_ESCAPE) {
            i++;
            if (i >= end) return -1;
            if (u_len >= sizeof(unescaped)) return -1;
            unescaped[u_len++] = frame[i] ^ 0x20;
        } else {
            if (u_len >= sizeof(unescaped)) return -1;
            unescaped[u_len++] = b;
        }
    }

    if (u_len < 6) return -1; /* 2 type + 2 len + 2 crc */

    uint16_t pkt_type = ((uint16_t)unescaped[0] << 8) | unescaped[1];
    uint16_t pkt_len = ((uint16_t)unescaped[2] << 8) | unescaped[3];

    if (u_len != (size_t)(4 + pkt_len + 2)) {
        return -1;
    }

    uint16_t rx_crc = ((uint16_t)unescaped[4 + pkt_len] << 8) | unescaped[4 + pkt_len + 1];
    uint16_t calc_crc = bsl_crc16(0, unescaped, 4 + pkt_len);

    if (rx_crc != calc_crc) {
        return -2; /* CRC error */
    }

    if (type) *type = pkt_type;
    if (len) *len = pkt_len;
    if (payload && pkt_len > 0) {
        for (uint16_t i = 0; i < pkt_len; i++) {
            payload[i] = unescaped[4 + i];
        }
    }

    return 0;
}
