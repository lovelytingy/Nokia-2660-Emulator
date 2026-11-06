/*
 * Unisoc T107 (UMS9117) BootROM and Multi-Stage Bootloader Driver
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "bootrom.h"
#include "kpd.h"
#include "sprd_uart.h"

#define SPRD_GPIO_BASE 0x50040000
#define REG32(addr) (*(volatile uint32_t *)(addr))

void bootrom_init(void)
{
    /* Initialize UART0 for BootROM serial console messages */
    uart_init();
}

boot_mode_t bootrom_detect_mode(void)
{
    /* 1. Check Keypad Matrix STATUS0 for '*' key (KEY_STAR) */
    if (kpd_is_key_pressed(KEY_STAR)) {
        return BOOT_MODE_BSL_DOWNLOAD;
    }

    /* 2. Check GPIO Bank 0 Pin 1 (Strap / USB VBUS boot flag) */
    uint32_t gpio_data = REG32(SPRD_GPIO_BASE);
    if (gpio_data & 0x00000002) {
        return BOOT_MODE_BSL_DOWNLOAD;
    }

    return BOOT_MODE_NORMAL_NAND;
}

uint32_t bootrom_calc_checksum(const uint8_t *data, size_t size)
{
    uint32_t csum = 0;
    size_t full_words = size / 4;
    const uint32_t *words = (const uint32_t *)data;

    for (size_t i = 0; i < full_words; i++) {
        csum += words[i];
    }

    for (size_t i = full_words * 4; i < size; i++) {
        csum += data[i];
    }

    return csum;
}

int bootrom_verify_header(const sprd_boot0_hdr_t *hdr)
{
    if (!hdr) {
        return -1;
    }

    /* Verify Magic */
    if (hdr->magic != SPRD_BOOT0_MAGIC && hdr->magic != SPRD_BOOT0_MAGIC_BIMG) {
        return -2;
    }

    /* Verify Load Address (Must be in SRAM 0x00000000 - 0x0000FFFF) */
    if (hdr->load_addr > 0x0000E000) {
        return -3;
    }

    /* Verify Image Size (Must be positive and fit below stack) */
    if (hdr->image_size == 0 || hdr->image_size > T107_BOOT0_MAX_SIZE) {
        return -4;
    }

    return 0;
}

int bootrom_load_boot0(uintptr_t flash_addr, uint8_t *sram_dest, sprd_boot0_hdr_t *out_hdr)
{
    const sprd_boot0_hdr_t *hdr = (const sprd_boot0_hdr_t *)flash_addr;

    int res = bootrom_verify_header(hdr);
    if (res != 0) {
        return res;
    }

    if (out_hdr) {
        *out_hdr = *hdr;
    }

    const uint8_t *payload_src = (const uint8_t *)(flash_addr + sizeof(sprd_boot0_hdr_t));
    uint8_t *dest = sram_dest ? sram_dest : (uint8_t *)hdr->load_addr;

    /* Verify checksum before staging */
    uint32_t calculated_csum = bootrom_calc_checksum(payload_src, hdr->image_size);
    if (calculated_csum != hdr->checksum) {
        return -5; /* Checksum mismatch */
    }

    /* Stage payload into target SRAM */
    for (uint32_t i = 0; i < hdr->image_size; i++) {
        dest[i] = payload_src[i];
    }

    return 0;
}

boot_mode_t bootrom_dispatch(uint8_t *sram_dest, sprd_boot0_hdr_t *out_hdr)
{
    boot_mode_t mode = bootrom_detect_mode();

    if (mode == BOOT_MODE_BSL_DOWNLOAD) {
        uart_puts("[BootROM] Download mode requested ('*' key / boot strap active)!\n");
        uart_puts("[BootROM] Entering Spreadtrum BSL download protocol on UART0...\n");
        return BOOT_MODE_BSL_DOWNLOAD;
    }

    uart_puts("[BootROM] Normal cold boot mode. Probing SPI NAND Flash Block 0...\n");
    int res = bootrom_load_boot0(T107_NAND_FLASH_BASE, sram_dest, out_hdr);

    if (res == 0) {
        uart_puts("[BootROM] SPRD BOOT0 header validated! Loaded payload to SRAM at ");
        uart_puthex32(out_hdr ? out_hdr->load_addr : T107_BOOT0_LOAD_ADDR);
        uart_puts("\n[BootROM] Branching to BOOT0 entry point...\n");
        return BOOT_MODE_NORMAL_NAND;
    } else {
        uart_puts("[BootROM] Warning: No valid BOOT0 in SPI NAND Block 0 (error ");
        uart_putdec((uint32_t)(-res));
        uart_puts("). Falling back to BSL recovery mode...\n");
        return BOOT_MODE_RECOVERY;
    }
}
