#!/usr/bin/env python3
"""
Capstone Disassembler with Unisoc T107 Symbol Annotations
Disassembles ARMv7-A / Thumb-2 firmware binaries and identifies hardware MMIO references.

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

import capstone

# Known Unisoc T107 symbols for annotation
T107_SYMBOLS = {
    0x00000000: "IRAM_BASE (Internal SRAM)",
    0x00006200: "FDL1_ENTRY_POINT",
    0x40000000: "PERIPH_AON_BASE",
    0x50000000: "SPRD_UART0_TXD",
    0x50000004: "SPRD_UART0_RXD",
    0x50000008: "SPRD_UART0_STS0",
    0x5000000C: "SPRD_UART0_STS1 (FIFO Status)",
    0x50000010: "SPRD_UART0_IEN",
    0x50010000: "SPRD_UART1_BASE",
    0x50020000: "SPRD_UART2_BASE",
    0x50030000: "SPRD_TIMERS_BASE",
    0x50040000: "SPRD_GPIO_BASE",
    0x50080000: "SPRD_SPI0_LCD_BASE",
    0x80000000: "DRAM_BASE (LPDDR2)",
    0x80020000: "BOOT1_ENTRY_POINT",
    0x80040000: "MOCOR_OS_ENTRY",
    0x80100000: "FDL2_ENTRY_POINT",
}

def disassemble_binary(bin_path, base_addr, mode_thumb=False, count=0):
    with open(bin_path, "rb") as f:
        code = f.read()

    arch_mode = capstone.CS_MODE_THUMB if mode_thumb else capstone.CS_MODE_ARM
    md = capstone.Cs(capstone.CS_ARCH_ARM, arch_mode)
    md.detail = True

    print(f"Disassembling '{bin_path}' at base 0x{base_addr:08X} ({'Thumb' if mode_thumb else 'ARM'} mode):")
    print("=" * 80)

    n = 0
    for insn in md.disasm(code, base_addr):
        annotation = ""
        # Check if immediate operand matches a known hardware symbol
        for op in insn.operands:
            if op.type == capstone.arm.ARM_OP_IMM:
                imm = op.imm & 0xFFFFFFFF
                if imm in T107_SYMBOLS:
                    annotation = f"  ; <{T107_SYMBOLS[imm]}>"

        print(f"0x{insn.address:08X}:  {insn.bytes.hex():12s}  {insn.mnemonic:8s} {insn.op_str:<30s}{annotation}")
        n += 1
        if count > 0 and n >= count:
            break

def main():
    parser = argparse.ArgumentParser(description="Disassemble ARM/Thumb binary with T107 annotations")
    parser.add_argument("binary", help="Path to raw binary file")
    parser.add_argument("--base", type=lambda x: int(x, 0), default=0x80000000,
                        help="Base load address (default: 0x80000000)")
    parser.add_argument("--thumb", action="store_true", help="Disassemble as Thumb/Thumb-2 instead of ARM")
    parser.add_argument("-n", "--count", type=int, default=100, help="Number of instructions to disassemble (0=all)")
    args = parser.parse_args()

    if not os.path.isfile(args.binary):
        print(f"File not found: {args.binary}")
        sys.exit(1)

    disassemble_binary(args.binary, args.base, mode_thumb=args.thumb, count=args.count)

if __name__ == "__main__":
    main()
