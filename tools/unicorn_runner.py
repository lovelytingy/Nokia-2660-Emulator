#!/usr/bin/env python3
"""
N2660Emu Unicorn Execution Runner
Implements Unisoc T107 (UMS9117) memory map and SPRD UART MMIO in Unicorn.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import sys
import os
import argparse

for pyver in ["3.9", "3.10", "3.11", "3.12", "3.13", "3.14", "3.15"]:
    p = os.path.expanduser(f"~/Library/Python/{pyver}/lib/python/site-packages")
    if os.path.isdir(p) and p not in sys.path:
        sys.path.append(p)

from unicorn import (
    Uc, UC_ARCH_ARM, UC_MODE_ARM, UC_HOOK_CODE, UC_HOOK_MEM_READ,
    UC_HOOK_MEM_WRITE, UC_HOOK_MEM_UNMAPPED, UC_MEM_WRITE, UC_MEM_READ,
    UcError
)
from unicorn.arm_const import (
    UC_ARM_REG_R0, UC_ARM_REG_R1, UC_ARM_REG_R2, UC_ARM_REG_R3,
    UC_ARM_REG_R4, UC_ARM_REG_R5, UC_ARM_REG_SP, UC_ARM_REG_LR,
    UC_ARM_REG_PC, UC_ARM_REG_CPSR
)
import capstone

# T107 Memory Map Constants
SRAM_BASE  = 0x00000000
SRAM_SIZE  = 64 * 1024           # 64 KB Internal SRAM

DRAM_BASE  = 0x80000000
DRAM_SIZE  = 48 * 1024 * 1024    # 48 MB System DRAM

MMIO_BASE  = 0x40000000
MMIO_SIZE  = 0x20000000          # 512 MB MMIO Space

SPRD_UART0_BASE = 0x50000000
SPRD_TXD_REG    = 0x0000
SPRD_RXD_REG    = 0x0004
SPRD_STS0_REG   = 0x0008
SPRD_STS1_REG   = 0x000C

SPRD_TIMERS_BASE = 0x50030000
TIMER_LOAD_LO    = 0x0000
TIMER_LOAD_HI    = 0x0004
TIMER_VALUE_LO   = 0x0008
TIMER_CTL        = 0x0010
TIMER_INT        = 0x0014
TIMER_SHDW_LO    = 0x0018

SPRD_GPIO_BASE   = 0x50040000
GPIO_DATA        = 0x0000
GPIO_DMSK        = 0x0004
GPIO_DIR         = 0x0008
GPIO_INEN        = 0x0028

SPRD_KPD_BASE    = 0x50060000
KPD_CTRL         = 0x0000
KPD_INT_EN       = 0x0004
KPD_INT_RAW      = 0x0008
KPD_INT_MASK     = 0x000C
KPD_INT_CLR      = 0x0010
KPD_POLARITY     = 0x0014
KPD_CLK_DIV      = 0x0018
KPD_STATUS0      = 0x001C
KPD_STATUS1      = 0x0020

# Nokia 2660 Key Map (Key code -> Bit offset)
NOKIA_KEYS = {
    '0': 0, '1': 1, '2': 2, '3': 3, '4': 4,
    '5': 5, '6': 6, '7': 7, '8': 8, '9': 9,
    '*': 10, 'star': 10,
    '#': 11, 'hash': 11,
    'up': 12, 'down': 13, 'left': 14, 'right': 15,
    'ok': 16, 'center': 16,
    'lsk': 17, 'rsk': 18,
    'call': 19, 'end': 20, 'power': 20,
    'sos': 21,
    'vol_up': 22, 'vol_down': 23,
}

SPRD_SPI0_BASE   = 0x50080000
SPRD_SPI1_BASE   = 0x50090000
SPI_TXD          = 0x0000
SPI_CLKD         = 0x0004
SPI_CTL0         = 0x0008
SPI_CTL1         = 0x000C
SPI_CTL4         = 0x0018
SPI_STS1         = 0x0030

GC9307_WIDTH     = 240
GC9307_HEIGHT    = 320

ST7735_WIDTH     = 120
ST7735_HEIGHT    = 160

PL011_UART_BASE = 0x09000000

class T107UnicornVM:
    def __init__(self, trace_instructions=False, max_instructions=0):
        self.trace_instructions = trace_instructions
        self.max_instructions = max_instructions
        self.instruction_count = 0
        self.cs = capstone.Cs(capstone.CS_ARCH_ARM, capstone.CS_MODE_ARM)

        # Initialize Unicorn ARM emulator
        self.uc = Uc(UC_ARCH_ARM, UC_MODE_ARM)

        # Timer and GPIO peripheral state
        self.timer_load = 0
        self.timer_ctl = 0
        self.timer_int = 0
        self.timer_start_insn = 0

        self.gpio_data_out = 0
        self.gpio_data_in = 0x00000001  # Default Pin 0 high (e.g. flip open)
        self.gpio_dir = 0
        self.gpio_inen = 0

        # Keypad Matrix State
        self.kpd_ctrl = 0
        self.kpd_int_en = 0
        self.kpd_int_raw = 0
        self.kpd_polarity = 0
        self.kpd_clk_div = 0
        self.kpd_status0 = 0
        self.kpd_status1 = 0

        # GalaxyCore GC9307 LCD Display State (240x320 RGB565)
        self.framebuffer = [[0 for _ in range(GC9307_WIDTH)] for _ in range(GC9307_HEIGHT)]
        self.lcd_sleep = True
        self.lcd_on = False
        self.lcd_col_start = 0
        self.lcd_col_end = GC9307_WIDTH - 1
        self.lcd_row_start = 0
        self.lcd_row_end = GC9307_HEIGHT - 1
        self.lcd_cur_x = 0
        self.lcd_cur_y = 0
        self.lcd_state = 'CMD'
        self.lcd_cmd = 0
        self.lcd_params = []
        self.lcd_expected_params = 0
        self.lcd_pixel_high = None
        self.spi_dc_mode = False  # False = Command, True = Data

        # Sitronix ST7735 Outer Sub-Display State (120x160 RGB565)
        self.sub_framebuffer = [[0 for _ in range(ST7735_WIDTH)] for _ in range(ST7735_HEIGHT)]
        self.sub_lcd_sleep = True
        self.sub_lcd_on = False
        self.sub_lcd_col_start = 0
        self.sub_lcd_col_end = ST7735_WIDTH - 1
        self.sub_lcd_row_start = 0
        self.sub_lcd_row_end = ST7735_HEIGHT - 1
        self.sub_lcd_cur_x = 0
        self.sub_lcd_cur_y = 0
        self.sub_lcd_state = 'CMD'
        self.sub_lcd_cmd = 0
        self.sub_lcd_params = []
        self.sub_lcd_expected_params = 0
        self.sub_lcd_pixel_high = None
        self.sub_spi_dc_mode = False

        # Map Memory Regions
        # 1. Internal SRAM
        self.uc.mem_map(SRAM_BASE, SRAM_SIZE)

        # 2. DRAM (map 48 MB, or 16 MB initial window for speed)
        self.dram_map_size = 16 * 1024 * 1024
        self.uc.mem_map(DRAM_BASE, self.dram_map_size)

        # 3. Peripheral MMIO space (map pages for SPRD UART, Timers, GPIO, KPD, SPI0, SPI1, PL011)
        self.uc.mem_map(SPRD_UART0_BASE, 0x1000)
        self.uc.mem_map(SPRD_TIMERS_BASE, 0x1000)
        self.uc.mem_map(SPRD_GPIO_BASE, 0x1000)
        self.uc.mem_map(SPRD_KPD_BASE, 0x1000)
        self.uc.mem_map(SPRD_SPI0_BASE, 0x1000)
        self.uc.mem_map(SPRD_SPI1_BASE, 0x1000)
        self.uc.mem_map(PL011_UART_BASE, 0x1000)
        # Also map page around 0x40001000 to test unmapped traps
        self.uc.mem_map(0x40001000 & ~0xFFF, 0x1000)

        # Register Hooks
        self.uc.hook_add(UC_HOOK_MEM_WRITE, self._hook_mem_write)
        self.uc.hook_add(UC_HOOK_MEM_READ, self._hook_mem_read)
        self.uc.hook_add(UC_HOOK_MEM_UNMAPPED, self._hook_mem_unmapped)
        self.uc.hook_add(UC_HOOK_CODE, self._hook_code)

    def _hook_code(self, uc, address, size, user_data):
        self.instruction_count += 1
        if self.trace_instructions:
            code = uc.mem_read(address, size)
            for insn in self.cs.disasm(code, address):
                print(f"[TRACE] 0x{insn.address:08X}: {insn.mnemonic:8s} {insn.op_str}")

        if self.max_instructions > 0 and self.instruction_count >= self.max_instructions:
            print(f"[VM] Instruction limit ({self.max_instructions}) reached. Stopping.")
            uc.emu_stop()

    def _hook_mem_write(self, uc, access, address, size, value, user_data):
        # 1. SPRD UART0 TXD
        if address == (SPRD_UART0_BASE + SPRD_TXD_REG):
            char = chr(value & 0xFF)
            try:
                sys.stdout.write(char)
                sys.stdout.flush()
            except BrokenPipeError:
                pass
            return

        # 2. PL011 UART fallback
        if address == PL011_UART_BASE:
            return

        # 3. SPRD System Timer
        if SPRD_TIMERS_BASE <= address < (SPRD_TIMERS_BASE + 0x1000):
            offset = address - SPRD_TIMERS_BASE
            if offset == TIMER_LOAD_LO:
                self.timer_load = value & 0xFFFFFFFF
                self.timer_start_insn = self.instruction_count
            elif offset == TIMER_CTL:
                self.timer_ctl = value & 0xFFFFFFFF
                self.timer_start_insn = self.instruction_count
            elif offset == TIMER_INT:
                self.timer_int = value & 0xFFFFFFFF
            return

        # 4. SPRD GPIO
        if SPRD_GPIO_BASE <= address < (SPRD_GPIO_BASE + 0x1000):
            offset = address - SPRD_GPIO_BASE
            if offset == GPIO_DATA:
                self.gpio_data_out = value & 0xFFFFFFFF
            elif offset == GPIO_DIR:
                self.gpio_dir = value & 0xFFFFFFFF
            elif offset == GPIO_INEN:
                self.gpio_inen = value & 0xFFFFFFFF
            return

        # 5. SPRD SPI0 / GalaxyCore GC9307 Display
        if SPRD_SPI0_BASE <= address < (SPRD_SPI0_BASE + 0x1000):
            offset = address - SPRD_SPI0_BASE
            if offset == SPI_CTL4:
                self.spi_dc_mode = bool(value & 0x1)
            elif offset == SPI_TXD:
                byte = value & 0xFF
                if not self.spi_dc_mode:
                    # Command Mode
                    self.lcd_cmd = byte
                    self.lcd_params = []
                    self.lcd_pixel_high = None
                    if byte == 0x01: # SWRESET
                        self.framebuffer = [[0 for _ in range(GC9307_WIDTH)] for _ in range(GC9307_HEIGHT)]
                        self.lcd_col_start, self.lcd_col_end = 0, GC9307_WIDTH - 1
                        self.lcd_row_start, self.lcd_row_end = 0, GC9307_HEIGHT - 1
                    elif byte == 0x11: # SLPOUT
                        self.lcd_sleep = False
                    elif byte == 0x29: # DISPON
                        self.lcd_on = True
                    elif byte == 0x2A: # CASET
                        self.lcd_state = 'PARAM'
                        self.lcd_expected_params = 4
                    elif byte == 0x2B: # RASET
                        self.lcd_state = 'PARAM'
                        self.lcd_expected_params = 4
                    elif byte == 0x2C: # RAMWR
                        self.lcd_state = 'PIXELS'
                        self.lcd_cur_x = self.lcd_col_start
                        self.lcd_cur_y = self.lcd_row_start
                    elif byte in (0x3A, 0x36):
                        self.lcd_state = 'PARAM'
                        self.lcd_expected_params = 1
                else:
                    # Data Mode
                    if self.lcd_state == 'PARAM':
                        self.lcd_params.append(byte)
                        if len(self.lcd_params) >= self.lcd_expected_params:
                            if self.lcd_cmd == 0x2A:
                                self.lcd_col_start = min((self.lcd_params[0] << 8) | self.lcd_params[1], GC9307_WIDTH - 1)
                                self.lcd_col_end   = min((self.lcd_params[2] << 8) | self.lcd_params[3], GC9307_WIDTH - 1)
                            elif self.lcd_cmd == 0x2B:
                                self.lcd_row_start = min((self.lcd_params[0] << 8) | self.lcd_params[1], GC9307_HEIGHT - 1)
                                self.lcd_row_end   = min((self.lcd_params[2] << 8) | self.lcd_params[3], GC9307_HEIGHT - 1)
                            self.lcd_state = 'CMD'
                    elif self.lcd_state == 'PIXELS':
                        if self.lcd_pixel_high is None:
                            self.lcd_pixel_high = byte
                        else:
                            color = (self.lcd_pixel_high << 8) | byte
                            self.lcd_pixel_high = None
                            if 0 <= self.lcd_cur_y < GC9307_HEIGHT and 0 <= self.lcd_cur_x < GC9307_WIDTH:
                                self.framebuffer[self.lcd_cur_y][self.lcd_cur_x] = color
                            self.lcd_cur_x += 1
                            if self.lcd_cur_x > self.lcd_col_end:
                                self.lcd_cur_x = self.lcd_col_start
                                self.lcd_cur_y += 1
                                if self.lcd_cur_y > self.lcd_row_end:
                                    self.lcd_cur_y = self.lcd_row_start
            return

        # 5. SPRD Keypad Matrix Controller
        if SPRD_KPD_BASE <= address < (SPRD_KPD_BASE + 0x1000):
            offset = address - SPRD_KPD_BASE
            if offset == KPD_CTRL:
                self.kpd_ctrl = value & 0xFFFFFFFF
            elif offset == KPD_INT_EN:
                self.kpd_int_en = value & 0xFFFFFFFF
            elif offset == KPD_INT_CLR:
                if value & 0x1:
                    self.kpd_int_raw &= ~0x1
            elif offset == KPD_POLARITY:
                self.kpd_polarity = value & 0xFFFFFFFF
            elif offset == KPD_CLK_DIV:
                self.kpd_clk_div = value & 0xFFFFFFFF
            return

        # 6. SPRD SPI1 / Sitronix ST7735 Outer Sub-Display (120x160)
        if SPRD_SPI1_BASE <= address < (SPRD_SPI1_BASE + 0x1000):
            offset = address - SPRD_SPI1_BASE
            if offset == SPI_CTL4:
                self.sub_spi_dc_mode = bool(value & 0x1)
            elif offset == SPI_TXD:
                byte = value & 0xFF
                if not self.sub_spi_dc_mode:
                    # Command Mode
                    self.sub_lcd_cmd = byte
                    self.sub_lcd_params = []
                    self.sub_lcd_pixel_high = None
                    if byte == 0x01: # SWRESET
                        self.sub_framebuffer = [[0 for _ in range(ST7735_WIDTH)] for _ in range(ST7735_HEIGHT)]
                        self.sub_lcd_col_start, self.sub_lcd_col_end = 0, ST7735_WIDTH - 1
                        self.sub_lcd_row_start, self.sub_lcd_row_end = 0, ST7735_HEIGHT - 1
                    elif byte == 0x11: # SLPOUT
                        self.sub_lcd_sleep = False
                    elif byte == 0x29: # DISPON
                        self.sub_lcd_on = True
                    elif byte == 0x2A: # CASET
                        self.sub_lcd_state = 'PARAM'
                        self.sub_lcd_expected_params = 4
                    elif byte == 0x2B: # RASET
                        self.sub_lcd_state = 'PARAM'
                        self.sub_lcd_expected_params = 4
                    elif byte == 0x2C: # RAMWR
                        self.sub_lcd_state = 'PIXELS'
                        self.sub_lcd_cur_x = self.sub_lcd_col_start
                        self.sub_lcd_cur_y = self.sub_lcd_row_start
                    elif byte in (0x3A, 0x36):
                        self.sub_lcd_state = 'PARAM'
                        self.sub_lcd_expected_params = 1
                else:
                    # Data Mode
                    if self.sub_lcd_state == 'PARAM':
                        self.sub_lcd_params.append(byte)
                        if len(self.sub_lcd_params) >= self.sub_lcd_expected_params:
                            if self.sub_lcd_cmd == 0x2A:
                                self.sub_lcd_col_start = min((self.sub_lcd_params[0] << 8) | self.sub_lcd_params[1], ST7735_WIDTH - 1)
                                self.sub_lcd_col_end   = min((self.sub_lcd_params[2] << 8) | self.sub_lcd_params[3], ST7735_WIDTH - 1)
                            elif self.sub_lcd_cmd == 0x2B:
                                self.sub_lcd_row_start = min((self.sub_lcd_params[0] << 8) | self.sub_lcd_params[1], ST7735_HEIGHT - 1)
                                self.sub_lcd_row_end   = min((self.sub_lcd_params[2] << 8) | self.sub_lcd_params[3], ST7735_HEIGHT - 1)
                            self.sub_lcd_state = 'CMD'
                    elif self.sub_lcd_state == 'PIXELS':
                        if self.sub_lcd_pixel_high is None:
                            self.sub_lcd_pixel_high = byte
                        else:
                            color = (self.sub_lcd_pixel_high << 8) | byte
                            self.sub_lcd_pixel_high = None
                            if 0 <= self.sub_lcd_cur_y < ST7735_HEIGHT and 0 <= self.sub_lcd_cur_x < ST7735_WIDTH:
                                self.sub_framebuffer[self.sub_lcd_cur_y][self.sub_lcd_cur_x] = color
                            self.sub_lcd_cur_x += 1
                            if self.sub_lcd_cur_x > self.sub_lcd_col_end:
                                self.sub_lcd_cur_x = self.sub_lcd_col_start
                                self.sub_lcd_cur_y += 1
                                if self.sub_lcd_cur_y > self.sub_lcd_row_end:
                                    self.sub_lcd_cur_y = self.sub_lcd_row_start
            return

        # 7. Catch-all unknown peripheral writes
        if 0x40000000 <= address < 0x60000000:
            pc = uc.reg_read(UC_ARM_REG_PC)
            print(f"\n[N2660Emu MMIO TRAP] Unknown peripheral WRITE at 0x{address:08X} = 0x{value:08X} (size {size}) from PC 0x{pc:08X}")

    def _hook_mem_read(self, uc, access, address, size, value, user_data):
        # 1. SPRD UART0 Status
        if address == (SPRD_UART0_BASE + SPRD_STS1_REG):
            uc.mem_write(address, (1 << 8).to_bytes(4, 'little'))
            return

        # 2. SPRD System Timer
        if SPRD_TIMERS_BASE <= address < (SPRD_TIMERS_BASE + 0x1000):
            offset = address - SPRD_TIMERS_BASE
            if offset in (TIMER_VALUE_LO, TIMER_SHDW_LO):
                if self.timer_ctl & 2:  # Timer enabled
                    elapsed = (self.instruction_count - self.timer_start_insn) * 4
                    val = max(0, self.timer_load - elapsed) & 0xFFFFFFFF
                else:
                    val = self.timer_load & 0xFFFFFFFF
                uc.mem_write(address, val.to_bytes(4, 'little'))
                return
            elif offset == TIMER_LOAD_LO:
                uc.mem_write(address, (self.timer_load & 0xFFFFFFFF).to_bytes(4, 'little'))
                return
            elif offset == TIMER_CTL:
                uc.mem_write(address, (self.timer_ctl & 0xFFFFFFFF).to_bytes(4, 'little'))
                return
            elif offset == TIMER_INT:
                uc.mem_write(address, (self.timer_int & 0xFFFFFFFF).to_bytes(4, 'little'))
                return

        # 3. SPRD GPIO
        if SPRD_GPIO_BASE <= address < (SPRD_GPIO_BASE + 0x1000):
            offset = address - SPRD_GPIO_BASE
            if offset == GPIO_DATA:
                val = ((self.gpio_data_out & self.gpio_dir) |
                       (self.gpio_data_in & ~self.gpio_dir)) & 0xFFFFFFFF
                uc.mem_write(address, val.to_bytes(4, 'little'))
                return
            elif offset == GPIO_DIR:
                uc.mem_write(address, (self.gpio_dir & 0xFFFFFFFF).to_bytes(4, 'little'))
                return
            elif offset == GPIO_INEN:
                uc.mem_write(address, (self.gpio_inen & 0xFFFFFFFF).to_bytes(4, 'little'))
                return

        # 4. SPRD Keypad Matrix Controller
        if SPRD_KPD_BASE <= address < (SPRD_KPD_BASE + 0x1000):
            offset = address - SPRD_KPD_BASE
            if offset == KPD_CTRL:
                val = self.kpd_ctrl
            elif offset == KPD_INT_EN:
                val = self.kpd_int_en
            elif offset == KPD_INT_RAW:
                val = self.kpd_int_raw
            elif offset == KPD_INT_MASK:
                val = self.kpd_int_raw & self.kpd_int_en
            elif offset == KPD_POLARITY:
                val = self.kpd_polarity
            elif offset == KPD_CLK_DIV:
                val = self.kpd_clk_div
            elif offset == KPD_STATUS0:
                val = self.kpd_status0
            elif offset == KPD_STATUS1:
                val = self.kpd_status1
            else:
                val = 0
            uc.mem_write(address, (val & 0xFFFFFFFF).to_bytes(4, 'little'))
            return

        # 5. SPRD SPI0 Controller Status
        if SPRD_SPI0_BASE <= address < (SPRD_SPI0_BASE + 0x1000):
            offset = address - SPRD_SPI0_BASE
            if offset == SPI_STS1:
                # Bit 8 = 1 (TX FIFO Empty / ready)
                uc.mem_write(address, (1 << 8).to_bytes(4, 'little'))
                return

        # 6. SPRD SPI1 Controller Status (Sub-Display)
        if SPRD_SPI1_BASE <= address < (SPRD_SPI1_BASE + 0x1000):
            offset = address - SPRD_SPI1_BASE
            if offset == SPI_STS1:
                # Bit 8 = 1 (TX FIFO Empty / ready)
                uc.mem_write(address, (1 << 8).to_bytes(4, 'little'))
                return

        # 4. Catch-all unknown peripheral reads
        if 0x40000000 <= address < 0x60000000:
            pc = uc.reg_read(UC_ARM_REG_PC)
            print(f"\n[N2660Emu MMIO TRAP] Unknown peripheral READ at 0x{address:08X} (size {size}) from PC 0x{pc:08X}")

    def _hook_mem_unmapped(self, uc, access, address, size, value, user_data):
        pc = uc.reg_read(UC_ARM_REG_PC)
        access_type = "WRITE to" if access == UC_MEM_WRITE else "READ from"
        print(f"\n[N2660Emu FAULT] Unmapped memory access: {access_type} 0x{address:08X} (size {size}) at PC 0x{pc:08X}")
        return False

    def load_binary(self, binary_path, load_addr):
        with open(binary_path, "rb") as f:
            data = f.read()
        print(f"[VM] Loading binary '{binary_path}' ({len(data)} bytes) to address 0x{load_addr:08X}...")
        self.uc.mem_write(load_addr, data)

    def run(self, entry_point, end_address=None, timeout_sec=10):
        # Setup Cortex-A7 initial registers
        # CPSR: Supervisor mode (0x13), IRQ/FIQ disabled (0xD3)
        self.uc.reg_write(UC_ARM_REG_CPSR, 0xD3)
        self.uc.reg_write(UC_ARM_REG_PC, entry_point)

        print(f"[VM] Starting execution at PC = 0x{entry_point:08X}...")
        try:
            self.uc.emu_start(entry_point, end_address or 0xFFFFFFFF, timeout=timeout_sec * 1000000)
        except UcError as e:
            pc = self.uc.reg_read(UC_ARM_REG_PC)
            sp = self.uc.reg_read(UC_ARM_REG_SP)
            lr = self.uc.reg_read(UC_ARM_REG_LR)
            r0 = self.uc.reg_read(UC_ARM_REG_R0)
            print(f"\n[VM Error] Emulation error: {e}")
            print(f"[Registers] PC=0x{pc:08X} SP=0x{sp:08X} LR=0x{lr:08X} R0=0x{r0:08X}")
            return False

        print("[VM] Emulation finished cleanly.")
        return True

    def inject_key(self, key_name, pressed=True):
        """Simulate pressing or releasing a key on the Nokia 2660 keypad matrix."""
        key_id = NOKIA_KEYS.get(str(key_name).lower())
        if key_id is None:
            raise ValueError(f"Unknown key name '{key_name}'. Supported: {list(NOKIA_KEYS.keys())}")
        bit = 1 << (key_id % 16)
        if key_id < 16:
            if pressed:
                self.kpd_status0 |= bit
            else:
                self.kpd_status0 &= ~bit
        else:
            if pressed:
                self.kpd_status1 |= bit
            else:
                self.kpd_status1 &= ~bit

        if pressed:
            self.kpd_int_raw |= 0x1

    def set_flip_state(self, is_open: bool):
        """Set Hall effect flip sensor state (GPIO Bank 0 Pin 0: 1=Open, 0=Closed)."""
        if is_open:
            self.gpio_data_in |= 0x00000001
        else:
            self.gpio_data_in &= ~0x00000001

    def export_ppm(self, file_path):
        """Exports 240x320 framebuffer as standard PPM image."""
        os.makedirs(os.path.dirname(os.path.abspath(file_path)), exist_ok=True)
        with open(file_path, "wb") as f:
            header = f"P6\n{GC9307_WIDTH} {GC9307_HEIGHT}\n255\n"
            f.write(header.encode('ascii'))
            for y in range(GC9307_HEIGHT):
                row = bytearray()
                for x in range(GC9307_WIDTH):
                    c = self.framebuffer[y][x]
                    # Convert RGB565 to RGB888
                    r = ((c >> 11) & 0x1F) * 255 // 31
                    g = ((c >> 5) & 0x3F) * 255 // 63
                    b = (c & 0x1F) * 255 // 31
                    row.extend([r, g, b])
                f.write(row)
        print(f"[N2660Emu Display] Saved framebuffer image to '{file_path}' ({GC9307_WIDTH}x{GC9307_HEIGHT})")

    def render_terminal_preview(self):
        """Renders downscaled 24-bit ANSI truecolor screen preview in terminal."""
        print("\n=== Nokia 2660 Flip Main Screen Preview (240x320 QVGA) ===")
        # 60 cols x 40 rows
        for y in range(0, GC9307_HEIGHT, 8):
            line = ""
            for x in range(0, GC9307_WIDTH, 4):
                c = self.framebuffer[y][x]
                r = ((c >> 11) & 0x1F) * 255 // 31
                g = ((c >> 5) & 0x3F) * 255 // 63
                b = (c & 0x1F) * 255 // 31
                line += f"\033[48;2;{r};{g};{b}m \033[0m"
            print(line)
        print("=" * 60)

    def export_sub_ppm(self, file_path):
        """Exports 120x160 secondary sub-display framebuffer as standard PPM image."""
        os.makedirs(os.path.dirname(os.path.abspath(file_path)), exist_ok=True)
        with open(file_path, "wb") as f:
            header = f"P6\n{ST7735_WIDTH} {ST7735_HEIGHT}\n255\n"
            f.write(header.encode('ascii'))
            for y in range(ST7735_HEIGHT):
                row = bytearray()
                for x in range(ST7735_WIDTH):
                    c = self.sub_framebuffer[y][x]
                    # Convert RGB565 to RGB888
                    r = ((c >> 11) & 0x1F) * 255 // 31
                    g = ((c >> 5) & 0x3F) * 255 // 63
                    b = (c & 0x1F) * 255 // 31
                    row.extend([r, g, b])
                f.write(row)
        print(f"[N2660Emu Sub-Display] Saved outer screen image to '{file_path}' ({ST7735_WIDTH}x{ST7735_HEIGHT})")

    def render_sub_terminal_preview(self):
        """Renders downscaled 24-bit ANSI truecolor preview of outer sub-display."""
        print("\n=== Nokia 2660 Flip Outer Sub-Screen Preview (120x160 QQVGA) ===")
        # 30 cols x 20 rows
        for y in range(0, ST7735_HEIGHT, 8):
            line = ""
            for x in range(0, ST7735_WIDTH, 4):
                c = self.sub_framebuffer[y][x]
                r = ((c >> 11) & 0x1F) * 255 // 31
                g = ((c >> 5) & 0x3F) * 255 // 63
                b = (c & 0x1F) * 255 // 31
                line += f"\033[48;2;{r};{g};{b}m \033[0m"
            print(line)
        print("=" * 60)

def main():
    parser = argparse.ArgumentParser(description="N2660Emu Unicorn VM Runner")
    parser.add_argument("binary", help="Path to raw binary file to execute")
    parser.add_argument("--entry", type=lambda x: int(x, 0), default=0x80000000,
                        help="Execution entry point address (default: 0x80000000)")
    parser.add_argument("--trace", action="store_true", help="Enable instruction tracing")
    parser.add_argument("--max-insns", type=int, default=0, help="Maximum instructions to execute")
    parser.add_argument("--timeout", type=int, default=5, help="Timeout in seconds")
    parser.add_argument("--screen-dump", type=str, default=None, help="Path to dump main framebuffer image (.ppm)")
    parser.add_argument("--sub-screen-dump", type=str, default=None, help="Path to dump outer sub-framebuffer image (.ppm)")
    parser.add_argument("--preview-screen", action="store_true", help="Render ANSI truecolor main screen in terminal")
    parser.add_argument("--preview-sub-screen", action="store_true", help="Render ANSI truecolor outer sub-screen in terminal")
    parser.add_argument("--key", type=str, action="append", help="Inject key press (e.g. --key 5 or --key ok)")
    parser.add_argument("--flip", choices=["open", "closed"], default="open", help="Hall effect flip sensor initial state (default: open)")
    args = parser.parse_args()

    if not os.path.isfile(args.binary):
        print(f"Error: Binary file '{args.binary}' not found.")
        sys.exit(1)

    vm = T107UnicornVM(trace_instructions=args.trace, max_instructions=args.max_insns)
    vm.set_flip_state(args.flip == "open")

    if args.key:
        for k in args.key:
            vm.inject_key(k, pressed=True)

    vm.load_binary(args.binary, args.entry)
    success = vm.run(entry_point=args.entry, timeout_sec=args.timeout)

    if args.screen_dump:
        vm.export_ppm(args.screen_dump)

    if args.sub_screen_dump:
        vm.export_sub_ppm(args.sub_screen_dump)

    if args.preview_screen:
        vm.render_terminal_preview()

    if args.preview_sub_screen:
        vm.render_sub_terminal_preview()

    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
