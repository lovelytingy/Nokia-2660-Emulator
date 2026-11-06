"""
Unit tests for Unisoc T107 BootROM reset vector, SPRD BOOT0 packaging,
and multi-stage boot flow (BOOT0 vs BSL mode).

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import unittest
import os
import sys
import struct
import tempfile

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from tools.mksprd_img import (
    SPRD_BOOT0_MAGIC, SPRD_BOOT0_MAGIC_BIMG, calculate_checksum,
    create_boot0_image, parse_boot0_header, inject_boot0_into_flash,
    extract_boot0_from_flash
)
from tools.unicorn_runner import T107UnicornVM

class TestBootRomAndMultiStageBoot(unittest.TestCase):

    def setUp(self):
        self.test_payload = bytes([
            # ARM code: mov r0, #42; b .
            0x2A, 0x00, 0xA0, 0xE3,
            0xFE, 0xFF, 0xFF, 0xEA
        ])

    def test_mksprd_img_header_creation_and_verification(self):
        """Test SPRD BOOT0 image creation, checksum calculation and header validation."""
        image = create_boot0_image(self.test_payload, load_addr=0x00006200, entry_point=0x00006200)
        self.assertEqual(len(image), 32 + len(self.test_payload))

        info = parse_boot0_header(image)
        self.assertTrue(info["is_valid_magic"])
        self.assertEqual(info["magic"], SPRD_BOOT0_MAGIC)
        self.assertEqual(info["magic_str"], "SPRD")
        self.assertEqual(info["load_addr"], 0x00006200)
        self.assertEqual(info["entry_point"], 0x00006200)
        self.assertEqual(info["image_size"], len(self.test_payload))
        self.assertTrue(info["checksum_valid"])

        # Verify that modifying payload corrupts checksum check
        corrupted_image = bytearray(image)
        corrupted_image[32] ^= 0xFF
        corrupt_info = parse_boot0_header(bytes(corrupted_image))
        self.assertFalse(corrupt_info["checksum_valid"])

    def test_mksprd_img_flash_inject_and_extract(self):
        """Test injecting BOOT0 into an SPI NAND flash image and extracting it."""
        image = create_boot0_image(self.test_payload, load_addr=0x00006200, entry_point=0x00006200)

        with tempfile.NamedTemporaryFile(suffix=".bin", delete=False) as f_flash:
            flash_path = f_flash.name

        try:
            inject_boot0_into_flash(image, flash_path, block_index=0)
            self.assertTrue(os.path.exists(flash_path))
            self.assertGreaterEqual(os.path.getsize(flash_path), len(image))

            extracted = extract_boot0_from_flash(flash_path, block_index=0)
            self.assertEqual(extracted, image)
        finally:
            if os.path.exists(flash_path):
                os.remove(flash_path)

    def test_bootrom_normal_cold_boot_flow(self):
        """Test normal cold boot: BootROM reads BOOT0 from SPI NAND Block 0, stages to 0x6200, and runs."""
        vm = T107UnicornVM(trace_instructions=False)

        # Stage BOOT0 into SPI NAND Block 0
        with tempfile.NamedTemporaryFile(suffix=".bin", delete=False) as f_payload:
            f_payload.write(self.test_payload)
            payload_path = f_payload.name

        try:
            vm.load_boot0(payload_path, block_index=0, auto_wrap=True)
            vm.load_bootrom()

            # Execute starting at BootROM reset vector (PC = 0x00000000)
            success = vm.run(entry_point=0x00000000, timeout_sec=1)
            self.assertTrue(success)

            # Verify that payload was staged into SRAM at 0x00006200
            sram_staged = vm.uc.mem_read(0x00006200, len(self.test_payload))
            self.assertEqual(bytes(sram_staged), self.test_payload)
        finally:
            if os.path.exists(payload_path):
                os.remove(payload_path)

    def test_bootrom_download_mode_star_key(self):
        """Test BSL download mode: holding '*' key bypasses SPI NAND and jumps to FDL1 in SRAM."""
        vm = T107UnicornVM(trace_instructions=False)

        # Inject download key ('*' key = bit 10 of KPD_STATUS0)
        vm.inject_key("*", pressed=True)

        with tempfile.NamedTemporaryFile(suffix=".bin", delete=False) as f_payload:
            f_payload.write(self.test_payload)
            payload_path = f_payload.name

        try:
            # Stage FDL1 at 0x00006200
            vm.load_fdl1(payload_path, load_addr=0x00006200)
            vm.load_bootrom()

            # Execute starting at BootROM reset vector (PC = 0x00000000)
            success = vm.run(entry_point=0x00000000, timeout_sec=1)
            self.assertTrue(success)

            # Check that UART TX buffer contains BSL download mode string
            tx_output = "".join(chr(c) for c in vm.uart0_tx_buffer)
            self.assertIn("Entering Spreadtrum BSL download protocol on UART0", tx_output)
            self.assertIn("Branching to FDL1", tx_output)
            # Must NOT have probed SPI NAND
            self.assertNotIn("Probing SPI NAND Flash Block 0", tx_output)
        finally:
            if os.path.exists(payload_path):
                os.remove(payload_path)

    def test_bootrom_fallback_on_blank_nand(self):
        """Test fallback to BSL mode when SPI NAND flash is blank/unprogrammed."""
        vm = T107UnicornVM(trace_instructions=False)
        # Flash storage remains all 0xFF (blank)
        vm.load_bootrom()

        success = vm.run(entry_point=0x00000000, timeout_sec=1)
        self.assertTrue(success)

        tx_output = "".join(chr(c) for c in vm.uart0_tx_buffer)
        self.assertIn("Normal cold boot. Probing SPI NAND Flash Block 0", tx_output)
        self.assertIn("No valid BOOT0 in SPI NAND Block 0. Fallback to BSL mode", tx_output)

    def test_baremetal_test_firmware_test12(self):
        """Test full baremetal test payload execution verifying all 12 hardware & firmware checks."""
        fw_path = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        self.assertTrue(os.path.isfile(fw_path))

        vm = T107UnicornVM(trace_instructions=False)
        vm.load_binary(fw_path, 0x80000000)
        success = vm.run(entry_point=0x80000000, timeout_sec=5)
        self.assertTrue(success)

        tx_output = "".join(chr(c) for c in vm.uart0_tx_buffer)
        self.assertIn("[PASS] BootROM reset vector, SPRD header validation & BOOT0 staging verified.", tx_output)
        self.assertIn("[PASS] ARM Boot Tags (ATAG_CORE, MEM, CMDLINE, SERIAL) & kernel handoff verified.", tx_output)
        self.assertIn("*** ALL PHASE 1, 2, 3 & 4 (DISPLAY, INPUT, STORAGE, AUDIO, BOOTROM & ATAGS) CHECKS PASSED ***", tx_output)

if __name__ == "__main__":
    unittest.main()
