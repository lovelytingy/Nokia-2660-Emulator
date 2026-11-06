/*
 * Unisoc T107 (UMS9117) BootROM and Multi-Stage Bootloader Driver
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef BOOTROM_H
#define BOOTROM_H

#include <stdint.h>
#include <stddef.h>

#define SPRD_BOOT0_MAGIC        0x53505244  /* "SPRD" */
#define SPRD_BOOT0_MAGIC_BIMG   0x42494D47  /* "BIMG" */
#define SPRD_BOOT0_VERSION      1

#define T107_BOOT0_LOAD_ADDR    0x00006200  /* Default SRAM staging address */
#define T107_BOOT0_MAX_SIZE     (36 * 1024) /* 36 KB max payload size in SRAM */
#define T107_NAND_FLASH_BASE    0x60000000  /* Memory-mapped SPI NAND window */
#define T107_NAND_BLOCK0_OFFSET 0x00000000

/* SPRD BOOT0 Header (32 bytes) */
typedef struct __attribute__((packed)) {
    uint32_t magic;         /* "SPRD" (0x53505244) or "BIMG" */
    uint32_t version;       /* Header version (1) */
    uint32_t load_addr;     /* Target SRAM address (0x00006200) */
    uint32_t image_size;    /* Size of executable payload in bytes */
    uint32_t entry_point;   /* Entry point address (0x00006200) */
    uint32_t checksum;      /* 32-bit addition sum of payload bytes */
    uint32_t flags;         /* Boot attributes */
    uint32_t reserved;      /* Reserved padding */
} sprd_boot0_hdr_t;

typedef enum {
    BOOT_MODE_NORMAL_NAND = 0,
    BOOT_MODE_BSL_DOWNLOAD = 1,
    BOOT_MODE_RECOVERY     = 2
} boot_mode_t;

/* Initialize early BootROM environment */
void bootrom_init(void);

/* Detect boot mode from hardware straps (Keypad '*' key and GPIO Bank 0 Pin 1) */
boot_mode_t bootrom_detect_mode(void);

/* Calculate 32-bit additive checksum of payload */
uint32_t bootrom_calc_checksum(const uint8_t *data, size_t size);

/* Verify SPRD BOOT0 header validity */
int bootrom_verify_header(const sprd_boot0_hdr_t *hdr);

/* Load and verify BOOT0 from SPI NAND flash into SRAM */
int bootrom_load_boot0(uintptr_t flash_addr, uint8_t *sram_dest, sprd_boot0_hdr_t *out_hdr);

/* Orchestrate BootROM dispatch flow */
boot_mode_t bootrom_dispatch(uint8_t *sram_dest, sprd_boot0_hdr_t *out_hdr);

#endif /* BOOTROM_H */
