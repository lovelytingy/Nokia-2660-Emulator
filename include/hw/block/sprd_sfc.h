/*
 * Unisoc / Spreadtrum Serial Flash Controller (SFC) & SPI NAND Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_BLOCK_SPRD_SFC_H
#define HW_BLOCK_SPRD_SFC_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qom/object.h"

#define TYPE_SPRD_SFC "sprd-sfc"
OBJECT_DECLARE_SIMPLE_TYPE(SprdSfcState, SPRD_SFC)

#define SPRD_SFC_BASE          0x500A0000
#define SPRD_SFC_REGS_SIZE     0x1000

#define SPRD_FLASH_DIRECT_BASE 0x60000000
#define SPRD_FLASH_SIZE        (128 * 1024 * 1024) /* 128 MB (1 Gb) */

/* SPI NAND Geometry (Winbond W25N01GV / GigaDevice GD5F1GQ4) */
#define NAND_PAGE_SIZE         2048
#define NAND_SPARE_SIZE        64
#define NAND_PAGE_TOTAL_SIZE   (NAND_PAGE_SIZE + NAND_SPARE_SIZE) /* 2112 bytes */
#define NAND_PAGES_PER_BLOCK   64
#define NAND_BLOCK_SIZE        (NAND_PAGES_PER_BLOCK * NAND_PAGE_SIZE) /* 128 KB */
#define NAND_BLOCK_COUNT       1024 /* 1024 blocks * 128 KB = 128 MB */

/* Register Offsets */
#define SPRD_SFC_CMD           0x0000  /* Command opcode */
#define SPRD_SFC_ADDR          0x0004  /* Page/Block/Column address */
#define SPRD_SFC_DATA          0x0008  /* Data register / FIFO */
#define SPRD_SFC_CTL           0x000C  /* Control: trigger, length, dir */
#define SPRD_SFC_STS           0x0010  /* Status: bit 0 busy, bit 1 done */
#define SPRD_SFC_BUF_PTR       0x0014  /* Cache buffer offset (0-2111) */
#define SPRD_SFC_INT_EN        0x0018  /* Interrupt enable */
#define SPRD_SFC_INT_CLR       0x001C  /* Interrupt clear */

/* SPI NAND Flash Commands */
#define NAND_CMD_RESET         0xFF
#define NAND_CMD_READ_ID       0x9F
#define NAND_CMD_PAGE_READ     0x13    /* Page data read into cache buffer */
#define NAND_CMD_READ_CACHE    0x03    /* Read data from cache buffer */
#define NAND_CMD_READ_FAST     0x0B    /* Fast read data from cache buffer */
#define NAND_CMD_WRITE_ENABLE  0x06
#define NAND_CMD_WRITE_DISABLE 0x04
#define NAND_CMD_PROGRAM_LOAD  0x02    /* Load data into cache buffer */
#define NAND_CMD_PROGRAM_EXEC  0x10    /* Execute programming from cache to array */
#define NAND_CMD_BLOCK_ERASE   0xD8    /* Block erase (128 KB) */
#define NAND_CMD_GET_FEATURE   0x0F
#define NAND_CMD_SET_FEATURE   0x1F

/* Winbond W25N01GV JEDEC Identification */
#define JEDEC_MFR_WINBOND      0xEF
#define JEDEC_DEV_W25N01GV     0xAA21

struct SprdSfcState {
    /*< private >*/
    SysBusDevice parent_obj;

    /*< public >*/
    MemoryRegion iomem;
    MemoryRegion direct_mem;
    qemu_irq irq;

    /* Registers */
    uint32_t cmd;
    uint32_t addr;
    uint32_t ctl;
    uint32_t sts;
    uint32_t buf_ptr;
    uint32_t int_en;

    /* SPI NAND Internal State */
    bool write_enabled;
    uint8_t status_reg1; /* Protection */
    uint8_t status_reg2; /* Configuration */
    uint8_t status_reg3; /* Status */

    /* 2112-byte Page Data Buffer (2048 Data + 64 Spare) */
    uint8_t page_buffer[NAND_PAGE_TOTAL_SIZE];

    /* In-memory Flash Storage Array (128 MB) */
    uint8_t *storage;
};

DeviceState *sprd_sfc_create(hwaddr mmio_addr, hwaddr direct_addr, qemu_irq irq);

#endif /* HW_BLOCK_SPRD_SFC_H */
