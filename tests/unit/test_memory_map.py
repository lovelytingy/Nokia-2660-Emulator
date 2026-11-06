#!/usr/bin/env python3
"""
Unit tests for N2660Emu memory map, peripheral register definitions,
and baremetal test execution.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import unittest
import os
import sys
import subprocess

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

# Add site packages for unicorn/capstone
for pyver in ["3.9", "3.10", "3.11", "3.12", "3.13", "3.14", "3.15"]:
    p = os.path.expanduser(f"~/Library/Python/{pyver}/lib/python/site-packages")
    if os.path.isdir(p) and p not in sys.path:
        sys.path.append(p)

from tools.unicorn_runner import (
    SRAM_BASE, SRAM_SIZE, DRAM_BASE, DRAM_SIZE,
    SPRD_UART0_BASE, SPRD_TXD_REG, SPRD_STS1_REG,
    SPRD_TIMERS_BASE, TIMER_LOAD_LO, TIMER_VALUE_LO, TIMER_CTL,
    SPRD_GPIO_BASE, GPIO_DATA, GPIO_DIR,
    SPRD_SPI0_BASE, GC9307_WIDTH, GC9307_HEIGHT,
    T107UnicornVM
)

class TestN2660EmuMemoryMap(unittest.TestCase):
    def test_sram_boundaries(self):
        self.assertEqual(SRAM_BASE, 0x00000000)
        self.assertEqual(SRAM_SIZE, 64 * 1024)
        self.assertEqual(SRAM_BASE + SRAM_SIZE, 0x00010000)

    def test_dram_configuration(self):
        self.assertEqual(DRAM_BASE, 0x80000000)
        # 48 MB RAM configuration for Nokia 2660 Flip TA-1480
        self.assertEqual(DRAM_SIZE, 48 * 1024 * 1024)
        self.assertEqual(DRAM_BASE + DRAM_SIZE, 0x83000000)

    def test_sprd_uart_registers(self):
        self.assertEqual(SPRD_UART0_BASE, 0x50000000)
        self.assertEqual(SPRD_TXD_REG, 0x0000)
        self.assertEqual(SPRD_STS1_REG, 0x000C)

    def test_sprd_timer_registers(self):
        self.assertEqual(SPRD_TIMERS_BASE, 0x50030000)
        self.assertEqual(TIMER_LOAD_LO, 0x0000)
        self.assertEqual(TIMER_VALUE_LO, 0x0008)
        self.assertEqual(TIMER_CTL, 0x0010)

    def test_sprd_gpio_registers(self):
        self.assertEqual(SPRD_GPIO_BASE, 0x50040000)
        self.assertEqual(GPIO_DATA, 0x0000)
        self.assertEqual(GPIO_DIR, 0x0008)

    def test_sprd_spi_and_gc9307_display(self):
        self.assertEqual(SPRD_SPI0_BASE, 0x50080000)
        self.assertEqual(GC9307_WIDTH, 240)
        self.assertEqual(GC9307_HEIGHT, 320)

        bin_path = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        vm = T107UnicornVM(trace_instructions=False)
        vm.load_binary(bin_path, DRAM_BASE)
        success = vm.run(entry_point=DRAM_BASE, timeout_sec=5)
        self.assertTrue(success)

        # Verify GC9307 color bar pixel values
        self.assertEqual(vm.framebuffer[50][15], 0xFFFF)   # White
        self.assertEqual(vm.framebuffer[50][45], 0xFFE0)   # Yellow
        self.assertEqual(vm.framebuffer[50][75], 0x07FF)   # Cyan
        self.assertEqual(vm.framebuffer[50][105], 0x07E0)  # Green
        self.assertEqual(vm.framebuffer[50][135], 0xF81F)  # Magenta
        self.assertEqual(vm.framebuffer[50][165], 0xF800)  # Red
        self.assertEqual(vm.framebuffer[50][195], 0x001F)  # Blue
        self.assertEqual(vm.framebuffer[50][225], 0x0000)  # Black
        self.assertEqual(vm.framebuffer[260][120], 0x0215) # Nokia Blue footer

    def test_baremetal_payload_execution(self):
        bin_path = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        self.assertTrue(os.path.isfile(bin_path), f"Baremetal binary not found at {bin_path}")

        vm = T107UnicornVM(trace_instructions=False)
        vm.load_binary(bin_path, DRAM_BASE)
        success = vm.run(entry_point=DRAM_BASE, timeout_sec=5)
        self.assertTrue(success, "Baremetal test firmware failed during execution")

if __name__ == "__main__":
    unittest.main()
