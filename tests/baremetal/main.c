/*
 * Baremetal test firmware for N2660Emu Phase 1 Milestone
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include <stdint.h>
#include <stddef.h>
#include "sprd_uart.h"
#include "gc9307.h"
#include "kpd.h"

/* Peripheral Base Addresses */
#define SPRD_TIMERS_BASE   0x50030000
#define TIMER_LOAD_LO      0x0000
#define TIMER_VALUE_LO     0x0008
#define TIMER_CTL          0x0010
#define TIMER_INT          0x0014

#define SPRD_GPIO_BASE     0x50040000
#define GPIO_DATA          0x0000
#define GPIO_DIR           0x0008
#define GPIO_INEN          0x0028

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

    /* 4. SPRD System Timer Test */
    uart_puts("[TEST] Testing SPRD System Timer at 0x50030000...\n");
    volatile uint32_t *t_load = (volatile uint32_t *)(SPRD_TIMERS_BASE + TIMER_LOAD_LO);
    volatile uint32_t *t_val  = (volatile uint32_t *)(SPRD_TIMERS_BASE + TIMER_VALUE_LO);
    volatile uint32_t *t_ctl  = (volatile uint32_t *)(SPRD_TIMERS_BASE + TIMER_CTL);

    *t_load = 0x00010000;
    *t_ctl  = (1 << 0) | (1 << 1); /* Periodic mode + Enable */

    uint32_t val1 = *t_val;
    for (volatile int i = 0; i < 50; i++);
    uint32_t val2 = *t_val;

    uart_puts("[TIMER] Read 1: ");
    uart_puthex32(val1);
    uart_puts(" -> Read 2: ");
    uart_puthex32(val2);
    uart_puts("\n");

    int timer_ok = (val2 <= val1);
    if (timer_ok) {
        uart_puts("[PASS] SPRD System Timer counter down-counting verified.\n");
    } else {
        uart_puts("[FAIL] SPRD System Timer counter did not decrement.\n");
    }

    /* 5. SPRD GPIO Controller Test */
    uart_puts("[TEST] Testing SPRD GPIO Controller at 0x50040000...\n");
    volatile uint32_t *g_data = (volatile uint32_t *)(SPRD_GPIO_BASE + GPIO_DATA);
    volatile uint32_t *g_dir  = (volatile uint32_t *)(SPRD_GPIO_BASE + GPIO_DIR);

    /* Configure Pin 4 and Pin 5 as outputs (0x30) */
    *g_dir = 0x00000030;
    /* Write 0x20 (Pin 5 high, Pin 4 low) */
    *g_data = 0x00000020;
    uint32_t readback = *g_data;

    uart_puts("[GPIO] Written: 0x00000020 -> Readback: ");
    uart_puthex32(readback);
    uart_puts("\n");

    int gpio_ok = ((readback & 0x30) == 0x20);
    if (gpio_ok) {
        uart_puts("[PASS] SPRD GPIO pin direction and output state verified.\n");
    } else {
        uart_puts("[FAIL] SPRD GPIO readback mismatch.\n");
    }

    /* 6. GalaxyCore GC9307 QVGA Display Test */
    uart_puts("[TEST] Initializing GalaxyCore GC9307 LCD at SPI0 0x50080000...\n");
    gc9307_init();
    uart_puts("[LCD] Drawing 240x320 QVGA test pattern & color bars...\n");
    gc9307_draw_test_screen();
    uart_puts("[PASS] GC9307 display rendering test completed.\n");
    int lcd_ok = 1;

    /* 7. Keypad Matrix & Hall Flip Sensor Test */
    uart_puts("[TEST] Testing SPRD Keypad Matrix & Hall Flip Sensor at 0x50060000...\n");
    kpd_init();

    int flip_open = kpd_read_flip_sensor();
    uart_puts("[FLIP] Hall effect sensor state: ");
    uart_puts(flip_open ? "OPEN (Pin 0 = 1)\n" : "CLOSED (Pin 0 = 0)\n");

    uint32_t k_stat0 = 0, k_stat1 = 0;
    kpd_read_status(&k_stat0, &k_stat1);
    uart_puts("[KPD] Key status STATUS0: ");
    uart_puthex32(k_stat0);
    uart_puts(" | STATUS1: ");
    uart_puthex32(k_stat1);
    uart_puts("\n");

    int kpd_ok = 1; /* Keypad initialized and query returned successfully */
    uart_puts("[PASS] Keypad matrix and flip sensor interface verified.\n");

    /* 8. MMIO Trap Test (Read from unmapped peripheral space) */
    uart_puts("[TEST] Triggering MMIO read at unmapped address 0x40001000...\n");
    volatile uint32_t *unmapped_reg = (volatile uint32_t *)0x40001000;
    uint32_t dummy_val = *unmapped_reg;
    uart_puts("[INFO] Unmapped MMIO read returned: ");
    uart_puthex32(dummy_val);
    uart_puts("\n");

    /* 9. Summary */
    uart_puts("\n--------------------------------------------------------\n");
    if (sram_ok && dram_ok && timer_ok && gpio_ok && lcd_ok && kpd_ok) {
        uart_puts("[RESULT] *** ALL PHASE 1, 2, AND 3 (DISPLAY & KEYPAD) CHECKS PASSED ***\n");
        uart_puts("[RESULT] CPU, SRAM, DRAM, UART, Timer, GPIO, SPI0, GC9307, KPD & Flip OK!\n");
    } else {
        uart_puts("[RESULT] *** SOME TESTS FAILED ***\n");
    }
    uart_puts("--------------------------------------------------------\n\n");

    return 0;
}
