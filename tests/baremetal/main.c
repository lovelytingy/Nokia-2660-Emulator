/*
 * Baremetal test firmware for N2660Emu Phase 1 Milestone
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include <stdint.h>
#include <stddef.h>
#include "sprd_uart.h"

static inline uint32_t read_cp15_midr(void)
{
    uint32_t val;
    __asm__ volatile("mrc p15, 0, %0, c0, c0, 0" : "=r"(val));
    return val;
}

static inline uint32_t read_cpsr(void)
{
    uint32_t val;
    __asm__ volatile("mrs %0, cpsr" : "=r"(val));
    return val;
}

static int test_memory_region(const char *name, uintptr_t base_addr, size_t word_count)
{
    volatile uint32_t *ptr = (volatile uint32_t *)base_addr;
    uart_puts("[TEST] Testing ");
    uart_puts(name);
    uart_puts(" at ");
    uart_puthex32((uint32_t)base_addr);
    uart_puts(" (");
    uart_putdec((uint32_t)(word_count * 4));
    uart_puts(" bytes)...\n");

    /* Pattern 1: Incremental address pattern */
    for (size_t i = 0; i < word_count; i++) {
        ptr[i] = (uint32_t)(base_addr + (i * 4));
    }
    for (size_t i = 0; i < word_count; i++) {
        uint32_t expected = (uint32_t)(base_addr + (i * 4));
        uint32_t actual = ptr[i];
        if (actual != expected) {
            uart_puts("[FAIL] Mismatch at offset ");
            uart_puthex32((uint32_t)(i * 4));
            uart_puts(": expected ");
            uart_puthex32(expected);
            uart_puts(", got ");
            uart_puthex32(actual);
            uart_puts("\n");
            return 0;
        }
    }

    /* Pattern 2: Walking bit / Alternating pattern */
    for (size_t i = 0; i < word_count; i++) {
        ptr[i] = (i % 2 == 0) ? 0xAA55AA55 : 0x55AA55AA;
    }
    for (size_t i = 0; i < word_count; i++) {
        uint32_t expected = (i % 2 == 0) ? 0xAA55AA55 : 0x55AA55AA;
        uint32_t actual = ptr[i];
        if (actual != expected) {
            uart_puts("[FAIL] Pattern mismatch at offset ");
            uart_puthex32((uint32_t)(i * 4));
            uart_puts("\n");
            return 0;
        }
    }

    uart_puts("[PASS] ");
    uart_puts(name);
    uart_puts(" verified successfully.\n");
    return 1;
}

int main(void)
{
    uart_init();

    uart_puts("\n");
    uart_puts("========================================================\n");
    uart_puts("   Nokia 2660 Flip Emulator (N2660Emu) - Test Firmware  \n");
    uart_puts("   Target: Model TA-1480 | Unisoc T107 / UMS9117        \n");
    uart_puts("========================================================\n\n");

    /* 1. CPU Identification */
    uint32_t midr = read_cp15_midr();
    uint32_t cpsr = read_cpsr();

    uart_puts("[CPU] MIDR: ");
    uart_puthex32(midr);
    uart_puts(" | CPSR: ");
    uart_puthex32(cpsr);
    uart_puts("\n");

    uint32_t implementer = (midr >> 24) & 0xFF;
    uint32_t part_no = (midr >> 4) & 0xFFF;
    uint32_t arch = (midr >> 16) & 0xF;

    if (implementer == 0x41) {
        uart_puts("[CPU] Implementer: ARM Ltd (0x41)\n");
    }
    if (arch == 0xF) {
        uart_puts("[CPU] Architecture: ARMv7-A\n");
    }
    if (part_no == 0xC07) {
        uart_puts("[CPU] Core Model: ARM Cortex-A7 MPCore (0xC07)\n");
    } else {
        uart_puts("[CPU] Core Model Part: ");
        uart_puthex32(part_no);
        uart_puts("\n");
    }

    /* 2. Internal SRAM Test (AP On-Chip RAM at 0x00000000) */
    /* Test 256 words (1 KB) at offset 0x1000 in SRAM */
    int sram_ok = test_memory_region("Internal SRAM (IRAM)", 0x00001000, 256);

    /* 3. High DRAM Test (LPDDR2 at 0x80000000) */
    /* Test 256 words (1 KB) at offset 0x80020000 */
    int dram_ok = test_memory_region("High DRAM (LPDDR2)", 0x80020000, 256);

    /* 4. MMIO Trap Test (Read from unmapped peripheral space) */
    uart_puts("[TEST] Triggering MMIO read at unmapped address 0x40001000...\n");
    volatile uint32_t *unmapped_reg = (volatile uint32_t *)0x40001000;
    uint32_t dummy_val = *unmapped_reg;
    uart_puts("[INFO] Unmapped MMIO read returned: ");
    uart_puthex32(dummy_val);
    uart_puts("\n");

    /* 5. Summary */
    uart_puts("\n--------------------------------------------------------\n");
    if (sram_ok && dram_ok) {
        uart_puts("[RESULT] *** ALL PHASE 1 MILESTONE CHECKS PASSED ***\n");
        uart_puts("[RESULT] CPU, Internal SRAM, DRAM, and Virtual UART OK!\n");
    } else {
        uart_puts("[RESULT] *** SOME TESTS FAILED ***\n");
    }
    uart_puts("--------------------------------------------------------\n\n");

    return 0;
}
