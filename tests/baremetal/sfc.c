/*
 * Baremetal Unisoc SPRD Serial Flash Controller (SFC) & SPI NAND Driver Implementation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "sfc.h"

#define REG32(addr) (*(volatile uint32_t *)(addr))

void sfc_init(void)
{
    /* Issue SPI NAND Reset */
    REG32(SPRD_SFC_BASE + SFC_CMD_REG) = NAND_CMD_RESET;
    for (volatile int i = 0; i < 50; i++);
}

uint32_t sfc_read_jedec_id(void)
{
    REG32(SPRD_SFC_BASE + SFC_CMD_REG) = NAND_CMD_READ_ID;
    uint32_t raw = REG32(SPRD_SFC_BASE + SFC_DATA_REG);
    uint32_t mfr = raw & 0xFF;
    uint32_t d0  = (raw >> 8) & 0xFF;
    uint32_t d1  = (raw >> 16) & 0xFF;
    return (mfr << 16) | (d0 << 8) | d1; /* Returns 0xEFAA21 for Winbond W25N01GV */
}

void sfc_page_read(uint32_t page_addr)
{
    REG32(SPRD_SFC_BASE + SFC_ADDR_REG) = page_addr;
    REG32(SPRD_SFC_BASE + SFC_CMD_REG) = NAND_CMD_PAGE_READ;
}

void sfc_read_cache(uint16_t col_offset, uint8_t *buf, uint32_t len)
{
    REG32(SPRD_SFC_BASE + SFC_ADDR_REG) = (uint32_t)col_offset;
    REG32(SPRD_SFC_BASE + SFC_CMD_REG) = NAND_CMD_READ_CACHE;

    uint32_t words = len / 4;
    uint32_t *p32 = (uint32_t *)buf;
    for (uint32_t i = 0; i < words; i++) {
        p32[i] = REG32(SPRD_SFC_BASE + SFC_DATA_REG);
    }

    uint32_t rem = len % 4;
    if (rem > 0) {
        uint32_t val = REG32(SPRD_SFC_BASE + SFC_DATA_REG);
        for (uint32_t i = 0; i < rem; i++) {
            buf[words * 4 + i] = (val >> (i * 8)) & 0xFF;
        }
    }
}

void sfc_program_page(uint32_t page_addr, const uint8_t *data, uint32_t len)
{
    /* 1. Write Enable */
    REG32(SPRD_SFC_BASE + SFC_CMD_REG) = NAND_CMD_WRITE_ENABLE;

    /* 2. Program Load (load into cache buffer starting at col 0) */
    REG32(SPRD_SFC_BASE + SFC_ADDR_REG) = 0x0000;
    REG32(SPRD_SFC_BASE + SFC_CMD_REG) = NAND_CMD_PROGRAM_LOAD;

    uint32_t words = (len + 3) / 4;
    const uint32_t *p32 = (const uint32_t *)data;
    for (uint32_t i = 0; i < words; i++) {
        REG32(SPRD_SFC_BASE + SFC_DATA_REG) = p32[i];
    }

    /* 3. Program Execute (writes buffer to flash array page) */
    REG32(SPRD_SFC_BASE + SFC_ADDR_REG) = page_addr;
    REG32(SPRD_SFC_BASE + SFC_CMD_REG) = NAND_CMD_PROGRAM_EXEC;
}

void sfc_block_erase(uint32_t block_addr)
{
    /* 1. Write Enable */
    REG32(SPRD_SFC_BASE + SFC_CMD_REG) = NAND_CMD_WRITE_ENABLE;

    /* 2. Block Erase */
    REG32(SPRD_SFC_BASE + SFC_ADDR_REG) = block_addr * 64; /* Page 0 of target block */
    REG32(SPRD_SFC_BASE + SFC_CMD_REG) = NAND_CMD_BLOCK_ERASE;
}
