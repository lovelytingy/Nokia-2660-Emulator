/*
 * Baremetal Unisoc SPRD Serial Flash Controller (SFC) & SPI NAND Driver
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef TEST_SFC_H
#define TEST_SFC_H

#include <stdint.h>

#define SPRD_SFC_BASE          0x500A0000
#define SPRD_FLASH_DIRECT_BASE 0x60000000

#define SFC_CMD_REG            0x0000
#define SFC_ADDR_REG           0x0004
#define SFC_DATA_REG           0x0008
#define SFC_CTL_REG            0x000C
#define SFC_STS_REG            0x0010
#define SFC_BUF_PTR_REG        0x0014

#define NAND_CMD_RESET         0xFF
#define NAND_CMD_READ_ID       0x9F
#define NAND_CMD_PAGE_READ     0x13
#define NAND_CMD_READ_CACHE    0x03
#define NAND_CMD_WRITE_ENABLE  0x06
#define NAND_CMD_WRITE_DISABLE 0x04
#define NAND_CMD_PROGRAM_LOAD  0x02
#define NAND_CMD_PROGRAM_EXEC  0x10
#define NAND_CMD_BLOCK_ERASE   0xD8

#define JEDEC_MFR_WINBOND      0xEF
#define JEDEC_DEV_W25N01GV     0xAA21

#define NAND_PAGE_SIZE         2048
#define NAND_BLOCK_SIZE        (64 * 2048) /* 128 KB */

void sfc_init(void);
uint32_t sfc_read_jedec_id(void);
void sfc_page_read(uint32_t page_addr);
void sfc_read_cache(uint16_t col_offset, uint8_t *buf, uint32_t len);
void sfc_program_page(uint32_t page_addr, const uint8_t *data, uint32_t len);
void sfc_block_erase(uint32_t block_addr);

#endif /* TEST_SFC_H */
