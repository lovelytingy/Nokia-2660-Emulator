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
    SPRD_KPD_BASE, KPD_CTRL, KPD_INT_EN, KPD_INT_RAW, KPD_STATUS0, KPD_STATUS1,
    SPRD_SPI0_BASE, GC9307_WIDTH, GC9307_HEIGHT,
    SPRD_SPI1_BASE, ST7735_WIDTH, ST7735_HEIGHT,
    SPRD_SFC_BASE, FLASH_DIRECT_BASE, FLASH_TOTAL_SIZE, NAND_PAGE_SIZE, NAND_BLOCK_SIZE,
    SPRD_AUDIO_BASE, AUD_CTL, AUD_STS, AUD_TXD, AUD_DAC_CTL, AUD_PATH_CTL,
    TONE_CTL, TONE_FREQ, TONE_DUR, TONE_VOL,
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

    def test_sprd_kpd_registers(self):
        self.assertEqual(SPRD_KPD_BASE, 0x50060000)
        self.assertEqual(KPD_CTRL, 0x0000)
        self.assertEqual(KPD_INT_EN, 0x0004)
        self.assertEqual(KPD_INT_RAW, 0x0008)
        self.assertEqual(KPD_STATUS0, 0x001C)
        self.assertEqual(KPD_STATUS1, 0x0020)

    def test_keypad_injection_and_flip_sensor(self):
        vm = T107UnicornVM(trace_instructions=False)
        # Flip sensor verification
        self.assertEqual(vm.gpio_data_in & 0x1, 1, "Default flip state should be OPEN")
        vm.set_flip_state(False)
        self.assertEqual(vm.gpio_data_in & 0x1, 0, "Flip state should be CLOSED")
        vm.set_flip_state(True)
        self.assertEqual(vm.gpio_data_in & 0x1, 1, "Flip state should be OPEN")

        # Keypad matrix injection verification
        self.assertEqual(vm.kpd_status0, 0)
        self.assertEqual(vm.kpd_status1, 0)

        # Press key '5' (key code 5 -> bit 5 of STATUS0)
        vm.inject_key('5', pressed=True)
        self.assertEqual(vm.kpd_status0, (1 << 5))
        self.assertEqual(vm.kpd_int_raw & 0x1, 1)

        # Release key '5'
        vm.inject_key('5', pressed=False)
        self.assertEqual(vm.kpd_status0, 0)

        # Press navigation key 'ok' (key code 16 -> bit 0 of STATUS1)
        vm.inject_key('ok', pressed=True)
        self.assertEqual(vm.kpd_status1, (1 << 0))

        # Press emergency key 'sos' (key code 21 -> bit 5 of STATUS1)
        vm.inject_key('sos', pressed=True)
        self.assertEqual(vm.kpd_status1, (1 << 0) | (1 << 5))

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

    def test_sprd_spi1_and_st7735_sub_display(self):
        self.assertEqual(SPRD_SPI1_BASE, 0x50090000)
        self.assertEqual(ST7735_WIDTH, 120)
        self.assertEqual(ST7735_HEIGHT, 160)

        bin_path = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        vm = T107UnicornVM(trace_instructions=False)
        vm.load_binary(bin_path, DRAM_BASE)
        success = vm.run(entry_point=DRAM_BASE, timeout_sec=5)
        self.assertTrue(success)

        # Verify ST7735 outer screen pixel values
        self.assertEqual(vm.sub_framebuffer[10][60], 0x001F)   # Top status bar (Blue)
        self.assertEqual(vm.sub_framebuffer[10][12], 0x07E0)   # Signal indicator (Green)
        self.assertEqual(vm.sub_framebuffer[10][105], 0xFFE0)  # Battery indicator (Yellow)
        self.assertEqual(vm.sub_framebuffer[70][60], 0x07FF)   # Digital clock box (Cyan)
        self.assertEqual(vm.sub_framebuffer[120][60], 0x0215)  # Notification banner (Nokia Blue)
        self.assertEqual(vm.sub_framebuffer[158][60], 0xFFFF)  # Bottom accent bar (White)

    def test_sprd_sfc_and_spi_nand_flash(self):
        self.assertEqual(SPRD_SFC_BASE, 0x500A0000)
        self.assertEqual(FLASH_DIRECT_BASE, 0x60000000)
        self.assertEqual(FLASH_TOTAL_SIZE, 128 * 1024 * 1024)
        self.assertEqual(NAND_PAGE_SIZE, 2048)
        self.assertEqual(NAND_BLOCK_SIZE, 128 * 1024)

        bin_path = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        vm = T107UnicornVM(trace_instructions=False)
        vm.load_binary(bin_path, DRAM_BASE)
        success = vm.run(entry_point=DRAM_BASE, timeout_sec=5)
        self.assertTrue(success)

        # Verify programmed page 128 (offset 128 * 2048 = 0x40000)
        flash_offset = 128 * 2048
        val0 = (vm.flash_storage[flash_offset] |
                (vm.flash_storage[flash_offset + 1] << 8) |
                (vm.flash_storage[flash_offset + 2] << 16) |
                (vm.flash_storage[flash_offset + 3] << 24))
        self.assertEqual(val0, 0xAA000000)

    def test_sprd_audio_subsystem(self):
        self.assertEqual(SPRD_AUDIO_BASE, 0x500C0000)
        self.assertEqual(AUD_CTL, 0x0000)
        self.assertEqual(AUD_STS, 0x0004)
        self.assertEqual(AUD_TXD, 0x0008)
        self.assertEqual(TONE_CTL, 0x0030)

        bin_path = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        vm = T107UnicornVM(trace_instructions=False)
        vm.load_binary(bin_path, DRAM_BASE)
        success = vm.run(entry_point=DRAM_BASE, timeout_sec=5)
        self.assertTrue(success)

        # Verify that 16 PCM stereo samples were captured
        self.assertGreaterEqual(len(vm.audio_tx_samples), 16)
        # Verify that the 1000 Hz buzzer tone event was recorded
        self.assertGreaterEqual(len(vm.audio_tone_events), 1)
        tone = vm.audio_tone_events[0]
        self.assertEqual(tone["freq"], 1000)
        self.assertEqual(tone["dur"], 50)
        self.assertEqual(tone["vol"], 85)

    def test_baremetal_payload_execution(self):
        bin_path = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        self.assertTrue(os.path.isfile(bin_path), f"Baremetal binary not found at {bin_path}")

        vm = T107UnicornVM(trace_instructions=False)
        vm.load_binary(bin_path, DRAM_BASE)
        success = vm.run(entry_point=DRAM_BASE, timeout_sec=5)
        self.assertTrue(success, "Baremetal test firmware failed during execution")

if __name__ == "__main__":
    unittest.main()
