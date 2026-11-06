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
import collections
import struct
import math
import wave

try:
    from tools.bsl_engine import (
        BslServer, BslClient, decode_packet, encode_packet,
        get_cmd_name, BSL_CMD_CONNECT, BSL_CMD_START_DATA,
        BSL_CMD_MIDST_DATA, BSL_CMD_END_DATA, BSL_CMD_EXEC_DATA
    )
except ImportError:
    from bsl_engine import (
        BslServer, BslClient, decode_packet, encode_packet,
        get_cmd_name, BSL_CMD_CONNECT, BSL_CMD_START_DATA,
        BSL_CMD_MIDST_DATA, BSL_CMD_END_DATA, BSL_CMD_EXEC_DATA
    )

try:
    from tools.mksprd_img import (
        SPRD_BOOT0_MAGIC, SPRD_BOOT0_MAGIC_BIMG, create_boot0_image, BLOCK_SIZE
    )
except ImportError:
    from mksprd_img import (
        SPRD_BOOT0_MAGIC, SPRD_BOOT0_MAGIC_BIMG, create_boot0_image, BLOCK_SIZE
    )

try:
    from tools.atags import (
        create_standard_n2660_atags, DEFAULT_CMDLINE, DEFAULT_BOARD_ID, DEFAULT_ATAGS_ADDR
    )
except ImportError:
    from atags import (
        create_standard_n2660_atags, DEFAULT_CMDLINE, DEFAULT_BOARD_ID, DEFAULT_ATAGS_ADDR
    )

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

SPRD_SFC_BASE        = 0x500A0000
SFC_CMD              = 0x0000
SFC_ADDR             = 0x0004
SFC_DATA             = 0x0008
SFC_CTL              = 0x000C
SFC_STS              = 0x0010
SFC_BUF_PTR          = 0x0014
SFC_INT_EN           = 0x0018
SFC_INT_CLR          = 0x001C

FLASH_DIRECT_BASE    = 0x60000000
FLASH_DIRECT_SIZE    = 16 * 1024 * 1024 # 16 MB mapped direct window
FLASH_TOTAL_SIZE     = 128 * 1024 * 1024 # 128 MB full flash size

NAND_PAGE_SIZE       = 2048
NAND_SPARE_SIZE      = 64
NAND_PAGE_TOTAL_SIZE = 2112
NAND_PAGES_PER_BLOCK = 64
NAND_BLOCK_SIZE      = 64 * 2048 # 128 KB
NAND_BLOCK_COUNT     = 1024

NAND_CMD_RESET         = 0xFF
NAND_CMD_READ_ID       = 0x9F
NAND_CMD_PAGE_READ     = 0x13
NAND_CMD_READ_CACHE    = 0x03
NAND_CMD_READ_FAST     = 0x0B
NAND_CMD_WRITE_ENABLE  = 0x06
NAND_CMD_WRITE_DISABLE = 0x04
NAND_CMD_PROGRAM_LOAD  = 0x02
NAND_CMD_PROGRAM_EXEC  = 0x10
NAND_CMD_BLOCK_ERASE   = 0xD8
NAND_CMD_GET_FEATURE   = 0x0F
NAND_CMD_SET_FEATURE   = 0x1F

JEDEC_MFR_WINBOND      = 0xEF
JEDEC_DEV_W25N01GV     = 0xAA21

SPRD_AUDIO_BASE        = 0x500C0000
AUD_CTL                = 0x0000
AUD_STS                = 0x0004
AUD_TXD                = 0x0008
AUD_RXD                = 0x000C
AUD_INT_EN             = 0x0010
AUD_INT_RAW            = 0x0014
AUD_INT_CLR            = 0x0018
AUD_FIFO_CTL           = 0x001C
AUD_DAC_CTL            = 0x0020
AUD_PATH_CTL           = 0x0024

TONE_CTL               = 0x0030
TONE_FREQ              = 0x0034
TONE_DUR               = 0x0038
TONE_VOL               = 0x003C

PL011_UART_BASE = 0x09000000

class T107UnicornVM:
    def __init__(self, trace_instructions=False, max_instructions=0):
        self.trace_instructions = trace_instructions
        self.max_instructions = max_instructions
        self.instruction_count = 0
        self.cs = capstone.Cs(capstone.CS_ARCH_ARM, capstone.CS_MODE_ARM)

        # Initialize Unicorn ARM emulator
        self.uc = Uc(UC_ARCH_ARM, UC_MODE_ARM)

        # UART0 FIFOs
        self.uart0_rx_fifo = collections.deque()
        self.uart0_tx_buffer = bytearray()

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

        # SPI NAND Flash & Serial Flash Controller State
        self.flash_storage = bytearray(b'\xFF' * FLASH_DIRECT_SIZE)
        self.nand_page_buffer = bytearray(b'\xFF' * NAND_PAGE_TOTAL_SIZE)
        self.sfc_cmd = 0
        self.sfc_addr = 0
        self.sfc_ctl = 0
        self.sfc_sts = 0x02  # Ready / Done
        self.sfc_buf_ptr = 0
        self.sfc_int_en = 0
        self.nand_write_enabled = False
        self.nand_status_reg1 = 0x00
        self.nand_status_reg2 = 0x18
        self.nand_status_reg3 = 0x00

        # SPRD Audio Subsystem & Tone Generator State
        self.aud_ctl = 0
        self.aud_int_en = 0
        self.aud_int_raw = 0
        self.aud_fifo_ctl = (16 << 8) | (16 << 16)
        self.aud_dac_ctl = 0x00008001
        self.aud_path_ctl = 2  # Loudspeaker
        self.tone_ctl = 0
        self.tone_freq = 1000
        self.tone_dur = 50
        self.tone_vol = 80
        self.audio_tx_samples = []
        self.audio_tone_events = []

        # Map Memory Regions
        # 1. Internal SRAM
        self.uc.mem_map(SRAM_BASE, SRAM_SIZE)

        # 2. DRAM (map 48 MB, or 16 MB initial window for speed)
        self.dram_map_size = 16 * 1024 * 1024
        self.uc.mem_map(DRAM_BASE, self.dram_map_size)

        # 3. Peripheral MMIO space (map pages for SPRD UART, Timers, GPIO, KPD, SPI0, SPI1, SFC, PL011)
        self.uc.mem_map(SPRD_UART0_BASE, 0x1000)
        self.uc.mem_map(SPRD_TIMERS_BASE, 0x1000)
        self.uc.mem_map(SPRD_GPIO_BASE, 0x1000)
        self.uc.mem_map(SPRD_KPD_BASE, 0x1000)
        self.uc.mem_map(SPRD_SPI0_BASE, 0x1000)
        self.uc.mem_map(SPRD_SPI1_BASE, 0x1000)
        self.uc.mem_map(SPRD_SFC_BASE, 0x1000)
        self.uc.mem_map(SPRD_AUDIO_BASE, 0x1000)
        self.uc.mem_map(PL011_UART_BASE, 0x1000)
        # Also map page around 0x40001000 to test unmapped traps
        self.uc.mem_map(0x40001000 & ~0xFFF, 0x1000)

        # 4. Direct Flash Read Window (16 MB window at 0x60000000)
        self.uc.mem_map(FLASH_DIRECT_BASE, FLASH_DIRECT_SIZE)
        self.uc.mem_write(FLASH_DIRECT_BASE, bytes(self.flash_storage))

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
            byte_val = value & 0xFF
            self.uart0_tx_buffer.append(byte_val)
            char = chr(byte_val)
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

        # 7. SPRD Serial Flash Controller (SFC) & SPI NAND (0x500A0000)
        if SPRD_SFC_BASE <= address < (SPRD_SFC_BASE + 0x1000):
            offset = address - SPRD_SFC_BASE
            if offset == SFC_CMD:
                self.sfc_cmd = value & 0xFF
                cmd = self.sfc_cmd
                if cmd == NAND_CMD_RESET:
                    self.nand_write_enabled = False
                    self.sfc_buf_ptr = 0
                    self.nand_page_buffer = bytearray(b'\xFF' * NAND_PAGE_TOTAL_SIZE)
                elif cmd == NAND_CMD_READ_ID:
                    self.sfc_buf_ptr = 0
                    self.nand_page_buffer[0] = JEDEC_MFR_WINBOND
                    self.nand_page_buffer[1] = (JEDEC_DEV_W25N01GV >> 8) & 0xFF
                    self.nand_page_buffer[2] = JEDEC_DEV_W25N01GV & 0xFF
                    self.nand_page_buffer[3] = 0x00
                elif cmd == NAND_CMD_PAGE_READ:
                    page_idx = self.sfc_addr
                    flash_offset = page_idx * NAND_PAGE_SIZE
                    self.sfc_buf_ptr = 0
                    if flash_offset + NAND_PAGE_SIZE <= len(self.flash_storage):
                        self.nand_page_buffer[:NAND_PAGE_SIZE] = self.flash_storage[flash_offset : flash_offset + NAND_PAGE_SIZE]
                    else:
                        self.nand_page_buffer[:NAND_PAGE_SIZE] = b'\xFF' * NAND_PAGE_SIZE
                elif cmd in (NAND_CMD_READ_CACHE, NAND_CMD_READ_FAST):
                    self.sfc_buf_ptr = self.sfc_addr % NAND_PAGE_TOTAL_SIZE
                elif cmd == NAND_CMD_WRITE_ENABLE:
                    self.nand_write_enabled = True
                    self.nand_status_reg3 |= 0x02
                elif cmd == NAND_CMD_WRITE_DISABLE:
                    self.nand_write_enabled = False
                    self.nand_status_reg3 &= ~0x02
                elif cmd == NAND_CMD_PROGRAM_LOAD:
                    self.sfc_buf_ptr = self.sfc_addr % NAND_PAGE_TOTAL_SIZE
                elif cmd == NAND_CMD_PROGRAM_EXEC:
                    if self.nand_write_enabled:
                        page_idx = self.sfc_addr
                        flash_offset = page_idx * NAND_PAGE_SIZE
                        if flash_offset + NAND_PAGE_SIZE <= len(self.flash_storage):
                            for i in range(NAND_PAGE_SIZE):
                                self.flash_storage[flash_offset + i] &= self.nand_page_buffer[i]
                            if flash_offset + NAND_PAGE_SIZE <= FLASH_DIRECT_SIZE:
                                self.uc.mem_write(FLASH_DIRECT_BASE + flash_offset, bytes(self.flash_storage[flash_offset : flash_offset + NAND_PAGE_SIZE]))
                    self.nand_write_enabled = False
                    self.nand_status_reg3 &= ~0x02
                elif cmd == NAND_CMD_BLOCK_ERASE:
                    if self.nand_write_enabled:
                        block_idx = self.sfc_addr // NAND_PAGES_PER_BLOCK
                        flash_offset = block_idx * NAND_BLOCK_SIZE
                        if flash_offset + NAND_BLOCK_SIZE <= len(self.flash_storage):
                            self.flash_storage[flash_offset : flash_offset + NAND_BLOCK_SIZE] = b'\xFF' * NAND_BLOCK_SIZE
                            if flash_offset + NAND_BLOCK_SIZE <= FLASH_DIRECT_SIZE:
                                self.uc.mem_write(FLASH_DIRECT_BASE + flash_offset, b'\xFF' * NAND_BLOCK_SIZE)
                    self.nand_write_enabled = False
                    self.nand_status_reg3 &= ~0x02
                elif cmd == NAND_CMD_GET_FEATURE:
                    self.sfc_buf_ptr = 0
                    if self.sfc_addr == 0xC0:
                        self.nand_page_buffer[0] = self.nand_status_reg3
                    elif self.sfc_addr == 0xB0:
                        self.nand_page_buffer[0] = self.nand_status_reg2
                    else:
                        self.nand_page_buffer[0] = self.nand_status_reg1
                self.sfc_sts = 0x02  # Done / Ready
            elif offset == SFC_ADDR:
                self.sfc_addr = value & 0xFFFFFFFF
            elif offset == SFC_DATA:
                if self.sfc_buf_ptr + 4 <= NAND_PAGE_TOTAL_SIZE:
                    self.nand_page_buffer[self.sfc_buf_ptr]     = value & 0xFF
                    self.nand_page_buffer[self.sfc_buf_ptr + 1] = (value >> 8) & 0xFF
                    self.nand_page_buffer[self.sfc_buf_ptr + 2] = (value >> 16) & 0xFF
                    self.nand_page_buffer[self.sfc_buf_ptr + 3] = (value >> 24) & 0xFF
                    self.sfc_buf_ptr += 4
            elif offset == SFC_CTL:
                self.sfc_ctl = value & 0xFFFFFFFF
            elif offset == SFC_BUF_PTR:
                self.sfc_buf_ptr = (value & 0xFFFFFFFF) % NAND_PAGE_TOTAL_SIZE
            return

        # 8. SPRD Audio Subsystem (0x500C0000)
        if SPRD_AUDIO_BASE <= address < (SPRD_AUDIO_BASE + 0x1000):
            offset = address - SPRD_AUDIO_BASE
            if offset == AUD_CTL:
                self.aud_ctl = value & 0xFFFFFFFF
            elif offset == AUD_TXD:
                sample = value & 0xFFFFFFFF
                self.audio_tx_samples.append(sample)
            elif offset == AUD_INT_EN:
                self.aud_int_en = value & 0xFFFFFFFF
            elif offset == AUD_INT_CLR:
                self.aud_int_raw &= ~(value & 0xFFFFFFFF)
            elif offset == AUD_FIFO_CTL:
                self.aud_fifo_ctl = value & 0xFFFFFFFF
                if value & 1:  # Reset TX FIFO
                    self.audio_tx_samples.clear()
            elif offset == AUD_DAC_CTL:
                self.aud_dac_ctl = value & 0xFFFFFFFF
            elif offset == AUD_PATH_CTL:
                self.aud_path_ctl = value & 0xFFFFFFFF
            elif offset == TONE_CTL:
                self.tone_ctl = value & 0xFFFFFFFF
                if value & 1:  # TONE_EN
                    event = {
                        "freq": self.tone_freq,
                        "dur": self.tone_dur,
                        "vol": self.tone_vol,
                        "cont": bool(value & 2),
                        "sine": bool(value & 4)
                    }
                    self.audio_tone_events.append(event)
                    print(f"[N2660Emu Audio] Beep Tone: {self.tone_freq} Hz, {self.tone_dur} ms, volume {self.tone_vol}%")
                    if not (value & 2):  # Self-clear if not continuous
                        self.tone_ctl &= ~1
            elif offset == TONE_FREQ:
                self.tone_freq = value & 0xFFFFFFFF
            elif offset == TONE_DUR:
                self.tone_dur = value & 0xFFFFFFFF
            elif offset == TONE_VOL:
                self.tone_vol = value & 0xFFFFFFFF
            return

        # 9. Catch-all unknown peripheral writes
        if 0x40000000 <= address < 0x60000000:
            pc = uc.reg_read(UC_ARM_REG_PC)
            print(f"\n[N2660Emu MMIO TRAP] Unknown peripheral WRITE at 0x{address:08X} = 0x{value:08X} (size {size}) from PC 0x{pc:08X}")

    def _hook_mem_read(self, uc, access, address, size, value, user_data):
        # 1. SPRD UART0 Status & RX FIFO
        if address == (SPRD_UART0_BASE + SPRD_STS1_REG):
            rx_count = min(0xFF, len(self.uart0_rx_fifo))
            tx_ready = 1 << 8
            sts = (tx_ready | rx_count)
            uc.mem_write(address, sts.to_bytes(4, 'little'))
            return

        if address == (SPRD_UART0_BASE + SPRD_RXD_REG):
            rx_byte = self.uart0_rx_fifo.popleft() if self.uart0_rx_fifo else 0
            uc.mem_write(address, rx_byte.to_bytes(4, 'little'))
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

        # 7. SPRD Serial Flash Controller (SFC)
        if SPRD_SFC_BASE <= address < (SPRD_SFC_BASE + 0x1000):
            offset = address - SPRD_SFC_BASE
            if offset == SFC_CMD:
                val = self.sfc_cmd
            elif offset == SFC_ADDR:
                val = self.sfc_addr
            elif offset == SFC_DATA:
                if self.sfc_buf_ptr + 4 <= NAND_PAGE_TOTAL_SIZE:
                    val = (self.nand_page_buffer[self.sfc_buf_ptr] |
                           (self.nand_page_buffer[self.sfc_buf_ptr + 1] << 8) |
                           (self.nand_page_buffer[self.sfc_buf_ptr + 2] << 16) |
                           (self.nand_page_buffer[self.sfc_buf_ptr + 3] << 24))
                    self.sfc_buf_ptr += 4
                elif self.sfc_buf_ptr < NAND_PAGE_TOTAL_SIZE:
                    val = self.nand_page_buffer[self.sfc_buf_ptr]
                    self.sfc_buf_ptr += 1
                else:
                    val = 0xFFFFFFFF
            elif offset == SFC_CTL:
                val = self.sfc_ctl
            elif offset == SFC_STS:
                val = self.sfc_sts
            elif offset == SFC_BUF_PTR:
                val = self.sfc_buf_ptr
            else:
                val = 0
            uc.mem_write(address, (val & 0xFFFFFFFF).to_bytes(4, 'little'))
            return

        # 8. SPRD Audio Subsystem (0x500C0000)
        if SPRD_AUDIO_BASE <= address < (SPRD_AUDIO_BASE + 0x1000):
            offset = address - SPRD_AUDIO_BASE
            if offset == AUD_CTL:
                val = self.aud_ctl
            elif offset == AUD_STS:
                cnt = min(64, len(self.audio_tx_samples))
                val = (1 if cnt == 0 else 0) | (2 if cnt >= 64 else 0) | 4 | (cnt << 8)
            elif offset == AUD_RXD:
                val = 0
            elif offset == AUD_INT_EN:
                val = self.aud_int_en
            elif offset == AUD_INT_RAW:
                val = self.aud_int_raw
            elif offset == AUD_FIFO_CTL:
                val = self.aud_fifo_ctl
            elif offset == AUD_DAC_CTL:
                val = self.aud_dac_ctl
            elif offset == AUD_PATH_CTL:
                val = self.aud_path_ctl
            elif offset == TONE_CTL:
                val = self.tone_ctl
            elif offset == TONE_FREQ:
                val = self.tone_freq
            elif offset == TONE_DUR:
                val = self.tone_dur
            elif offset == TONE_VOL:
                val = self.tone_vol
            else:
                val = 0
            uc.mem_write(address, (val & 0xFFFFFFFF).to_bytes(4, 'little'))
            return

        # 9. Catch-all unknown peripheral reads
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

    def load_fdl1(self, file_path, load_addr=0x00006200, simulate_bsl=False):
        """Loads FDL1 (First Downloader) into SRAM at load_addr."""
        with open(file_path, "rb") as f:
            data = f.read()
        print(f"[N2660Emu BSL] Staging FDL1 '{file_path}' ({len(data)} bytes) to address 0x{load_addr:08X}...")
        if simulate_bsl:
            print(f"[N2660Emu BSL] Simulating Spreadtrum BSL Protocol Handshake over UART0...")
            client = BslClient(use_crc16=True)
            stream = client.build_file_transfer_stream(data, target_addr=load_addr, chunk_size=1024)
            print(f"[N2660Emu BSL] Generated {len(stream)} BSL HDLC packets (CONNECT -> START_DATA -> MIDST -> END -> EXEC)")
            def mem_writer(addr, chunk):
                self.uc.mem_write(addr, chunk)
            server = BslServer(write_mem_fn=mem_writer)
            for pkt in stream:
                server.process_incoming(pkt)
            print(f"[N2660Emu BSL] BSL handshake acknowledged. FDL1 loaded at 0x{load_addr:08X}, ready to execute.")
        else:
            self.uc.mem_write(load_addr, data)
        return len(data)

    def load_fdl2(self, file_path, load_addr=0x80100000):
        """Loads FDL2 (Second Downloader) into high DRAM at load_addr."""
        with open(file_path, "rb") as f:
            data = f.read()
        print(f"[N2660Emu BSL] Staging FDL2 '{file_path}' ({len(data)} bytes) to address 0x{load_addr:08X}...")
        self.uc.mem_write(load_addr, data)
        return len(data)

    def load_flash_image(self, file_path, offset=0):
        """Loads a raw binary flash image or partition into flash memory."""
        with open(file_path, "rb") as f:
            data = f.read()
        end = offset + len(data)
        if end > len(self.flash_storage):
            self.flash_storage.extend(b'\xFF' * (end - len(self.flash_storage)))
        self.flash_storage[offset:end] = data
        direct_len = min(len(data), max(0, FLASH_DIRECT_SIZE - offset))
        if direct_len > 0:
            self.uc.mem_write(FLASH_DIRECT_BASE + offset, bytes(self.flash_storage[offset : offset + direct_len]))
        print(f"[VM Flash] Loaded '{file_path}' ({len(data)} bytes) at flash offset 0x{offset:08X}")

    def load_bootrom(self, file_path=None):
        """Loads Unisoc T107 BootROM binary at 0x00000000."""
        if file_path is None:
            default_path = os.path.join(os.path.dirname(__file__), "..", "roms", "bootrom.bin")
            if not os.path.isfile(default_path):
                roms_dir = os.path.join(os.path.dirname(__file__), "..", "roms")
                if os.path.isfile(os.path.join(roms_dir, "Makefile")):
                    try:
                        subprocess.run(["make", "-C", roms_dir], check=True, capture_output=True)
                    except Exception:
                        pass
            if os.path.isfile(default_path):
                file_path = default_path
            else:
                raise FileNotFoundError(f"Default BootROM binary not found at '{default_path}'. Build with 'make -C roms'.")
        with open(file_path, "rb") as f:
            data = f.read()
        print(f"[VM BootROM] Staging Clean-Room BootROM '{file_path}' ({len(data)} bytes) at 0x00000000...")
        self.uc.mem_write(0x00000000, data)
        return len(data)

    def load_boot0(self, file_path, block_index=0, auto_wrap=True):
        """Loads or packages a BOOT0 image into SPI NAND flash block 0."""
        with open(file_path, "rb") as f:
            data = f.read()
        if len(data) >= 4:
            magic = struct.unpack("<I", data[:4])[0]
        else:
            magic = 0
        if magic not in (SPRD_BOOT0_MAGIC, SPRD_BOOT0_MAGIC_BIMG) and auto_wrap:
            print(f"[VM BootROM] Raw binary detected. Wrapping '{file_path}' ({len(data)} bytes) into SPRD BOOT0 container...")
            data = create_boot0_image(data, load_addr=0x00006200, entry_point=0x00006200)
        offset = block_index * BLOCK_SIZE
        end = offset + len(data)
        if end > len(self.flash_storage):
            self.flash_storage.extend(b'\xFF' * (end - len(self.flash_storage)))
        self.flash_storage[offset:end] = data
        direct_len = min(len(data), max(0, FLASH_DIRECT_SIZE - offset))
        if direct_len > 0:
            self.uc.mem_write(FLASH_DIRECT_BASE + offset, bytes(self.flash_storage[offset : offset + direct_len]))
        print(f"[VM BootROM] Staged BOOT0 ({len(data)} bytes) into SPI NAND Block {block_index} (Flash Offset 0x{offset:08X})")
        return len(data)

    def setup_atags(self, ram_base=0x80000000, ram_size=48*1024*1024,
                    cmdline=DEFAULT_CMDLINE, atags_addr=DEFAULT_ATAGS_ADDR, board_id=DEFAULT_BOARD_ID):
        """Constructs and stages standard ARM ATAGS in DRAM, setting up r0, r1, r2 registers."""
        data = create_standard_n2660_atags(dram_base=ram_base, dram_size=ram_size, cmdline=cmdline)
        self.uc.mem_write(atags_addr, data)
        self.uc.reg_write(UC_ARM_REG_R0, 0)
        self.uc.reg_write(UC_ARM_REG_R1, board_id)
        self.uc.reg_write(UC_ARM_REG_R2, atags_addr)
        print(f"[VM ATAGS] Staged ARM Boot Tags ({len(data)} bytes) at 0x{atags_addr:08X} (r0=0, r1=0x{board_id:04X}, r2=0x{atags_addr:08X})")
        return len(data)

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

    def export_wav(self, file_path):
        """Exports captured audio PCM samples and buzzer tones as standard 16-bit 44.1 kHz WAV."""
        os.makedirs(os.path.dirname(os.path.abspath(file_path)), exist_ok=True)
        sample_rate = 44100
        raw_samples = bytearray()

        # If PCM samples written to AUD_TXD exist, output them
        if self.audio_tx_samples:
            for s in self.audio_tx_samples:
                left = (s & 0xFFFF)
                if left >= 0x8000:
                    left -= 0x10000
                right = ((s >> 16) & 0xFFFF)
                if right >= 0x8000:
                    right -= 0x10000
                raw_samples.extend(struct.pack("<hh", left, right))

        # If tone events exist, synthesize them into PCM audio
        if self.audio_tone_events:
            for ev in self.audio_tone_events:
                freq = max(20, min(20000, ev.get("freq", 1000)))
                dur_ms = max(10, min(5000, ev.get("dur", 50)))
                vol = max(0, min(100, ev.get("vol", 80))) / 100.0
                amplitude = int(32767 * vol * 0.8)
                num_samples = int(sample_rate * dur_ms / 1000.0)
                is_sine = ev.get("sine", False)

                for i in range(num_samples):
                    t = float(i) / sample_rate
                    if is_sine:
                        sample_val = int(amplitude * math.sin(2.0 * math.pi * freq * t))
                    else:
                        # Square wave buzzer
                        period = 1.0 / freq
                        sample_val = amplitude if (t % period) < (period / 2.0) else -amplitude
                    raw_samples.extend(struct.pack("<hh", sample_val, sample_val))

        if not raw_samples:
            raw_samples.extend(b'\x00' * (sample_rate // 10 * 4))

        with wave.open(file_path, "wb") as wf:
            wf.setnchannels(2)      # Stereo
            wf.setsampwidth(2)      # 16-bit
            wf.setframerate(sample_rate)
            wf.writeframes(raw_samples)
        print(f"[N2660Emu Audio] Saved audio dump to '{file_path}' ({len(raw_samples)//4} frames, 44.1 kHz stereo)")

def main():
    parser = argparse.ArgumentParser(description="N2660Emu Unicorn VM Runner")
    parser.add_argument("binary", nargs="?", default=None, help="Path to raw binary file to execute")
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
    parser.add_argument("--flash", type=str, default=None, help="Path to raw SPI NAND flash image to mount")
    parser.add_argument("--fdl1", type=str, default=None, help="Path to FDL1 binary to stage and run (default addr: 0x00006200)")
    parser.add_argument("--fdl1-addr", type=lambda x: int(x, 0), default=0x00006200, help="FDL1 load address (default: 0x00006200)")
    parser.add_argument("--fdl2", type=str, default=None, help="Path to FDL2 binary to stage into DRAM (default addr: 0x80100000)")
    parser.add_argument("--fdl2-addr", type=lambda x: int(x, 0), default=0x80100000, help="FDL2 load address (default: 0x80100000)")
    parser.add_argument("--bsl", action="store_true", help="Simulate BootROM BSL handshake over UART0 when loading FDL1")
    parser.add_argument("--bootrom", action="store_true", help="Execute hardware BootROM reset vector at 0x00000000")
    parser.add_argument("--bootrom-file", type=str, default=None, help="Path to custom BootROM binary (default: roms/bootrom.bin)")
    parser.add_argument("--boot0", type=str, default=None, help="Path to BOOT0 binary to stage into SPI NAND Block 0")
    parser.add_argument("--boot-mode", choices=["auto", "bsl", "nand"], default="auto", help="Boot mode dispatch (default: auto)")
    parser.add_argument("--boot-key", choices=["star", "none"], default="none", help="Hardware boot key to hold at reset (default: none)")
    parser.add_argument("--atags", action="store_true", help="Stage ARM Boot Tags (ATAGS) at 0x80000100 for kernel booting")
    parser.add_argument("--atags-addr", type=lambda x: int(x, 0), default=DEFAULT_ATAGS_ADDR, help="Physical DRAM address to stage ATAGS (default: 0x80000100)")
    parser.add_argument("--board-id", type=lambda x: int(x, 0), default=DEFAULT_BOARD_ID, help="ARM machine/board ID in r1 (default: 0x9117)")
    parser.add_argument("--cmdline", type=str, default=DEFAULT_CMDLINE, help="Kernel command line string passed via ATAG_CMDLINE")
    parser.add_argument("--audio-dump", type=str, default=None, help="Path to dump audio output as WAV file")
    args = parser.parse_args()

    if not args.binary and not args.fdl1 and not args.bootrom and not args.boot0:
        print("Error: Either binary, --fdl1, --bootrom, or --boot0 must be specified.")
        sys.exit(1)

    vm = T107UnicornVM(trace_instructions=args.trace, max_instructions=args.max_insns)
    vm.set_flip_state(args.flip == "open")

    if args.flash:
        vm.load_flash_image(args.flash)

    if args.boot0:
        vm.load_boot0(args.boot0, block_index=0)

    # Boot key injection
    if args.boot_key == "star" or args.boot_mode == "bsl":
        vm.inject_key("*", pressed=True)

    if args.key:
        for k in args.key:
            vm.inject_key(k, pressed=True)

    entry_point = args.entry
    if args.fdl1:
        if not os.path.isfile(args.fdl1):
            print(f"Error: FDL1 binary file '{args.fdl1}' not found.")
            sys.exit(1)
        vm.load_fdl1(args.fdl1, load_addr=args.fdl1_addr, simulate_bsl=args.bsl)
        entry_point = args.fdl1_addr
        if args.fdl2:
            if not os.path.isfile(args.fdl2):
                print(f"Error: FDL2 binary file '{args.fdl2}' not found.")
                sys.exit(1)
            vm.load_fdl2(args.fdl2, load_addr=args.fdl2_addr)

    if args.bootrom or args.boot0 or (not args.binary and not args.fdl1):
        vm.load_bootrom(args.bootrom_file)
        entry_point = 0x00000000
    elif args.binary:
        if not os.path.isfile(args.binary):
            print(f"Error: Binary file '{args.binary}' not found.")
            sys.exit(1)
        vm.load_binary(args.binary, args.entry)
        entry_point = args.entry

    if args.atags:
        if args.binary and os.path.isfile(args.binary):
            bin_sz = os.path.getsize(args.binary)
            if args.entry <= args.atags_addr < args.entry + bin_sz:
                print(f"[VM ATAGS WARNING] Staging ATAGS at 0x{args.atags_addr:08X} overlaps with loaded binary [0x{args.entry:08X} - 0x{args.entry + bin_sz:08X}]!")
        vm.setup_atags(cmdline=args.cmdline, atags_addr=args.atags_addr, board_id=args.board_id)

    success = vm.run(entry_point=entry_point, timeout_sec=args.timeout)

    if args.screen_dump:
        vm.export_ppm(args.screen_dump)

    if args.sub_screen_dump:
        vm.export_sub_ppm(args.sub_screen_dump)

    if args.preview_screen:
        vm.render_terminal_preview()

    if args.preview_sub_screen:
        vm.render_sub_terminal_preview()

    if args.audio_dump:
        vm.export_wav(args.audio_dump)

    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
