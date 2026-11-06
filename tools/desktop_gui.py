#!/usr/bin/env python3
"""
N2660Emu: Desktop Host GUI for Nokia 2660 Flip Emulator
Features an authentic Nokia 2660 Flip hardware skin, live 240x320 QVGA main display,
120x160 sub-display, clickable T9 keypad, clamshell flip toggle, and hardware telemetry.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import sys
import os
import time
import struct
import argparse
import subprocess

for pyver in ["3.9", "3.10", "3.11", "3.12", "3.13", "3.14", "3.15"]:
    p = os.path.expanduser(f"~/Library/Python/{pyver}/lib/python/site-packages")
    if os.path.isdir(p) and p not in sys.path:
        sys.path.append(p)

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

import gi
gi.require_version('Gtk', '3.0')
gi.require_version('Gdk', '3.0')
from gi.repository import Gtk, Gdk, GLib, Pango
import cairo

from tools.unicorn_runner import (
    T107UnicornVM, N2660VM,
    GC9307_WIDTH, GC9307_HEIGHT,
    ST7735_WIDTH, ST7735_HEIGHT,
    NOKIA_KEYS,
    UC_ARM_REG_PC, UC_ARM_REG_SP, UC_ARM_REG_LR,
    UC_ARM_REG_CPSR, UC_ARM_REG_R0, UC_ARM_REG_R1,
    UC_ARM_REG_R2, UC_ARM_REG_R3
)

# Precomputed RGB565 to 32-bit ARGB/XRGB (Cairo FORMAT_RGB24) lookup table
LUT_RGB565_TO_RGB24 = [
    struct.pack(
        '<BBBB',
        (c & 0x1F) * 255 // 31,           # Blue
        ((c >> 5) & 0x3F) * 255 // 63,    # Green
        ((c >> 11) & 0x1F) * 255 // 31,   # Red
        0xFF                              # Alpha / X
    )
    for c in range(65536)
]

# Color themes for Nokia 2660 casing
THEMES = {
    "Deep Blue": {
        "body_top": (0.13, 0.22, 0.38),
        "body_bot": (0.07, 0.12, 0.22),
        "bezel": (0.04, 0.05, 0.07),
        "key_bg": (0.18, 0.24, 0.34),
        "key_press": (0.28, 0.38, 0.54),
        "key_text": (0.95, 0.95, 0.95),
        "accent": (0.22, 0.45, 0.78),
    },
    "Classic Black": {
        "body_top": (0.18, 0.18, 0.20),
        "body_bot": (0.09, 0.09, 0.10),
        "bezel": (0.02, 0.02, 0.02),
        "key_bg": (0.22, 0.22, 0.24),
        "key_press": (0.35, 0.35, 0.38),
        "key_text": (0.95, 0.95, 0.95),
        "accent": (0.50, 0.50, 0.55),
    },
    "Pop Red": {
        "body_top": (0.68, 0.14, 0.16),
        "body_bot": (0.42, 0.08, 0.10),
        "bezel": (0.04, 0.04, 0.05),
        "key_bg": (0.55, 0.16, 0.18),
        "key_press": (0.82, 0.26, 0.28),
        "key_text": (0.98, 0.98, 0.98),
        "accent": (0.95, 0.25, 0.28),
    }
}

class Nokia2660PhoneSkin:
    """
    Renders the authentic Nokia 2660 Flip physical skin, screen bezels,
    buttons, and handles mouse hit testing for keypad interaction.
    """

    def __init__(self):
        self.theme_name = "Deep Blue"
        self.theme = THEMES[self.theme_name]
        self.active_button = None
        self.hover_button = None
        self.flip_open = True
        self.main_surface = cairo.ImageSurface(cairo.FORMAT_RGB24, GC9307_WIDTH, GC9307_HEIGHT)
        self.sub_surface = cairo.ImageSurface(cairo.FORMAT_RGB24, ST7735_WIDTH, ST7735_HEIGHT)

        # Button layout hitboxes: (id, label, sub_label, x, y, w, h, kind)
        self.buttons = []
        self._build_button_geometry()

    def set_theme(self, name):
        if name in THEMES:
            self.theme_name = name
            self.theme = THEMES[name]

    def _build_button_geometry(self):
        self.buttons = [
            # Function cluster
            {"id": "lsk",   "label": "-",    "sub": "Menu",     "rect": (55, 445, 60, 36), "kind": "soft"},
            {"id": "rsk",   "label": "-",    "sub": "Names",    "rect": (265, 445, 60, 36), "kind": "soft"},
            {"id": "call",  "label": "CALL", "sub": "",         "rect": (55, 490, 60, 42), "kind": "call"},
            {"id": "end",   "label": "END",  "sub": "",         "rect": (265, 490, 60, 42), "kind": "end"},
            # D-Pad
            {"id": "up",    "label": "▲",    "sub": "",         "rect": (165, 445, 50, 26), "kind": "dpad"},
            {"id": "down",  "label": "▼",    "sub": "",         "rect": (165, 506, 50, 26), "kind": "dpad"},
            {"id": "left",  "label": "◀",    "sub": "",         "rect": (135, 471, 30, 35), "kind": "dpad"},
            {"id": "right", "label": "▶",    "sub": "",         "rect": (215, 471, 30, 35), "kind": "dpad"},
            {"id": "ok",    "label": "OK",   "sub": "",         "rect": (165, 471, 50, 35), "kind": "ok"},
            # Numeric T9 keys (Row 1)
            {"id": "1",     "label": "1",    "sub": " ⌕ ",      "rect": (55, 550, 80, 52), "kind": "num"},
            {"id": "2",     "label": "2",    "sub": "abc",      "rect": (150, 550, 80, 52), "kind": "num"},
            {"id": "3",     "label": "3",    "sub": "def",      "rect": (245, 550, 80, 52), "kind": "num"},
            # Row 2
            {"id": "4",     "label": "4",    "sub": "ghi",      "rect": (55, 612, 80, 52), "kind": "num"},
            {"id": "5",     "label": "5",    "sub": "jkl",      "rect": (150, 612, 80, 52), "kind": "num"},
            {"id": "6",     "label": "6",    "sub": "mno",      "rect": (245, 612, 80, 52), "kind": "num"},
            # Row 3
            {"id": "7",     "label": "7",    "sub": "pqrs",     "rect": (55, 674, 80, 52), "kind": "num"},
            {"id": "8",     "label": "8",    "sub": "tuv",      "rect": (150, 674, 80, 52), "kind": "num"},
            {"id": "9",     "label": "9",    "sub": "wxyz",     "rect": (245, 674, 80, 52), "kind": "num"},
            # Row 4
            {"id": "*",     "label": "*",    "sub": " + ",      "rect": (55, 736, 80, 52), "kind": "num"},
            {"id": "0",     "label": "0",    "sub": " ␣ ",      "rect": (150, 736, 80, 52), "kind": "num"},
            {"id": "#",     "label": "#",    "sub": " ⇧ ",      "rect": (245, 736, 80, 52), "kind": "num"},
        ]

    def hit_test(self, px, py):
        """Returns button ID if (px, py) intersects any keypad button when flip is open."""
        if not self.flip_open:
            return None
        for btn in self.buttons:
            bx, by, bw, bh = btn["rect"]
            if bx <= px <= bx + bw and by <= py <= by + bh:
                return btn["id"]
        return None

    def update_main_screen(self, framebuffer):
        """Converts VM RGB565 240x320 framebuffer to Cairo image surface."""
        raw = b''.join(b''.join(LUT_RGB565_TO_RGB24[pixel] for pixel in row) for row in framebuffer)
        buf = self.main_surface.get_data()
        buf[:len(raw)] = raw
        self.main_surface.mark_dirty()

    def update_sub_screen(self, sub_framebuffer):
        """Converts VM RGB565 120x160 sub-framebuffer to Cairo image surface."""
        raw = b''.join(b''.join(LUT_RGB565_TO_RGB24[pixel] for pixel in row) for row in sub_framebuffer)
        buf = self.sub_surface.get_data()
        buf[:len(raw)] = raw
        self.sub_surface.mark_dirty()

    def _draw_rounded_rect(self, cr, x, y, w, h, r):
        cr.new_sub_path()
        cr.arc(x + w - r, y + r, r, -1.5708, 0)
        cr.arc(x + w - r, y + h - r, r, 0, 1.5708)
        cr.arc(x + r, y + h - r, r, 1.5708, 3.1416)
        cr.arc(x + r, y + r, r, 3.1416, 4.7124)
        cr.close_path()

    def render(self, cr, width, height):
        # Clear background canvas
        cr.set_source_rgb(0.10, 0.11, 0.13)
        cr.paint()

        if self.flip_open:
            self._render_open_flip(cr)
        else:
            self._render_closed_flip(cr)

    def _render_open_flip(self, cr):
        t = self.theme

        # 1. Upper Clamshell Body (Screen Half)
        pat_top = cairo.LinearGradient(30, 20, 350, 400)
        pat_top.add_color_stop_rgb(0, *t["body_top"])
        pat_top.add_color_stop_rgb(1, *t["body_bot"])
        cr.set_source(pat_top)
        self._draw_rounded_rect(cr, 30, 20, 320, 380, 24)
        cr.fill_preserve()
        cr.set_source_rgba(1, 1, 1, 0.15)
        cr.set_line_width(2)
        cr.stroke()

        # Earpiece speaker slit
        cr.set_source_rgb(0.05, 0.05, 0.06)
        self._draw_rounded_rect(cr, 160, 32, 60, 6, 3)
        cr.fill()

        # Nokia Chrome Logo
        cr.set_source_rgb(0.85, 0.88, 0.92)
        cr.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
        cr.set_font_size(13)
        cr.move_to(166, 52)
        cr.show_text("NOKIA")

        # Screen Black Glass Bezel
        cr.set_source_rgb(*t["bezel"])
        self._draw_rounded_rect(cr, 64, 58, 252, 332, 10)
        cr.fill_preserve()
        cr.set_source_rgba(0, 0, 0, 0.6)
        cr.set_line_width(2)
        cr.stroke()

        # Main LCD Screen (240x320)
        cr.save()
        cr.set_source_surface(self.main_surface, 70, 64)
        cr.rectangle(70, 64, GC9307_WIDTH, GC9307_HEIGHT)
        cr.fill()
        cr.restore()

        # Subtle Glass Screen Reflection Gradient
        glass_pat = cairo.LinearGradient(70, 64, 310, 384)
        glass_pat.add_color_stop_rgba(0, 1, 1, 1, 0.08)
        glass_pat.add_color_stop_rgba(0.4, 1, 1, 1, 0.02)
        glass_pat.add_color_stop_rgba(1, 1, 1, 1, 0.0)
        cr.set_source(glass_pat)
        cr.rectangle(70, 64, GC9307_WIDTH, GC9307_HEIGHT)
        cr.fill()

        # 2. Dual Cylindrical Hinge
        hinge_pat = cairo.LinearGradient(30, 400, 350, 425)
        hinge_pat.add_color_stop_rgb(0, 0.20, 0.22, 0.26)
        hinge_pat.add_color_stop_rgb(0.5, 0.35, 0.38, 0.44)
        hinge_pat.add_color_stop_rgb(1, 0.15, 0.17, 0.20)
        cr.set_source(hinge_pat)
        self._draw_rounded_rect(cr, 45, 402, 290, 22, 8)
        cr.fill()

        # 3. Lower Clamshell Body (Keypad Half)
        pat_bot = cairo.LinearGradient(30, 425, 350, 855)
        pat_bot.add_color_stop_rgb(0, *t["body_top"])
        pat_bot.add_color_stop_rgb(1, *t["body_bot"])
        cr.set_source(pat_bot)
        self._draw_rounded_rect(cr, 30, 425, 320, 430, 24)
        cr.fill_preserve()
        cr.set_source_rgba(1, 1, 1, 0.12)
        cr.set_line_width(2)
        cr.stroke()

        # Render Keypad Buttons
        for btn in self.buttons:
            self._render_button(cr, btn)

        # Microphone pinhole
        cr.set_source_rgb(0.04, 0.04, 0.05)
        cr.arc(190, 815, 3, 0, 6.283)
        cr.fill()

    def _render_closed_flip(self, cr):
        t = self.theme

        # Front Clamshell Cover
        pat_cover = cairo.LinearGradient(30, 100, 350, 540)
        pat_cover.add_color_stop_rgb(0, *t["body_top"])
        pat_cover.add_color_stop_rgb(1, *t["body_bot"])
        cr.set_source(pat_cover)
        self._draw_rounded_rect(cr, 30, 100, 320, 440, 28)
        cr.fill_preserve()
        cr.set_source_rgba(1, 1, 1, 0.18)
        cr.set_line_width(2)
        cr.stroke()

        # Camera Module Pill
        cr.set_source_rgb(0.08, 0.09, 0.11)
        self._draw_rounded_rect(cr, 130, 120, 120, 38, 19)
        cr.fill()

        # Camera Lens
        cr.set_source_rgb(0.02, 0.02, 0.03)
        cr.arc(155, 139, 12, 0, 6.283)
        cr.fill()
        cr.set_source_rgb(0.15, 0.25, 0.40)
        cr.arc(155, 139, 7, 0, 6.283)
        cr.fill()

        # Camera Flash LED
        cr.set_source_rgb(0.95, 0.90, 0.60)
        cr.arc(220, 139, 6, 0, 6.283)
        cr.fill()

        # Nokia Logo on Cover
        cr.set_source_rgb(0.85, 0.88, 0.92)
        cr.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
        cr.set_font_size(15)
        cr.move_to(160, 195)
        cr.show_text("NOKIA")

        # Sub-Display Bezel (120x160 native, scaled up 1.5x -> 180x240)
        sub_x, sub_y = 100, 220
        sub_w, sub_h = 180, 240
        cr.set_source_rgb(*t["bezel"])
        self._draw_rounded_rect(cr, sub_x - 8, sub_y - 8, sub_w + 16, sub_h + 16, 12)
        cr.fill()

        # Outer ST7735 Sub-Display
        cr.save()
        cr.translate(sub_x, sub_y)
        cr.scale(1.5, 1.5)
        cr.set_source_surface(self.sub_surface, 0, 0)
        cr.rectangle(0, 0, ST7735_WIDTH, ST7735_HEIGHT)
        cr.fill()
        cr.restore()

        # Helper hint
        cr.set_source_rgb(0.60, 0.65, 0.72)
        cr.set_font_size(11)
        cr.move_to(95, 500)
        cr.show_text("Flip Closed (Click 'Open Flip' or press 'F')")

    def _render_button(self, cr, btn):
        bx, by, bw, bh = btn["rect"]
        bid = btn["id"]
        kind = btn["kind"]
        is_pressed = (bid == self.active_button)
        is_hover = (bid == self.hover_button)

        # Color evaluation
        if kind == "call":
            base_col = (0.15, 0.55, 0.22) if not is_pressed else (0.22, 0.75, 0.32)
        elif kind == "end":
            base_col = (0.75, 0.18, 0.20) if not is_pressed else (0.95, 0.25, 0.28)
        elif kind == "ok":
            base_col = (0.28, 0.35, 0.46) if not is_pressed else (0.42, 0.52, 0.68)
        elif is_pressed:
            base_col = self.theme["key_press"]
        elif is_hover:
            base_col = (self.theme["key_bg"][0] + 0.08, self.theme["key_bg"][1] + 0.08, self.theme["key_bg"][2] + 0.08)
        else:
            base_col = self.theme["key_bg"]

        # Button shadow / 3D depth
        cr.set_source_rgba(0, 0, 0, 0.35)
        self._draw_rounded_rect(cr, bx, by + 2, bw, bh, 8)
        cr.fill()

        # Button Face
        cr.set_source_rgb(*base_col)
        self._draw_rounded_rect(cr, bx, by if not is_pressed else by + 1, bw, bh, 8)
        cr.fill_preserve()
        cr.set_source_rgba(1, 1, 1, 0.15 if not is_pressed else 0.05)
        cr.set_line_width(1.5)
        cr.stroke()

        # Button Label Text
        text_y_offset = 1 if is_pressed else 0
        cr.set_source_rgb(*self.theme["key_text"])
        cr.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)

        if kind == "num":
            cr.set_font_size(18)
            cr.move_to(bx + 14, by + 30 + text_y_offset)
            cr.show_text(btn["label"])

            cr.set_source_rgb(0.70, 0.75, 0.80)
            cr.set_font_size(10)
            cr.move_to(bx + 34, by + 30 + text_y_offset)
            cr.show_text(btn["sub"])
        elif kind == "dpad":
            cr.set_font_size(12)
            cr.move_to(bx + bw // 2 - 5, by + bh // 2 + 5 + text_y_offset)
            cr.show_text(btn["label"])
        elif kind == "ok":
            cr.set_font_size(13)
            cr.move_to(bx + 14, by + 22 + text_y_offset)
            cr.show_text("OK")
        elif kind == "call":
            cr.set_font_size(11)
            cr.move_to(bx + 15, by + 26 + text_y_offset)
            cr.show_text("CALL")
        elif kind == "end":
            cr.set_font_size(11)
            cr.move_to(bx + 17, by + 26 + text_y_offset)
            cr.show_text("END")
        else: # Soft keys
            cr.set_font_size(10)
            cr.move_to(bx + 12, by + 22 + text_y_offset)
            cr.show_text(btn["sub"])

class Nokia2660EmulatorWindow(Gtk.Window):
    """
    Main Desktop Host Window for the Nokia 2660 Flip Emulator.
    Integrates Cairo phone skin canvas, control toolbar, live CPU telemetry, and serial log.
    """

    KEY_SHORTCUTS = {
        Gdk.KEY_0: "0", Gdk.KEY_1: "1", Gdk.KEY_2: "2", Gdk.KEY_3: "3", Gdk.KEY_4: "4",
        Gdk.KEY_5: "5", Gdk.KEY_6: "6", Gdk.KEY_7: "7", Gdk.KEY_8: "8", Gdk.KEY_9: "9",
        Gdk.KEY_asterisk: "*", Gdk.KEY_numbersign: "#",
        Gdk.KEY_Up: "up", Gdk.KEY_Down: "down", Gdk.KEY_Left: "left", Gdk.KEY_Right: "right",
        Gdk.KEY_Return: "ok", Gdk.KEY_KP_Enter: "ok", Gdk.KEY_space: "ok",
        Gdk.KEY_bracketleft: "lsk", Gdk.KEY_bracketright: "rsk",
        Gdk.KEY_c: "call", Gdk.KEY_C: "call",
        Gdk.KEY_x: "end", Gdk.KEY_X: "end", Gdk.KEY_e: "end", Gdk.KEY_E: "end",
        Gdk.KEY_plus: "vol_up", Gdk.KEY_equal: "vol_up",
        Gdk.KEY_minus: "vol_down", Gdk.KEY_underscore: "vol_down",
        Gdk.KEY_exclam: "sos",
    }

    def __init__(self, vm, entry_point=0x80000000, slice_insns=30000):
        super().__init__(title="Nokia 2660 Flip Hardware Emulator (TA-1480)")
        self.set_default_size(860, 920)
        self.set_position(Gtk.WindowPosition.CENTER)

        self.vm = vm
        self.entry_point = entry_point
        self.slice_insns = slice_insns
        self.is_running = True
        self.audio_effects = True
        self.last_audio_events_count = 0
        self.last_insn_count = 0
        self.last_time = time.time()
        self.current_mips = 0.0

        # Initialize phone skin
        self.skin = Nokia2660PhoneSkin()
        self.skin.flip_open = bool(self.vm.gpio_data_in & 0x1)

        # Initialize CPU
        self.vm.init_cpu(self.entry_point)

        self._build_ui()
        self._setup_events()

        # Start emulation tick (~60 FPS dispatch)
        GLib.timeout_add(16, self._on_emulation_tick)

    def _build_ui(self):
        # Apply CSS styling for clean telemetry & console fonts without deprecation warnings
        css_provider = Gtk.CssProvider()
        css_provider.load_from_data(b"""
        .telemetry-lbl {
            font-family: monospace;
            font-size: 10pt;
        }
        .serial-console {
            background-color: #181a20;
            color: #33dd88;
            font-family: monospace;
            font-size: 9pt;
        }
        """)
        screen = Gdk.Screen.get_default()
        if screen:
            Gtk.StyleContext.add_provider_for_screen(
                screen,
                css_provider,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
            )

        main_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)
        main_box.set_margin_top(10)
        main_box.set_margin_bottom(10)
        main_box.set_margin_start(10)
        main_box.set_margin_end(10)
        self.add(main_box)

        # Left: Phone Canvas
        canvas_frame = Gtk.Frame()
        canvas_frame.set_shadow_type(Gtk.ShadowType.ETCHED_IN)
        self.drawing_area = Gtk.DrawingArea()
        self.drawing_area.set_size_request(380, 870)
        canvas_frame.add(self.drawing_area)
        main_box.pack_start(canvas_frame, False, False, 0)

        # Right: Telemetry & Controls Box
        right_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        main_box.pack_start(right_box, True, True, 0)

        # 1. Header Banner
        header = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        title_lbl = Gtk.Label()
        title_lbl.set_markup("<span font='16' weight='bold'>Nokia 2660 Flip Emulator</span>")
        title_lbl.set_halign(Gtk.Align.START)
        sub_lbl = Gtk.Label()
        sub_lbl.set_markup("<span font='10' foreground='#888888'>Unisoc T107 (UMS9117) | ARM Cortex-A7 @ 1.0 GHz</span>")
        sub_lbl.set_halign(Gtk.Align.START)
        header.pack_start(title_lbl, False, False, 0)
        header.pack_start(sub_lbl, False, False, 0)
        right_box.pack_start(header, False, False, 0)

        # 2. Control Toolbar
        toolbar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        self.btn_pause = Gtk.Button(label="⏸ Pause")
        self.btn_pause.connect("clicked", self._on_pause_clicked)
        toolbar.pack_start(self.btn_pause, False, False, 0)

        btn_step = Gtk.Button(label="⏭ Step 1")
        btn_step.connect("clicked", lambda b: self._step_vm(1))
        toolbar.pack_start(btn_step, False, False, 0)

        btn_step1k = Gtk.Button(label="⏩ Step 1K")
        btn_step1k.connect("clicked", lambda b: self._step_vm(1000))
        toolbar.pack_start(btn_step1k, False, False, 0)

        btn_reset = Gtk.Button(label="🔄 Reset")
        btn_reset.connect("clicked", self._on_reset_clicked)
        toolbar.pack_start(btn_reset, False, False, 0)

        self.btn_flip = Gtk.Button(label="📱 Flip: OPEN")
        self.btn_flip.connect("clicked", self._on_flip_clicked)
        toolbar.pack_start(self.btn_flip, False, False, 0)

        btn_snap = Gtk.Button(label="📸 Snapshot")
        btn_snap.connect("clicked", self._on_snapshot_clicked)
        toolbar.pack_start(btn_snap, False, False, 0)

        # Color Theme Selector
        theme_combo = Gtk.ComboBoxText()
        for tname in THEMES.keys():
            theme_combo.append_text(tname)
        theme_combo.set_active(0)
        theme_combo.connect("changed", self._on_theme_changed)
        toolbar.pack_end(theme_combo, False, False, 0)

        right_box.pack_start(toolbar, False, False, 0)

        # 3. Telemetry Notebook
        telemetry_frame = Gtk.Frame(label=" Hardware Telemetry ")
        grid = Gtk.Grid()
        grid.set_column_spacing(16)
        grid.set_row_spacing(6)
        grid.set_margin_top(8)
        grid.set_margin_bottom(8)
        grid.set_margin_start(10)
        grid.set_margin_end(10)

        # Left Column: CPU Registers
        self.lbl_pc = Gtk.Label(label="PC:   0x80000000")
        self.lbl_sp = Gtk.Label(label="SP:   0x8003FF00")
        self.lbl_lr = Gtk.Label(label="LR:   0x80000000")
        self.lbl_cpsr = Gtk.Label(label="CPSR: 0x600000D3 [SVC]")
        self.lbl_insns = Gtk.Label(label="Insns: 0")
        self.lbl_speed = Gtk.Label(label="Speed: 0.0 MIPS")

        for lbl in [self.lbl_pc, self.lbl_sp, self.lbl_lr, self.lbl_cpsr, self.lbl_insns, self.lbl_speed]:
            lbl.set_halign(Gtk.Align.START)
            lbl.get_style_context().add_class("telemetry-lbl")

        grid.attach(self.lbl_pc, 0, 0, 1, 1)
        grid.attach(self.lbl_sp, 0, 1, 1, 1)
        grid.attach(self.lbl_lr, 0, 2, 1, 1)
        grid.attach(self.lbl_cpsr, 0, 3, 1, 1)
        grid.attach(self.lbl_insns, 0, 4, 1, 1)
        grid.attach(self.lbl_speed, 0, 5, 1, 1)

        # Right Column: Peripherals
        self.lbl_intc = Gtk.Label(label="INTC Raw:   0x00000000")
        self.lbl_mask = Gtk.Label(label="INTC Mask:  0x00000000")
        self.lbl_timer = Gtk.Label(label="Timer Cnt:  0x00000000")
        self.lbl_kpd = Gtk.Label(label="Keypad:     Idle")
        self.lbl_audio = Gtk.Label(label="Audio Tone: Idle")
        self.lbl_uart_tx = Gtk.Label(label="UART0 TX:   0 bytes")

        for lbl in [self.lbl_intc, self.lbl_mask, self.lbl_timer, self.lbl_kpd, self.lbl_audio, self.lbl_uart_tx]:
            lbl.set_halign(Gtk.Align.START)
            lbl.get_style_context().add_class("telemetry-lbl")

        grid.attach(self.lbl_intc, 1, 0, 1, 1)
        grid.attach(self.lbl_mask, 1, 1, 1, 1)
        grid.attach(self.lbl_timer, 1, 2, 1, 1)
        grid.attach(self.lbl_kpd, 1, 3, 1, 1)
        grid.attach(self.lbl_audio, 1, 4, 1, 1)
        grid.attach(self.lbl_uart_tx, 1, 5, 1, 1)

        telemetry_frame.add(grid)
        right_box.pack_start(telemetry_frame, False, False, 0)

        # 4. Live Serial Console (UART0)
        console_frame = Gtk.Frame(label=" Live UART0 Serial Console ")
        console_vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        console_frame.add(console_vbox)

        scroller = Gtk.ScrolledWindow()
        scroller.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        self.console_view = Gtk.TextView()
        self.console_view.set_editable(False)
        self.console_view.set_cursor_visible(False)
        self.console_view.get_style_context().add_class("serial-console")
        scroller.add(self.console_view)
        console_vbox.pack_start(scroller, True, True, 0)

        # UART RX Injection Input Box
        rx_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        rx_box.set_margin_start(4)
        rx_box.set_margin_end(4)
        rx_box.set_margin_bottom(4)
        self.entry_rx = Gtk.Entry()
        self.entry_rx.set_placeholder_text("Type message and press Enter to send over UART0...")
        self.entry_rx.connect("activate", self._on_send_uart)
        rx_box.pack_start(self.entry_rx, True, True, 0)
        btn_send = Gtk.Button(label="Send")
        btn_send.connect("clicked", self._on_send_uart)
        rx_box.pack_start(btn_send, False, False, 0)
        console_vbox.pack_start(rx_box, False, False, 0)

        right_box.pack_start(console_frame, True, True, 0)

    def _setup_events(self):
        self.drawing_area.set_events(
            Gdk.EventMask.BUTTON_PRESS_MASK |
            Gdk.EventMask.BUTTON_RELEASE_MASK |
            Gdk.EventMask.POINTER_MOTION_MASK |
            Gdk.EventMask.LEAVE_NOTIFY_MASK
        )
        self.drawing_area.connect("draw", self._on_canvas_draw)
        self.drawing_area.connect("button-press-event", self._on_canvas_button_press)
        self.drawing_area.connect("button-release-event", self._on_canvas_button_release)
        self.drawing_area.connect("motion-notify-event", self._on_canvas_motion)
        self.drawing_area.connect("leave-notify-event", self._on_canvas_leave)

        self.connect("key-press-event", self._on_key_press)
        self.connect("key-release-event", self._on_key_release)
        self.connect("destroy", Gtk.main_quit)

    def _on_canvas_draw(self, widget, cr):
        alloc = widget.get_allocation()
        self.skin.render(cr, alloc.width, alloc.height)
        return False

    def _on_canvas_button_press(self, widget, event):
        btn_id = self.skin.hit_test(event.x, event.y)
        if btn_id:
            self.skin.active_button = btn_id
            self.vm.inject_key(btn_id, pressed=True)
            self._play_click_audio()
            widget.queue_draw()
        return True

    def _on_canvas_button_release(self, widget, event):
        if self.skin.active_button:
            self.vm.inject_key(self.skin.active_button, pressed=False)
            self.skin.active_button = None
            widget.queue_draw()
        return True

    def _on_canvas_motion(self, widget, event):
        btn_id = self.skin.hit_test(event.x, event.y)
        if btn_id != self.skin.hover_button:
            self.skin.hover_button = btn_id
            widget.queue_draw()
        return True

    def _on_canvas_leave(self, widget, event):
        if self.skin.hover_button or self.skin.active_button:
            if self.skin.active_button:
                self.vm.inject_key(self.skin.active_button, pressed=False)
            self.skin.active_button = None
            self.skin.hover_button = None
            widget.queue_draw()
        return True

    def _on_key_press(self, widget, event):
        if event.keyval in self.KEY_SHORTCUTS:
            k = self.KEY_SHORTCUTS[event.keyval]
            self.skin.active_button = k
            self.vm.inject_key(k, pressed=True)
            self._play_click_audio()
            self.drawing_area.queue_draw()
            return True
        elif event.keyval in (Gdk.KEY_f, Gdk.KEY_F):
            self._toggle_flip()
            return True
        elif event.keyval in (Gdk.KEY_p, Gdk.KEY_P):
            self._on_pause_clicked(self.btn_pause)
            return True
        return False

    def _on_key_release(self, widget, event):
        if event.keyval in self.KEY_SHORTCUTS:
            k = self.KEY_SHORTCUTS[event.keyval]
            if self.skin.active_button == k:
                self.skin.active_button = None
            self.vm.inject_key(k, pressed=False)
            self.drawing_area.queue_draw()
            return True
        return False

    def _toggle_flip(self):
        new_state = not self.skin.flip_open
        self.skin.flip_open = new_state
        self.vm.set_flip_state(new_state)
        self.btn_flip.set_label("📱 Flip: OPEN" if new_state else "📱 Flip: CLOSED")
        self.drawing_area.queue_draw()

    def _on_flip_clicked(self, btn):
        self._toggle_flip()

    def _on_pause_clicked(self, btn):
        self.is_running = not self.is_running
        self.btn_pause.set_label("▶ Resume" if not self.is_running else "⏸ Pause")

    def _step_vm(self, count):
        self.vm.step(count=count)
        self._update_telemetry()
        self.skin.update_main_screen(self.vm.framebuffer)
        self.skin.update_sub_screen(self.vm.sub_framebuffer)
        self.drawing_area.queue_draw()

    def _on_reset_clicked(self, btn):
        self.vm.init_cpu(self.entry_point)
        self._update_telemetry()
        self.drawing_area.queue_draw()

    def _on_theme_changed(self, combo):
        name = combo.get_active_text()
        self.skin.set_theme(name)
        self.drawing_area.queue_draw()

    def _on_snapshot_clicked(self, btn):
        out_dir = os.path.join(PROJECT_ROOT, "build")
        os.makedirs(out_dir, exist_ok=True)
        main_ppm = os.path.join(out_dir, "screen_snapshot.ppm")
        sub_ppm = os.path.join(out_dir, "sub_screen_snapshot.ppm")
        self.vm.export_ppm(main_ppm)
        self.vm.export_sub_ppm(sub_ppm)
        dialog = Gtk.MessageDialog(
            transient_for=self,
            flags=0,
            message_type=Gtk.MessageType.INFO,
            buttons=Gtk.ButtonsType.OK,
            text="Framebuffers Saved!"
        )
        dialog.format_secondary_text(f"Exported:\n• {main_ppm}\n• {sub_ppm}")
        dialog.run()
        dialog.destroy()

    def _on_send_uart(self, widget):
        text = self.entry_rx.get_text()
        if text:
            for ch in text.encode('ascii', errors='replace'):
                self.vm.uart0_rx_fifo.append(ch)
            self.vm.uart0_rx_fifo.append(ord('\n'))
            self.entry_rx.set_text("")

    def _play_click_audio(self):
        if self.audio_effects and sys.platform == "darwin":
            sound_file = "/System/Library/Sounds/Tink.aiff"
            if os.path.isfile(sound_file):
                try:
                    subprocess.Popen(["afplay", sound_file], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                except Exception:
                    pass

    def _on_emulation_tick(self):
        if self.is_running:
            ok, err = self.vm.step(count=self.slice_insns, timeout_us=25000)
            if not ok:
                self.is_running = False
                self.btn_pause.set_label("▶ Resume")

        now = time.time()
        dt = now - self.last_time
        if dt >= 0.25:
            d_insn = self.vm.instruction_count - self.last_insn_count
            self.current_mips = (d_insn / dt) / 1000000.0 if dt > 0 else 0.0
            self.last_insn_count = self.vm.instruction_count
            self.last_time = now

        # Update displays
        self.skin.update_main_screen(self.vm.framebuffer)
        self.skin.update_sub_screen(self.vm.sub_framebuffer)
        self.drawing_area.queue_draw()

        # Update telemetry & serial logs
        self._update_telemetry()
        self._update_serial_console()

        return True

    def _update_telemetry(self):
        try:
            pc = self.vm.uc.reg_read(UC_ARM_REG_PC)
            sp = self.vm.uc.reg_read(UC_ARM_REG_SP)
            lr = self.vm.uc.reg_read(UC_ARM_REG_LR)
            cpsr = self.vm.uc.reg_read(UC_ARM_REG_CPSR)
        except Exception:
            pc, sp, lr, cpsr = 0, 0, 0, 0

        mode_num = cpsr & 0x1F
        mode_str = {0x10: "USR", 0x12: "IRQ", 0x13: "SVC", 0x17: "ABT", 0x1B: "UND", 0x1F: "SYS"}.get(mode_num, hex(mode_num))

        self.lbl_pc.set_text(f"PC:   0x{pc:08X}")
        self.lbl_sp.set_text(f"SP:   0x{sp:08X}")
        self.lbl_lr.set_text(f"LR:   0x{lr:08X}")
        self.lbl_cpsr.set_text(f"CPSR: 0x{cpsr:08X} [{mode_str}]")
        self.lbl_insns.set_text(f"Insns: {self.vm.instruction_count:,}")
        self.lbl_speed.set_text(f"Speed: {self.current_mips:5.2f} MIPS")

        raw_intc = getattr(self.vm, 'intc_raw_sts', 0)
        mask_intc = self.vm.intc_get_irq_masked_status() if hasattr(self.vm, 'intc_get_irq_masked_status') else 0
        self.lbl_intc.set_text(f"INTC Raw:  0x{raw_intc:08X}")
        self.lbl_mask.set_text(f"INTC Mask: 0x{mask_intc:08X}")
        self.lbl_timer.set_text(f"Timer Cnt: 0x{getattr(self.vm, 'timer_load', 0):08X}")

        # Audio tone display
        tones = getattr(self.vm, 'audio_tone_events', [])
        if tones:
            last_tone = tones[-1]
            self.lbl_audio.set_text(f"Audio Tone: {last_tone['freq']} Hz ({last_tone['dur']} ms)")
        else:
            self.lbl_audio.set_text("Audio Tone: Idle")

        uart_bytes = len(getattr(self.vm, 'uart0_tx_buffer', []))
        self.lbl_uart_tx.set_text(f"UART0 TX:  {uart_bytes:,} bytes")

    def _update_serial_console(self):
        buf = self.console_view.get_buffer()
        uart_raw = bytes(self.vm.uart0_tx_buffer)
        if len(uart_raw) > buf.get_char_count():
            # Get newly received characters
            new_text = uart_raw[buf.get_char_count():].decode('latin-1', errors='replace')
            end_iter = buf.get_end_iter()
            buf.insert(end_iter, new_text)
            # Auto scroll to bottom
            adj = self.console_view.get_vadjustment()
            if adj:
                adj.set_value(adj.get_upper())

def run_desktop_gui(bin_path, entry_point=0x80000000, flip="open", theme="Deep Blue"):
    """Launches the Nokia 2660 desktop host emulator window."""
    vm = T107UnicornVM(trace_instructions=False)
    vm.set_flip_state(flip == "open")

    if bin_path and os.path.isfile(bin_path):
        vm.load_binary(bin_path, entry_point)

    win = Nokia2660EmulatorWindow(vm=vm, entry_point=entry_point)
    win.skin.set_theme(theme)
    win.show_all()
    Gtk.main()
    return 0

def main():
    parser = argparse.ArgumentParser(description="N2660Emu: Desktop Host GUI for Nokia 2660 Flip")
    parser.add_argument("binary", nargs="?", default=None, help="Path to raw firmware binary")
    parser.add_argument("--entry", type=lambda x: int(x, 0), default=0x80000000, help="Entry point address")
    parser.add_argument("--flip", choices=["open", "closed"], default="open", help="Initial flip state")
    parser.add_argument("--theme", choices=list(THEMES.keys()), default="Deep Blue", help="Casing color theme")
    parser.add_argument("--test", action="store_true", help="Run test initialization and exit immediately")

    args = parser.parse_args()

    bin_path = args.binary
    if not bin_path:
        default_demo = os.path.join(PROJECT_ROOT, "tests", "baremetal", "interactive_demo.bin")
        default_test = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        bin_path = default_demo if os.path.isfile(default_demo) else default_test

    if args.test:
        vm = T107UnicornVM(trace_instructions=False)
        if bin_path and os.path.isfile(bin_path):
            vm.load_binary(bin_path, args.entry)
        win = Nokia2660EmulatorWindow(vm=vm, entry_point=args.entry)
        win.skin.set_theme(args.theme)
        win.skin.update_main_screen(vm.framebuffer)
        win.skin.update_sub_screen(vm.sub_framebuffer)
        print("[Desktop GUI] Window, widgets, Cairo phone skin, and VM initialized successfully.")
        sys.exit(0)

    sys.exit(run_desktop_gui(bin_path, args.entry, args.flip, args.theme))

if __name__ == "__main__":
    main()
