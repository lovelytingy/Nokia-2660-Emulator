"""
Unit tests for Nokia 2660 Flip Interactive GUI & Keypad Runner.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import unittest
import os
import sys
import time
import re

for pyver in ["3.9", "3.10", "3.11", "3.12", "3.13", "3.14", "3.15"]:
    p = os.path.expanduser(f"~/Library/Python/{pyver}/lib/python/site-packages")
    if os.path.isdir(p) and p not in sys.path:
        sys.path.append(p)

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from tools.gui_runner import (
    N2660GuiRunner, rgb565_to_rgb888, render_halfblock_screen,
    GC9307_WIDTH, GC9307_HEIGHT, ST7735_WIDTH, ST7735_HEIGHT
)
from tools.unicorn_runner import T107UnicornVM, NOKIA_KEYS

class TestGuiRunner(unittest.TestCase):

    def test_rgb565_color_conversion(self):
        """Test RGB565 to RGB888 color conversion math."""
        # Black
        self.assertEqual(rgb565_to_rgb888(0x0000), (0, 0, 0))
        # White
        self.assertEqual(rgb565_to_rgb888(0xFFFF), (255, 255, 255))
        # Pure Red (11111 000000 00000 = 0xF800)
        self.assertEqual(rgb565_to_rgb888(0xF800), (255, 0, 0))
        # Pure Green (00000 111111 00000 = 0x07E0)
        self.assertEqual(rgb565_to_rgb888(0x07E0), (0, 255, 0))
        # Pure Blue (00000 000000 11111 = 0x001F)
        self.assertEqual(rgb565_to_rgb888(0x001F), (0, 0, 255))

    def test_halfblock_screen_downsampler(self):
        """Test downsampling 2D RGB565 buffer into ANSI halfblock strings."""
        w, h = 240, 320
        fb = [[0x0000 for _ in range(w)] for _ in range(h)]
        # Top half red, bottom half blue
        for y in range(160):
            for x in range(w):
                fb[y][x] = 0xF800
        for y in range(160, h):
            for x in range(w):
                fb[y][x] = 0x001F

        lines = render_halfblock_screen(fb, w, h, out_cols=20, out_rows=10)
        self.assertEqual(len(lines), 10)
        # Each line must contain Unicode halfblock ▀
        for l in lines:
            self.assertIn("▀", l)

    def test_key_bindings_validity(self):
        """Verify that every keyboard shortcut in KEY_BINDINGS maps to valid NOKIA_KEYS."""
        for char_key, nokia_key in N2660GuiRunner.KEY_BINDINGS.items():
            self.assertIn(nokia_key, NOKIA_KEYS, f"Key binding '{char_key}' -> '{nokia_key}' not in NOKIA_KEYS")

    def test_key_injection_and_auto_release(self):
        """Test injecting a key and auto-releasing it after time elapses."""
        vm = T107UnicornVM(trace_instructions=False)
        runner = N2660GuiRunner(vm=vm)

        self.assertEqual(vm.kpd_status0, 0)
        self.assertEqual(vm.kpd_status1, 0)

        # Inject numeric key '5' (bit 5 in status0)
        runner.handle_key_event('5', duration_sec=0.05)
        self.assertTrue(vm.kpd_status0 & (1 << 5))
        self.assertIn('5', runner.active_keys)

        # Before duration elapsed
        runner.process_key_releases()
        self.assertTrue(vm.kpd_status0 & (1 << 5))

        # After duration elapsed
        time.sleep(0.06)
        runner.process_key_releases()
        self.assertFalse(vm.kpd_status0 & (1 << 5))
        self.assertNotIn('5', runner.active_keys)

    def test_flip_sensor_toggle(self):
        """Test toggling the Hall effect flip sensor and corresponding view switch."""
        vm = T107UnicornVM(trace_instructions=False)
        runner = N2660GuiRunner(vm=vm)

        # Initial state: Open (GPIO 0 Pin 0 = 1)
        self.assertTrue(bool(vm.gpio_data_in & 0x1))

        # Toggle to Closed
        runner.toggle_flip()
        self.assertFalse(bool(vm.gpio_data_in & 0x1))
        self.assertEqual(runner.screen_mode, "sub")

        # Toggle back to Open
        runner.toggle_flip()
        self.assertTrue(bool(vm.gpio_data_in & 0x1))
        self.assertEqual(runner.screen_mode, "main")

    def test_widget_builders_and_frame_render(self):
        """Test building keypad, telemetry, and complete screen frames."""
        vm = T107UnicornVM(trace_instructions=False)
        runner = N2660GuiRunner(vm=vm)

        keypad_lines = runner.build_keypad_widget()
        self.assertGreater(len(keypad_lines), 5)
        self.assertTrue(any("Keypad Matrix" in l for l in keypad_lines))
        self.assertTrue(any("LSK" in l for l in keypad_lines))
        self.assertTrue(any("CALL" in l for l in keypad_lines))

        telemetry_lines = runner.build_telemetry_widget()
        self.assertGreater(len(telemetry_lines), 5)
        self.assertTrue(any("Hardware Telemetry" in l for l in telemetry_lines))
        self.assertTrue(any("Cortex-A7" in l for l in telemetry_lines))

        # Test Main frame render
        runner.screen_mode = "main"
        frame_main = runner.render_frame()
        self.assertIn("Main Display", frame_main)

        # Test Sub frame render
        runner.screen_mode = "sub"
        frame_sub = runner.render_frame()
        self.assertIn("Outer Sub-Display", frame_sub)

        # Test Dual frame render
        runner.screen_mode = "dual"
        frame_dual = runner.render_frame()
        self.assertIn("Sub Display", frame_dual)

    def test_headless_execution_with_interactive_demo(self):
        """Test running interactive_demo.bin in headless mode with scripted events."""
        demo_bin = os.path.join(PROJECT_ROOT, "tests", "baremetal", "interactive_demo.bin")
        if not os.path.isfile(demo_bin):
            self.skipTest("interactive_demo.bin not compiled")

        vm = T107UnicornVM(trace_instructions=False)
        vm.load_binary(demo_bin, 0x80000000)
        runner = N2660GuiRunner(vm=vm, entry_point=0x80000000, slice_insns=15000)

        # Execute in headless mode for 0.4 seconds
        ret = runner.run_headless(duration_sec=0.4)
        self.assertTrue(ret)
        self.assertGreater(vm.instruction_count, 10000)

        # Check UART output from the interactive OS shell
        uart_bytes = bytes(vm.uart0_tx_buffer)
        uart_text = uart_bytes.decode('latin-1', errors='replace')
        self.assertIn("Nokia 2660 Flip (TA-1480) Interactive OS Shell", uart_text)
        self.assertIn("Peripherals, INTC and Keypad IRQs initialized", uart_text)

if __name__ == "__main__":
    unittest.main()
