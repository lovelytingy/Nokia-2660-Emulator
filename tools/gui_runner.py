#!/usr/bin/env python3
"""
N2660Emu: Interactive Real-Time Terminal GUI & Keypad Runner
Provides live 240x320 main LCD and 128x160 sub-LCD graphics rendering,
real-time Nokia 2660 keypad matrix input, hardware telemetry, and serial logs.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import sys
import os
import time
import struct
import argparse
import select

for pyver in ["3.9", "3.10", "3.11", "3.12", "3.13", "3.14", "3.15"]:
    p = os.path.expanduser(f"~/Library/Python/{pyver}/lib/python/site-packages")
    if os.path.isdir(p) and p not in sys.path:
        sys.path.append(p)

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from tools.unicorn_runner import (
    T107UnicornVM, N2660VM,
    GC9307_WIDTH, GC9307_HEIGHT,
    ST7735_WIDTH, ST7735_HEIGHT,
    NOKIA_KEYS,
    UC_ARM_REG_PC, UC_ARM_REG_SP, UC_ARM_REG_LR,
    UC_ARM_REG_CPSR, UC_ARM_REG_R0, UC_ARM_REG_R1,
    UC_ARM_REG_R2, UC_ARM_REG_R3
)

# Terminal ANSI Color and Control Codes
ESC = "\033"
C_RESET = f"{ESC}[0m"
C_BOLD = f"{ESC}[1m"
C_DIM = f"{ESC}[2m"
C_REV = f"{ESC}[7m"

C_RED = f"{ESC}[31m"
C_GREEN = f"{ESC}[32m"
C_YELLOW = f"{ESC}[33m"
C_BLUE = f"{ESC}[34m"
C_MAGENTA = f"{ESC}[35m"
C_CYAN = f"{ESC}[36m"
C_WHITE = f"{ESC}[37m"

C_BG_BLUE = f"{ESC}[44m"
C_BG_DARK = f"{ESC}[48;5;234m"

def rgb565_to_rgb888(c):
    """Converts 16-bit RGB565 integer to (r, g, b) 8-bit tuple."""
    r = ((c >> 11) & 0x1F) * 255 // 31
    g = ((c >> 5) & 0x3F) * 255 // 63
    b = (c & 0x1F) * 255 // 31
    return r, g, b

def render_halfblock_screen(framebuffer, src_w, src_h, out_cols, out_rows):
    """
    Renders a 2D RGB565 framebuffer into a list of terminal strings using
    Unicode upper-half block characters (▀) to achieve 2x vertical resolution.
    out_cols: number of terminal columns.
    out_rows: number of terminal character lines (height in pixels = out_rows * 2).
    """
    lines = []
    target_pixel_h = out_rows * 2

    for ty in range(out_rows):
        y_top = min(src_h - 1, int((ty * 2) * src_h / target_pixel_h))
        y_bot = min(src_h - 1, int((ty * 2 + 1) * src_h / target_pixel_h))
        row_top = framebuffer[y_top]
        row_bot = framebuffer[y_bot]

        line_chars = []
        for tx in range(out_cols):
            sx = min(src_w - 1, int(tx * src_w / out_cols))
            c_top = row_top[sx]
            c_bot = row_bot[sx]

            r1, g1, b1 = rgb565_to_rgb888(c_top)
            r2, g2, b2 = rgb565_to_rgb888(c_bot)

            # Foreground = top pixel, Background = bottom pixel
            line_chars.append(f"\033[38;2;{r1};{g1};{b1};48;2;{r2};{g2};{b2}m▀")

        line_chars.append("\033[0m")
        lines.append("".join(line_chars))

    return lines

class N2660GuiRunner:
    """
    Interactive Real-Time Terminal GUI for Nokia 2660 Flip emulator.
    Handles display rendering, keyboard input mapping, telemetry, and execution slicing.
    """

    KEY_BINDINGS = {
        '0': '0', '1': '1', '2': '2', '3': '3', '4': '4',
        '5': '5', '6': '6', '7': '7', '8': '8', '9': '9',
        '*': '*', '#': '#',
        '[': 'lsk',      # Left SoftKey
        ']': 'rsk',      # Right SoftKey
        'c': 'call',     # Call / Send
        'C': 'call',
        'x': 'end',      # End / Power
        'X': 'end',
        'e': 'end',
        'E': 'end',
        '\r': 'ok',      # Center / Select
        '\n': 'ok',
        ' ': 'ok',
        '!': 'sos',      # Rear Emergency Key
        '+': 'vol_up',   # Volume +
        '=': 'vol_up',
        '-': 'vol_down', # Volume -
        '_': 'vol_down',
    }

    def __init__(self, vm=None, entry_point=0x80000000, slice_insns=25000):
        self.vm = vm or T107UnicornVM(trace_instructions=False)
        self.entry_point = entry_point
        self.slice_insns = slice_insns
        self.is_running = True
        self.is_paused = False
        self.screen_mode = "main"   # "main", "sub", or "dual"
        self.active_keys = set()
        self.key_release_times = {}
        self.last_key_pressed = None
        self.key_flash_until = 0.0
        self.start_time = time.time()
        self.last_frame_time = time.time()
        self.last_insn_count = 0
        self.current_mips = 0.0
        self.status_msg = "Running"
        self.vm_initialized = False

    def init_vm(self):
        """Initializes CPU state if not already started."""
        if not self.vm_initialized:
            self.vm.init_cpu(self.entry_point)
            self.vm_initialized = True

    def handle_key_event(self, key_name, duration_sec=0.15):
        """Injects a Nokia keypad press event and schedules auto-release."""
        if key_name in NOKIA_KEYS:
            self.vm.inject_key(key_name, pressed=True)
            self.active_keys.add(key_name)
            self.key_release_times[key_name] = time.time() + duration_sec
            self.last_key_pressed = key_name
            self.key_flash_until = time.time() + duration_sec

    def toggle_flip(self):
        """Toggles the Hall effect flip sensor state."""
        current_state = bool(self.vm.gpio_data_in & 0x1)
        new_state = not current_state
        self.vm.set_flip_state(new_state)
        # Also switch default screen view to match flip state
        if new_state:
            self.status_msg = "Flip Opened -> Main Display"
            if self.screen_mode == "sub":
                self.screen_mode = "main"
        else:
            self.status_msg = "Flip Closed -> Outer Display"
            if self.screen_mode == "main":
                self.screen_mode = "sub"

    def process_key_releases(self):
        """Releases any keys whose press duration has elapsed."""
        now = time.time()
        to_remove = []
        for k, release_time in self.key_release_times.items():
            if now >= release_time:
                self.vm.inject_key(k, pressed=False)
                to_remove.append(k)
        for k in to_remove:
            self.active_keys.discard(k)
            del self.key_release_times[k]

    def step_execution(self, insn_count=None):
        """Executes a slice of instructions on the VM."""
        if not self.vm_initialized:
            self.init_vm()
        count = insn_count or self.slice_insns
        ok, err = self.vm.step(count=count, timeout_us=40000)
        now = time.time()
        dt = now - self.last_frame_time
        if dt >= 0.2:
            d_insn = self.vm.instruction_count - self.last_insn_count
            self.current_mips = (d_insn / dt) / 1000000.0 if dt > 0 else 0.0
            self.last_insn_count = self.vm.instruction_count
            self.last_frame_time = now
        if not ok:
            self.status_msg = f"Fault: {err}"
            self.is_paused = True
        return ok

    def render_keypad_cell(self, label, key_id, width=9):
        """Renders an individual keypad button box with active press highlighting."""
        is_active = (key_id in self.active_keys) or (self.last_key_pressed == key_id and time.time() < self.key_flash_until)
        text = label.center(width)
        if is_active:
            return f"{C_BOLD}{C_REV}{C_YELLOW}[{text}]{C_RESET}"
        return f"{C_DIM}[{C_RESET}{C_WHITE}{text}{C_RESET}{C_DIM}]{C_RESET}"

    def build_keypad_widget(self):
        """Builds terminal lines displaying the authentic Nokia 2660 keypad."""
        k = self.render_keypad_cell
        flip_open = bool(self.vm.gpio_data_in & 0x1)
        flip_str = f"{C_GREEN}OPEN{C_RESET}" if flip_open else f"{C_YELLOW}CLOSED{C_RESET}"

        lines = [
            f" ┌──────────────── Keypad Matrix ────────────────┐",
            f" │  {k('LSK [', 'lsk', 7)}   {k('UP ↑', 'up', 7)}   {k('RSK ]', 'rsk', 7)}   │",
            f" │  {k('CALL c', 'call', 7)}   {k('OK ↵', 'ok', 7)}   {k('END x', 'end', 7)}   │",
            f" │  {k('LEFT ←', 'left', 7)}               {k('RGHT →', 'right', 7)}   │",
            f" │  {k('1', '1', 7)}   {k('2 abc', '2', 7)}   {k('3 def', '3', 7)}   │",
            f" │  {k('4 ghi', '4', 7)}   {k('5 jkl', '5', 7)}   {k('6 mno', '6', 7)}   │",
            f" │  {k('7 pqrs', '7', 7)}   {k('8 tuv', '8', 7)}   {k('9 wxyz', '9', 7)}   │",
            f" │  {k('* +', '*', 7)}   {k('0 _', '0', 7)}   {k('# ⇧', '#', 7)}   │",
            f" │  Flip: [{flip_str}] (f)   {k('SOS !', 'sos', 7)}  │",
            f" └────────────────────────────────────────────────┘"
        ]
        return lines

    def build_telemetry_widget(self):
        """Builds hardware registers, INTC, Timer, Audio, and Console telemetry lines."""
        try:
            pc = self.vm.uc.reg_read(UC_ARM_REG_PC)
            sp = self.vm.uc.reg_read(UC_ARM_REG_SP)
            lr = self.vm.uc.reg_read(UC_ARM_REG_LR)
            cpsr = self.vm.uc.reg_read(UC_ARM_REG_CPSR)
            r0 = self.vm.uc.reg_read(UC_ARM_REG_R0)
            r1 = self.vm.uc.reg_read(UC_ARM_REG_R1)
            r2 = self.vm.uc.reg_read(UC_ARM_REG_R2)
            r3 = self.vm.uc.reg_read(UC_ARM_REG_R3)
        except Exception:
            pc, sp, lr, cpsr, r0, r1, r2, r3 = 0, 0, 0, 0, 0, 0, 0, 0

        mode_num = cpsr & 0x1F
        mode_str = {0x10: "USR", 0x12: "IRQ", 0x13: "SVC", 0x17: "ABT", 0x1B: "UND", 0x1F: "SYS"}.get(mode_num, hex(mode_num))
        state_str = f"{C_YELLOW}PAUSED{C_RESET}" if self.is_paused else f"{C_GREEN}RUNNING{C_RESET}"

        raw_intc = getattr(self.vm, 'intc_raw_sts', 0)
        mask_intc = self.vm.intc_get_irq_masked_status() if hasattr(self.vm, 'intc_get_irq_masked_status') else 0
        en_intc = getattr(self.vm, 'intc_irq_en', 0)

        # Active IRQ line name
        active_irq = "None"
        if mask_intc != 0:
            for irq_idx in range(32):
                if mask_intc & (1 << irq_idx):
                    names = {0: "TIMER", 1: "UART0", 4: "GPIO", 5: "KPD", 6: "SPI0", 7: "SPI1", 8: "SFC", 9: "AUDIO", 15: "SOFT"}
                    active_irq = f"{names.get(irq_idx, 'IRQ')}({irq_idx})"
                    break

        uart_tx_bytes = len(getattr(self.vm, 'uart0_tx_buffer', []))
        last_uart_text = ""
        if uart_tx_bytes > 0:
            raw = bytes(self.vm.uart0_tx_buffer[-240:])
            # Filter printable characters
            last_uart_text = "".join(chr(b) if 32 <= b <= 126 or b in (10, 13) else '.' for b in raw)

        lines = [
            f" ┌──────────────── Hardware Telemetry ───────────────┐",
            f" │ CPU: ARMv7-A Cortex-A7    State: {state_str:<18} │",
            f" │ PC:   0x{pc:08X}   SP:   0x{sp:08X}                 │",
            f" │ LR:   0x{lr:08X}   CPSR: 0x{cpsr:08X} [{mode_str:<3}]        │",
            f" │ R0:   0x{r0:08X}   R1:   0x{r1:08X}                 │",
            f" │ R2:   0x{r2:08X}   R3:   0x{r3:08X}                 │",
            f" │ Insns: {self.vm.instruction_count:<10,}  Speed: {self.current_mips:5.2f} MIPS          │",
            f" ├──────────────── Peripherals ───────────────────────┤",
            f" │ INTC Raw:  0x{raw_intc:08X}  Masked: 0x{mask_intc:08X}      │",
            f" │ INTC En:   0x{en_intc:08X}  Active: {active_irq:<14}  │",
            f" │ Timer Val: 0x{getattr(self.vm, 'timer_load', 0):08X}  Ctl: 0x{getattr(self.vm, 'timer_ctl', 0):02X}               │",
            f" │ UART0 TX:  {uart_tx_bytes:<7,} B   Audio Tones: {len(getattr(self.vm, 'audio_tone_events', [])):<6}  │",
            f" ├──────────────── Serial Console ────────────────────┤",
        ]

        # Extract last 4 console lines
        console_lines = [l.strip() for l in last_uart_text.splitlines() if l.strip()]
        recent_console = console_lines[-4:] if console_lines else ["[UART0 idle - waiting for firmware output]"]
        while len(recent_console) < 4:
            recent_console.insert(0, "")

        for cl in recent_console:
            truncated = (cl[:45] + "...") if len(cl) > 48 else cl
            lines.append(f" │ {C_CYAN}{truncated:<48}{C_RESET} │")

        lines.append(f" └────────────────────────────────────────────────────┘")
        return lines

    def render_frame(self):
        """Renders full dashboard frame (Displays + Keypad + Telemetry)."""
        # Determine screen dimension to render
        # Standard: 36 cols x 24 rows for main LCD (72x48 pixels)
        # Sub LCD:  24 cols x 16 rows for sub LCD (48x32 pixels)
        main_lines = render_halfblock_screen(self.vm.framebuffer, GC9307_WIDTH, GC9307_HEIGHT, out_cols=38, out_rows=22)
        sub_lines = render_halfblock_screen(self.vm.sub_framebuffer, ST7735_WIDTH, ST7735_HEIGHT, out_cols=24, out_rows=14)

        keypad_lines = self.build_keypad_widget()
        telemetry_lines = self.build_telemetry_widget()

        output = []
        # Header
        output.append(f"{C_BOLD}{C_BG_BLUE}{C_WHITE}  Nokia 2660 Flip Emulator (N2660Emu) - Interactive Terminal UI    {C_RESET}")

        # Screen display section
        if self.screen_mode == "dual":
            output.append(f"{C_BOLD} Main Display (240x320)                 Sub Display (120x160){C_RESET}")
            max_screen_rows = max(len(main_lines), len(sub_lines))
            for i in range(max_screen_rows):
                m_str = main_lines[i] if i < len(main_lines) else (" " * 38)
                s_str = sub_lines[i] if i < len(sub_lines) else (" " * 24)
                output.append(f" {m_str}    {s_str}")
        elif self.screen_mode == "sub":
            output.append(f"{C_BOLD} Nokia 2660 Outer Sub-Display (120x160 QQVGA ST7735) {C_RESET}")
            for l in sub_lines:
                output.append(f"   {l}")
        else: # "main"
            output.append(f"{C_BOLD} Nokia 2660 Main Display (240x320 QVGA GC9307) {C_RESET}")
            for l in main_lines:
                output.append(f"   {l}")

        # Combine Keypad (Left) and Telemetry (Right) side-by-side
        output.append("")
        max_rows = max(len(keypad_lines), len(telemetry_lines))
        for r in range(max_rows):
            k_part = keypad_lines[r] if r < len(keypad_lines) else (" " * 49)
            t_part = telemetry_lines[r] if r < len(telemetry_lines) else ""
            output.append(f"{k_part}  {t_part}")

        # Footer controls
        output.append(f"{C_DIM}───────────────────────────────────────────────────────────────────────────────{C_RESET}")
        output.append(f"{C_BOLD}Controls:{C_RESET} [0-9,*,#]: Keypad  [Arrows/Enter]: D-Pad  [c]: Call  [x]: End  [f]: Flip")
        output.append(f"          [v]: Mode (Main/Sub/Dual)  [p]: Pause  [t]: Step  [r]: Reset  [q]: Quit")
        output.append(f"{C_YELLOW}Status:{C_RESET} {self.status_msg}")

        return "\n".join(output)

    def run_interactive(self, target_fps=30):
        """Runs the interactive loop with raw non-blocking terminal input."""
        import termios
        import tty

        fd = sys.stdin.fileno()
        old_settings = termios.tcgetattr(fd)
        frame_interval = 1.0 / target_fps

        print(f"{ESC}[?1049h{ESC}[?25l{ESC}[2J{ESC}[H", end="", flush=True)  # Alternate buffer, hide cursor, clear

        try:
            tty.setcbreak(fd)
            self.init_vm()

            while self.is_running:
                frame_start = time.time()

                # 1. Read non-blocking keyboard input
                while select.select([sys.stdin], [], [], 0)[0]:
                    ch = sys.stdin.read(1)
                    if ch == '\x1b': # Escape sequence
                        # Check if more characters follow
                        if select.select([sys.stdin], [], [], 0.05)[0]:
                            seq = sys.stdin.read(2)
                            if seq == '[A':
                                self.handle_key_event('up')
                            elif seq == '[B':
                                self.handle_key_event('down')
                            elif seq == '[D':
                                self.handle_key_event('left')
                            elif seq == '[C':
                                self.handle_key_event('right')
                        else:
                            self.is_running = False
                            break
                    elif ch in ('q', 'Q', '\x03'):
                        self.is_running = False
                        break
                    elif ch in ('p', 'P'):
                        self.is_paused = not self.is_paused
                        self.status_msg = "Paused" if self.is_paused else "Running"
                    elif ch in ('t', 'T'):
                        if self.is_paused:
                            self.step_execution(1)
                            self.status_msg = "Stepped 1 instruction"
                    elif ch in ('s', 'S'):
                        if self.is_paused:
                            self.step_execution(1000)
                            self.status_msg = "Stepped 1,000 instructions"
                    elif ch in ('f', 'F'):
                        self.toggle_flip()
                    elif ch in ('v', 'V'):
                        modes = ["main", "sub", "dual"]
                        idx = (modes.index(self.screen_mode) + 1) % len(modes)
                        self.screen_mode = modes[idx]
                        self.status_msg = f"Screen View: {self.screen_mode.upper()}"
                    elif ch in ('r', 'R'):
                        self.init_vm()
                        self.status_msg = "VM Reset"
                    elif ch in self.KEY_BINDINGS:
                        k = self.KEY_BINDINGS[ch]
                        self.handle_key_event(k)

                # 2. Process key releases
                self.process_key_releases()

                # 3. Step VM execution if running
                if not self.is_paused and self.is_running:
                    self.step_execution()

                # 4. Render UI frame to terminal
                ui_str = self.render_frame()
                print(f"{ESC}[H{ui_str}", end="", flush=True)

                # 5. Maintain target frame rate
                elapsed = time.time() - frame_start
                if elapsed < frame_interval:
                    time.sleep(frame_interval - elapsed)

        finally:
            # Restore terminal attributes and show cursor
            termios.tcsetattr(fd, termios.TCSADRAIN, old_settings)
            print(f"{ESC}[?25h{ESC}[?1049l", end="", flush=True)
            print("[N2660Emu] Interactive GUI runner terminated cleanly.")

    def run_headless(self, duration_sec=1.0, script=None):
        """Runs the runner without interactive TTY for unit tests or scripted runs."""
        self.init_vm()
        start = time.time()
        step_idx = 0
        while time.time() - start < duration_sec and self.is_running:
            if script and step_idx < len(script):
                t_event, action, param = script[step_idx]
                if (time.time() - start) >= t_event:
                    if action == "key":
                        self.handle_key_event(param)
                    elif action == "flip":
                        self.toggle_flip()
                    step_idx += 1

            self.process_key_releases()
            if not self.is_paused:
                self.step_execution()
            time.sleep(0.01)
        return True

def main():
    parser = argparse.ArgumentParser(
        description="N2660Emu: Interactive Real-Time Terminal GUI for Nokia 2660 Flip",
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("binary", nargs="?", default=None, help="Path to raw firmware binary")
    parser.add_argument("--entry", type=lambda x: int(x, 0), default=0x80000000,
                        help="Entry address (default: 0x80000000)")
    parser.add_argument("--flip", choices=["open", "closed"], default="open",
                        help="Initial flip state (default: open)")
    parser.add_argument("--view", choices=["main", "sub", "dual"], default="main",
                        help="Initial display view (default: main)")
    parser.add_argument("--fps", type=int, default=30, help="Target UI refresh rate (default: 30)")
    parser.add_argument("--slice", type=int, default=25000, help="Instructions per frame slice (default: 25000)")
    parser.add_argument("--headless", action="store_true", help="Run in headless non-interactive mode")
    parser.add_argument("--timeout", type=float, default=2.0, help="Headless timeout in seconds (default: 2.0)")

    args = parser.parse_args()

    bin_path = args.binary
    if not bin_path:
        # Check default test firmware
        default_fw = os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
        if os.path.isfile(default_fw):
            bin_path = default_fw
        else:
            print("Error: No firmware binary specified and default test_firmware.bin not found.")
            sys.exit(1)

    vm = T107UnicornVM(trace_instructions=False)
    vm.set_flip_state(args.flip == "open")
    vm.load_binary(bin_path, args.entry)

    runner = N2660GuiRunner(vm=vm, entry_point=args.entry, slice_insns=args.slice)
    runner.screen_mode = args.view

    if args.headless or not sys.stdin.isatty():
        print(f"[N2660Emu GUI] Running in headless mode for {args.timeout} seconds...")
        runner.run_headless(duration_sec=args.timeout)
        print("[N2660Emu GUI] Headless run completed successfully.")
        sys.exit(0)
    else:
        runner.run_interactive(target_fps=args.fps)

if __name__ == "__main__":
    main()
