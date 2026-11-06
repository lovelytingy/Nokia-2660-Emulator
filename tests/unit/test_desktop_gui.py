"""
Unit tests for Nokia 2660 Flip Host Desktop GUI & Vector Skin.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import unittest
import os
import sys
import struct

for pyver in ["3.9", "3.10", "3.11", "3.12", "3.13", "3.14", "3.15"]:
    p = os.path.expanduser(f"~/Library/Python/{pyver}/lib/python/site-packages")
    if os.path.isdir(p) and p not in sys.path:
        sys.path.append(p)

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

import cairo
from tools.desktop_gui import (
    Nokia2660PhoneSkin, Nokia2660EmulatorWindow,
    LUT_RGB565_TO_RGB24, THEMES
)
from tools.unicorn_runner import (
    T107UnicornVM, NOKIA_KEYS,
    GC9307_WIDTH, GC9307_HEIGHT,
    ST7735_WIDTH, ST7735_HEIGHT
)

class TestDesktopGui(unittest.TestCase):

    def test_lut_rgb565_to_rgb24_lookup(self):
        """Verify the 64K lookup table RGB565 to Cairo ARGB32 conversions."""
        self.assertEqual(len(LUT_RGB565_TO_RGB24), 65536)

        # Black (0x0000) -> B=0, G=0, R=0, A=0xFF
        self.assertEqual(LUT_RGB565_TO_RGB24[0x0000], struct.pack('<BBBB', 0, 0, 0, 0xFF))

        # White (0xFFFF) -> B=255, G=255, R=255, A=0xFF
        self.assertEqual(LUT_RGB565_TO_RGB24[0xFFFF], struct.pack('<BBBB', 255, 255, 255, 0xFF))

        # Pure Red (0xF800: R=31, G=0, B=0) -> Little endian: B=0, G=0, R=255, A=0xFF
        self.assertEqual(LUT_RGB565_TO_RGB24[0xF800], struct.pack('<BBBB', 0, 0, 255, 0xFF))

        # Pure Green (0x07E0: R=0, G=63, B=0) -> B=0, G=255, R=0, A=0xFF
        self.assertEqual(LUT_RGB565_TO_RGB24[0x07E0], struct.pack('<BBBB', 0, 255, 0, 0xFF))

        # Pure Blue (0x001F: R=0, G=0, B=31) -> B=255, G=0, R=0, A=0xFF
        self.assertEqual(LUT_RGB565_TO_RGB24[0x001F], struct.pack('<BBBB', 255, 0, 0, 0xFF))

    def test_phone_skin_themes(self):
        """Verify casing color themes and switching."""
        skin = Nokia2660PhoneSkin()
        self.assertIn("Deep Blue", THEMES)
        self.assertIn("Classic Black", THEMES)
        self.assertIn("Pop Red", THEMES)

        self.assertEqual(skin.theme_name, "Deep Blue")
        skin.set_theme("Pop Red")
        self.assertEqual(skin.theme_name, "Pop Red")
        self.assertEqual(skin.theme, THEMES["Pop Red"])

        # Invalid theme name should be safely ignored
        skin.set_theme("NonExistentTheme")
        self.assertEqual(skin.theme_name, "Pop Red")

    def test_keypad_button_matrix_validity(self):
        """Ensure all buttons defined on the physical skin map to recognized NOKIA_KEYS."""
        skin = Nokia2660PhoneSkin()
        button_ids = {btn["id"] for btn in skin.buttons}

        required_keys = {
            "lsk", "rsk", "call", "end", "up", "down", "left", "right", "ok",
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "*", "0", "#"
        }
        for k in required_keys:
            self.assertIn(k, button_ids, f"Required button '{k}' missing from physical phone skin")
            self.assertIn(k, NOKIA_KEYS, f"Skin button '{k}' is not defined in NOKIA_KEYS")

    def test_keypad_hit_testing(self):
        """Test mouse coordinate hit testing across keypad buttons and flip states."""
        skin = Nokia2660PhoneSkin()
        skin.flip_open = True

        # Center of button '5' rect is (150, 612, 80, 52) -> (190, 638)
        self.assertEqual(skin.hit_test(190, 638), "5")

        # Center of 'call' button rect is (55, 490, 60, 42) -> (85, 511)
        self.assertEqual(skin.hit_test(85, 511), "call")

        # Center of 'end' button rect is (265, 490, 60, 42) -> (295, 511)
        self.assertEqual(skin.hit_test(295, 511), "end")

        # Center of 'ok' button rect is (165, 471, 50, 35) -> (190, 488)
        self.assertEqual(skin.hit_test(190, 488), "ok")

        # Point outside any button (e.g. top screen bezel)
        self.assertIsNone(skin.hit_test(10, 10))
        self.assertIsNone(skin.hit_test(150, 200))

        # When flip is closed, hit testing must return None
        skin.flip_open = False
        self.assertIsNone(skin.hit_test(190, 638))
        self.assertIsNone(skin.hit_test(85, 511))

    def test_screen_surface_updates(self):
        """Test updating Cairo image surfaces from simulated RGB565 framebuffers."""
        skin = Nokia2660PhoneSkin()

        # Generate 240x320 main framebuffer
        main_fb = [[0x07E0 for _ in range(GC9307_WIDTH)] for _ in range(GC9307_HEIGHT)]
        skin.update_main_screen(main_fb)
        self.assertEqual(skin.main_surface.get_width(), GC9307_WIDTH)
        self.assertEqual(skin.main_surface.get_height(), GC9307_HEIGHT)

        # Generate 120x160 sub framebuffer
        sub_fb = [[0xF800 for _ in range(ST7735_WIDTH)] for _ in range(ST7735_HEIGHT)]
        skin.update_sub_screen(sub_fb)
        self.assertEqual(skin.sub_surface.get_width(), ST7735_WIDTH)
        self.assertEqual(skin.sub_surface.get_height(), ST7735_HEIGHT)

    def test_vector_skin_rendering(self):
        """Verify vector rendering completes cleanly for open and closed flip states."""
        skin = Nokia2660PhoneSkin()
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 380, 870)
        cr = cairo.Context(surface)

        # Render open clamshell
        skin.flip_open = True
        skin.active_button = "5"
        skin.hover_button = "ok"
        skin.render(cr, 380, 870)

        # Render closed clamshell
        skin.flip_open = False
        skin.render(cr, 380, 870)

        # Check surface has non-zero byte content
        data = bytes(surface.get_data())
        self.assertTrue(any(b != 0 for b in data))

    def test_keyboard_shortcuts_coverage(self):
        """Verify that all keyboard shortcuts map to valid NOKIA_KEYS."""
        for keyval, nokia_id in Nokia2660EmulatorWindow.KEY_SHORTCUTS.items():
            self.assertIn(nokia_id, NOKIA_KEYS, f"Shortcut target '{nokia_id}' not in NOKIA_KEYS")

    def test_window_flip_toggle(self):
        """Verify clamshell flip toggling updates skin state and VM GPIO pin 0."""
        vm = T107UnicornVM(trace_instructions=False)
        win = Nokia2660EmulatorWindow(vm=vm)

        self.assertTrue(win.skin.flip_open)
        self.assertTrue(bool(vm.gpio_data_in & 0x1))

        # Toggle to closed
        win._toggle_flip()
        self.assertFalse(win.skin.flip_open)
        self.assertFalse(bool(vm.gpio_data_in & 0x1))

        # Toggle back to open
        win._toggle_flip()
        self.assertTrue(win.skin.flip_open)
        self.assertTrue(bool(vm.gpio_data_in & 0x1))

if __name__ == "__main__":
    unittest.main()
