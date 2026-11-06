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

PL011_UART_BASE = 0x09000000

class T107UnicornVM:
    def __init__(self, trace_instructions=False, max_instructions=0):
        self.trace_instructions = trace_instructions
        self.max_instructions = max_instructions
        self.instruction_count = 0
        self.cs = capstone.Cs(capstone.CS_ARCH_ARM, capstone.CS_MODE_ARM)

        # Initialize Unicorn ARM emulator
        self.uc = Uc(UC_ARCH_ARM, UC_MODE_ARM)

        # Map Memory Regions
        # 1. Internal SRAM
        self.uc.mem_map(SRAM_BASE, SRAM_SIZE)

        # 2. DRAM (map 48 MB, or 16 MB initial window for speed)
        self.dram_map_size = 16 * 1024 * 1024
        self.uc.mem_map(DRAM_BASE, self.dram_map_size)

        # 3. Peripheral MMIO space (map pages for SPRD UART and PL011)
        self.uc.mem_map(SPRD_UART0_BASE, 0x1000)
        self.uc.mem_map(PL011_UART_BASE, 0x1000)
        # Also map page around 0x40001000 to test unmapped traps
        self.uc.mem_map(0x40001000 & ~0xFFF, 0x1000)

        # Register Hooks
        self.uc.hook_add(UC_HOOK_MEM_WRITE, self._hook_mem_write)
        self.uc.hook_add(UC_HOOK_MEM_READ, self._hook_mem_read)
        self.uc.hook_add(UC_HOOK_MEM_UNMAPPED, self._hook_mem_unmapped)

        if self.trace_instructions:
            self.uc.hook_add(UC_HOOK_CODE, self._hook_code)

    def _hook_code(self, uc, address, size, user_data):
        self.instruction_count += 1
        code = uc.mem_read(address, size)
        for insn in self.cs.disasm(code, address):
            print(f"[TRACE] 0x{insn.address:08X}: {insn.mnemonic:8s} {insn.op_str}")

        if self.max_instructions > 0 and self.instruction_count >= self.max_instructions:
            print(f"[VM] Instruction limit ({self.max_instructions}) reached. Stopping.")
            uc.emu_stop()

    def _hook_mem_write(self, uc, access, address, size, value, user_data):
        # Intercept SPRD UART0 TXD register write
        if address == (SPRD_UART0_BASE + SPRD_TXD_REG):
            char = chr(value & 0xFF)
            try:
                sys.stdout.write(char)
                sys.stdout.flush()
            except BrokenPipeError:
                pass
            return

        # Intercept PL011 UART DR write
        if address == PL011_UART_BASE:
            # We don't need to duplicate output if both are written
            return

        # Check other peripheral writes
        if 0x40000000 <= address < 0x60000000:
            pc = uc.reg_read(UC_ARM_REG_PC)
            print(f"\n[N2660Emu MMIO TRAP] Unknown peripheral WRITE at 0x{address:08X} = 0x{value:08X} (size {size}) from PC 0x{pc:08X}")

    def _hook_mem_read(self, uc, access, address, size, value, user_data):
        # Intercept SPRD UART0 STS1 (FIFO status)
        if address == (SPRD_UART0_BASE + SPRD_STS1_REG):
            # Return bit 8 = 1 (TX FIFO empty / ready)
            return

        # Log unmapped / unknown peripheral reads
        if 0x40000000 <= address < 0x60000000 and address != (SPRD_UART0_BASE + SPRD_STS1_REG):
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

def main():
    parser = argparse.ArgumentParser(description="N2660Emu Unicorn VM Runner")
    parser.add_argument("binary", help="Path to raw binary file to execute")
    parser.add_argument("--entry", type=lambda x: int(x, 0), default=0x80000000,
                        help="Execution entry point address (default: 0x80000000)")
    parser.add_argument("--trace", action="store_true", help="Enable instruction tracing")
    parser.add_argument("--max-insns", type=int, default=0, help="Maximum instructions to execute")
    parser.add_argument("--timeout", type=int, default=5, help="Timeout in seconds")
    args = parser.parse_args()

    if not os.path.isfile(args.binary):
        print(f"Error: Binary file '{args.binary}' not found.")
        sys.exit(1)

    vm = T107UnicornVM(trace_instructions=args.trace, max_instructions=args.max_insns)
    vm.load_binary(args.binary, args.entry)
    success = vm.run(entry_point=args.entry, timeout_sec=args.timeout)
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
