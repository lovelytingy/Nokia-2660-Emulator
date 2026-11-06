/*
 * Unisoc / Spreadtrum Boot Serial Loader (BSL) baremetal protocol driver
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef BSL_H
#define BSL_H

#include <stdint.h>
#include <stddef.h>

#define HDLC_HEADER             0x7E
#define HDLC_ESCAPE             0x7D

/* BSL Command Codes */
#define BSL_CMD_CONNECT         0x0000
#define BSL_CMD_START_DATA      0x0001
#define BSL_CMD_MIDST_DATA      0x0002
#define BSL_CMD_END_DATA        0x0003
#define BSL_CMD_EXEC_DATA       0x0004
#define BSL_CMD_NORMAL_RESET    0x0005
#define BSL_CMD_READ_FLASH      0x0006
#define BSL_CMD_READ_CHIP_TYPE  0x0007
#define BSL_CMD_CHECK_BAUD      0x007E

/* BSL Response Codes */
#define BSL_REP_ACK             0x0080
#define BSL_REP_VER             0x0081
#define BSL_REP_INVALID_CMD     0x0082
#define BSL_REP_UNKNOWN_CMD     0x0083
#define BSL_REP_OPERATION_FAILED 0x0084
#define BSL_REP_CHECKSUM_ERROR  0x0085
#define BSL_REP_NOT_SUPPORT     0x0090

/* CCITT-16 CRC matching Spreadtrum BootROM */
uint16_t bsl_crc16(uint16_t crc, const uint8_t *src, size_t len);

/* Encodes packet into HDLC frame with byte stuffing and CRC16 */
int bsl_encode(uint16_t type, const uint8_t *payload, uint16_t len, uint8_t *out_buf, size_t max_out);

/* Decodes HDLC frame, verifies CRC16, and unescapes payload */
int bsl_decode(const uint8_t *frame, size_t frame_len, uint16_t *type, uint8_t *payload, uint16_t *len);

#endif /* BSL_H */
