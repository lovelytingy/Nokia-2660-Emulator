"""
Unit tests for Unisoc T107 SPRD Interrupt Controller (INTC),
software interrupts, and hardware IRQ exception handling.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import unittest
import os
import sys
import struct

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from tools.unicorn_runner import (
    T107UnicornVM, SPRD_INTC_BASE, INTC_IRQ_RAW_STS, INTC_IRQ_MASK_STS,
    INTC_IRQ_EN, INTC_IRQ_EN_CLR, INTC_IRQ_SOFT, INTC_IRQ_SOFT_CLR,
    INTC_IRQ_CLR, INTC_FIQ_RAW_STS, INTC_FIQ_MASK_STS, INTC_FIQ_EN,
    INTC_FIQ_EN_CLR, INTC_TYPE, SPRD_IRQ_TIMER, SPRD_IRQ_KPD
)

class TestSprdIntc(unittest.TestCase):

    def setUp(self):
        self.vm = T107UnicornVM(trace_instructions=False)

    def _read_u32(self, addr):
        self.vm._hook_mem_read(self.vm.uc, 0, addr, 4, 0, None)
        return struct.unpack("<I", self.vm.uc.mem_read(addr, 4))[0]

    def _write_u32(self, addr, val):
        self.vm._hook_mem_write(self.vm.uc, 0, addr, 4, val, None)

    def test_intc_reset_state(self):
        """Verify initial reset values of INTC registers."""
        raw = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_RAW_STS)
        mask = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_MASK_STS)
        en = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_EN)
        soft = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_SOFT)
        fiq_mask = self._read_u32(SPRD_INTC_BASE + INTC_FIQ_MASK_STS)

        self.assertEqual(raw, 0)
        self.assertEqual(mask, 0)
        self.assertEqual(en, 0)
        self.assertEqual(soft, 0)
        self.assertEqual(fiq_mask, 0)

    def test_intc_enable_and_clear(self):
        """Verify INTC_IRQ_EN and INTC_IRQ_EN_CLR behavior."""
        # Enable IRQ 0, 5, 15
        self._write_u32(SPRD_INTC_BASE + INTC_IRQ_EN, (1 << 0) | (1 << 5) | (1 << 15))
        en = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_EN)
        self.assertEqual(en, (1 << 0) | (1 << 5) | (1 << 15))

        # Clear IRQ 5
        self._write_u32(SPRD_INTC_BASE + INTC_IRQ_EN_CLR, (1 << 5))
        en = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_EN)
        self.assertEqual(en, (1 << 0) | (1 << 15))

    def test_intc_software_interrupt_and_masking(self):
        """Verify software interrupt triggering, status masking, and clearing."""
        # Enable IRQ 15 only
        self._write_u32(SPRD_INTC_BASE + INTC_IRQ_EN, (1 << 15))

        # Trigger soft IRQ 10 and 15
        self._write_u32(SPRD_INTC_BASE + INTC_IRQ_SOFT, (1 << 10) | (1 << 15))

        # Raw status reflects all active soft/hardware lines
        raw = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_RAW_STS)
        self.assertEqual(raw & (1 << 10), (1 << 10))
        self.assertEqual(raw & (1 << 15), (1 << 15))

        # Masked status reflects only enabled lines
        mask = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_MASK_STS)
        self.assertEqual(mask, (1 << 15))

        # Clear soft IRQ 15
        self._write_u32(SPRD_INTC_BASE + INTC_IRQ_SOFT_CLR, (1 << 15))
        mask = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_MASK_STS)
        self.assertEqual(mask, 0)

    def test_intc_fiq_type_routing(self):
        """Verify routing interrupt lines to FIQ via INTC_TYPE."""
        # Route IRQ 1 to FIQ (bit 1 = 1)
        self._write_u32(SPRD_INTC_BASE + INTC_TYPE, (1 << 1))
        # Enable both IRQ and FIQ for bit 1
        self._write_u32(SPRD_INTC_BASE + INTC_IRQ_EN, (1 << 1))
        self._write_u32(SPRD_INTC_BASE + INTC_FIQ_EN, (1 << 1))

        # Assert hardware interrupt on line 1
        self.vm.intc_assert_irq(1)

        # Line 1 is routed to FIQ, so IRQ masked status must be 0
        irq_mask = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_MASK_STS)
        self.assertEqual(irq_mask & (1 << 1), 0)

        # FIQ masked status should reflect line 1
        fiq_mask = self._read_u32(SPRD_INTC_BASE + INTC_FIQ_MASK_STS)
        self.assertEqual(fiq_mask & (1 << 1), (1 << 1))

    def test_intc_peripheral_assertion(self):
        """Verify asserting and deasserting peripheral IRQs."""
        self._write_u32(SPRD_INTC_BASE + INTC_IRQ_EN, (1 << SPRD_IRQ_TIMER) | (1 << SPRD_IRQ_KPD))

        self.vm.intc_assert_irq(SPRD_IRQ_TIMER)
        mask = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_MASK_STS)
        self.assertEqual(mask, (1 << SPRD_IRQ_TIMER))

        self.vm.intc_assert_irq(SPRD_IRQ_KPD)
        mask = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_MASK_STS)
        self.assertEqual(mask, (1 << SPRD_IRQ_TIMER) | (1 << SPRD_IRQ_KPD))

        self.vm.intc_deassert_irq(SPRD_IRQ_TIMER)
        mask = self._read_u32(SPRD_INTC_BASE + INTC_IRQ_MASK_STS)
        self.assertEqual(mask, (1 << SPRD_IRQ_KPD))

    def test_full_baremetal_intc_execution(self):
        """Execute full baremetal firmware test payload verifying Test 14 (INTC)."""
        fw_path = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        self.assertTrue(os.path.isfile(fw_path))

        vm = T107UnicornVM(trace_instructions=False)
        vm.load_binary(fw_path, 0x80000000)
        success = vm.run(entry_point=0x80000000, timeout_sec=5)
        self.assertTrue(success)

        tx_output = "".join(chr(c) for c in vm.uart0_tx_buffer)
        self.assertIn("[INTC] Software IRQ 15 successfully executed (Counter: 1)", tx_output)
        self.assertIn("[INTC] Keypad IRQ 5 successfully executed (Counter: 1)", tx_output)
        self.assertIn("[PASS] SPRD Interrupt Controller, software IRQs & hardware IRQ vectors verified.", tx_output)
        self.assertIn("*** ALL PHASE 1, 2, 3 & 4 (DISPLAY, INPUT, STORAGE, AUDIO, BOOTROM, ATAGS & INTC) CHECKS PASSED ***", tx_output)

if __name__ == "__main__":
    unittest.main()
