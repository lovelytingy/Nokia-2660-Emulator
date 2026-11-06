#!/usr/bin/env python3
"""
Unisoc / Spreadtrum BOOT0 Image Packaging & NAND Flash Utility
Generates and inspects Unisoc T107 (UMS9117) BOOT0 / SPL images with valid
SPRD headers, checksums, and SPI NAND flash block 0 layout.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import sys
import os
import struct
import argparse

# SPRD BOOT0 Header Definitions
SPRD_BOOT0_MAGIC       = 0x53505244  # "SPRD"
SPRD_BOOT0_MAGIC_BIMG  = 0x42494D47  # "BIMG"
SPRD_BOOT0_VERSION     = 1
DEFAULT_LOAD_ADDR      = 0x00006200  # Internal SRAM entry address
HEADER_SIZE            = 32          # 32-byte header
PAGE_SIZE              = 2048        # Standard 2KB SPI NAND page
BLOCK_SIZE             = 128 * 1024  # 128 KB SPI NAND block (64 pages)

# Header Structure (32 bytes, little-endian):
# uint32_t magic;        /* 0x00: "SPRD" (0x53505244) */
# uint32_t version;      /* 0x04: Version (1) */
# uint32_t load_addr;    /* 0x08: Target SRAM address (0x00006200) */
# uint32_t image_size;   /* 0x0C: Size of binary payload in bytes */
# uint32_t entry_point;  /* 0x10: Entry point address (0x00006200) */
# uint32_t checksum;     /* 0x14: 32-bit addition sum of payload bytes */
# uint32_t flags;        /* 0x18: Execution flags (0 = normal) */
# uint32_t reserved;     /* 0x1C: Reserved / padding */

HEADER_FORMAT = "<IIIIIIII"

def calculate_checksum(data: bytes) -> int:
    """Calculates 32-bit additive checksum of payload data."""
    csum = 0
    # Process 4 bytes at a time
    remainder = len(data) % 4
    full_words = len(data) - remainder
    if full_words > 0:
        words = struct.unpack(f"<{full_words // 4}I", data[:full_words])
        csum = sum(words) & 0xFFFFFFFF
    for b in data[full_words:]:
        csum = (csum + b) & 0xFFFFFFFF
    return csum

def create_boot0_image(payload: bytes, load_addr: int = DEFAULT_LOAD_ADDR,
                       entry_point: int = DEFAULT_LOAD_ADDR, flags: int = 0) -> bytes:
    """Wraps an executable binary in an SPRD BOOT0 header."""
    image_size = len(payload)
    checksum = calculate_checksum(payload)
    header = struct.pack(
        HEADER_FORMAT,
        SPRD_BOOT0_MAGIC,
        SPRD_BOOT0_VERSION,
        load_addr,
        image_size,
        entry_point,
        checksum,
        flags,
        0
    )
    return header + payload

def parse_boot0_header(data: bytes) -> dict:
    """Parses and validates an SPRD BOOT0 header."""
    if len(data) < HEADER_SIZE:
        raise ValueError(f"Data too short for BOOT0 header: {len(data)} bytes (need {HEADER_SIZE})")

    magic, version, load_addr, image_size, entry_point, checksum, flags, reserved = struct.unpack(
        HEADER_FORMAT, data[:HEADER_SIZE]
    )

    is_valid_magic = (magic in (SPRD_BOOT0_MAGIC, SPRD_BOOT0_MAGIC_BIMG))
    magic_str = struct.pack(">I", magic).decode("latin-1", errors="replace")

    payload = data[HEADER_SIZE : HEADER_SIZE + image_size]
    calculated_csum = calculate_checksum(payload) if len(payload) == image_size else None
    csum_valid = (calculated_csum == checksum) if calculated_csum is not None else False

    return {
        "magic": magic,
        "magic_str": magic_str,
        "is_valid_magic": is_valid_magic,
        "version": version,
        "load_addr": load_addr,
        "image_size": image_size,
        "entry_point": entry_point,
        "checksum": checksum,
        "calculated_checksum": calculated_csum,
        "checksum_valid": csum_valid,
        "flags": flags,
        "payload_len_actual": len(payload)
    }

def inject_boot0_into_flash(boot0_data: bytes, flash_path: str, block_index: int = 0, flash_size: int = 128 * 1024 * 1024):
    """Injects BOOT0 image into the specified block of an SPI NAND flash file."""
    offset = block_index * BLOCK_SIZE
    if os.path.exists(flash_path):
        with open(flash_path, "r+b") as f:
            f.seek(offset)
            f.write(boot0_data)
    else:
        # Create new initialized flash image (all 0xFF)
        os.makedirs(os.path.dirname(os.path.abspath(flash_path)) or ".", exist_ok=True)
        with open(flash_path, "wb") as f:
            # Write up to 1MB or block offset + boot0_data
            min_size = max(offset + len(boot0_data), 1024 * 1024)
            data = bytearray(b"\xFF" * min_size)
            data[offset : offset + len(boot0_data)] = boot0_data
            f.write(data)

def extract_boot0_from_flash(flash_path: str, block_index: int = 0) -> bytes:
    """Extracts BOOT0 image from the specified block of a flash image."""
    offset = block_index * BLOCK_SIZE
    with open(flash_path, "rb") as f:
        f.seek(offset)
        header_raw = f.read(HEADER_SIZE)
        info = parse_boot0_header(header_raw)
        if not info["is_valid_magic"]:
            raise ValueError(f"No valid SPRD header in block {block_index} at offset 0x{offset:08X} (magic: {info['magic_str']})")
        f.seek(offset)
        total_len = HEADER_SIZE + info["image_size"]
        return f.read(total_len)

def main():
    parser = argparse.ArgumentParser(description="Unisoc / Spreadtrum BOOT0 Image Utility")
    subparsers = parser.add_subparsers(dest="command", required=True, help="Command")

    # Command: create
    create_parser = subparsers.add_parser("create", help="Package raw binary into SPRD BOOT0 image")
    create_parser.add_argument("-i", "--input", required=True, help="Path to input raw binary payload")
    create_parser.add_argument("-o", "--output", required=True, help="Path to output BOOT0 image")
    create_parser.add_argument("--load-addr", type=lambda x: int(x, 0), default=DEFAULT_LOAD_ADDR,
                               help=f"Target SRAM load address (default: 0x{DEFAULT_LOAD_ADDR:08X})")
    create_parser.add_argument("--entry", type=lambda x: int(x, 0), default=DEFAULT_LOAD_ADDR,
                               help=f"Execution entry point (default: 0x{DEFAULT_LOAD_ADDR:08X})")
    create_parser.add_argument("--flags", type=lambda x: int(x, 0), default=0, help="Boot flags (default: 0)")

    # Command: verify
    verify_parser = subparsers.add_parser("verify", help="Inspect and verify an SPRD BOOT0 image")
    verify_parser.add_argument("file", help="Path to BOOT0 image file")

    # Command: inject
    inject_parser = subparsers.add_parser("inject", help="Inject BOOT0 image into SPI NAND flash image")
    inject_parser.add_argument("-b", "--boot0", required=True, help="Path to BOOT0 image file")
    inject_parser.add_argument("-f", "--flash", required=True, help="Path to SPI NAND flash image file")
    inject_parser.add_argument("--block", type=int, default=0, help="Flash block index (default: 0)")

    # Command: extract
    extract_parser = subparsers.add_parser("extract", help="Extract BOOT0 image from SPI NAND flash image")
    extract_parser.add_argument("-f", "--flash", required=True, help="Path to SPI NAND flash image file")
    extract_parser.add_argument("-o", "--output", required=True, help="Path to output BOOT0 image file")
    extract_parser.add_argument("--block", type=int, default=0, help="Flash block index (default: 0)")

    args = parser.parse_args()

    if args.command == "create":
        with open(args.input, "rb") as f:
            payload = f.read()
        image = create_boot0_image(payload, load_addr=args.load_addr, entry_point=args.entry, flags=args.flags)
        with open(args.output, "wb") as f:
            f.write(image)
        csum = calculate_checksum(payload)
        print(f"[mksprd_img] Created BOOT0 image '{args.output}':")
        print(f"  Payload size : {len(payload)} bytes (Total image: {len(image)} bytes)")
        print(f"  Load Address : 0x{args.load_addr:08X}")
        print(f"  Entry Point  : 0x{args.entry:08X}")
        print(f"  Checksum     : 0x{csum:08X}")

    elif args.command == "verify":
        with open(args.file, "rb") as f:
            data = f.read()
        info = parse_boot0_header(data)
        print(f"[mksprd_img] Inspecting '{args.file}':")
        print(f"  Magic        : 0x{info['magic']:08X} ('{info['magic_str']}') -> {'VALID' if info['is_valid_magic'] else 'INVALID'}")
        print(f"  Version      : {info['version']}")
        print(f"  Load Address : 0x{info['load_addr']:08X}")
        print(f"  Image Size   : {info['image_size']} bytes (actual available: {info['payload_len_actual']} bytes)")
        print(f"  Entry Point  : 0x{info['entry_point']:08X}")
        print(f"  Checksum     : 0x{info['checksum']:08X} (calculated: 0x{info['calculated_checksum']:08X}) -> {'VALID' if info['checksum_valid'] else 'INVALID'}")
        print(f"  Flags        : 0x{info['flags']:08X}")
        if not info["is_valid_magic"] or not info["checksum_valid"]:
            sys.exit(1)

    elif args.command == "inject":
        with open(args.boot0, "rb") as f:
            boot0_data = f.read()
        info = parse_boot0_header(boot0_data)
        if not info["is_valid_magic"]:
            print(f"Error: '{args.boot0}' is not a valid SPRD BOOT0 image.", file=sys.stderr)
            sys.exit(1)
        inject_boot0_into_flash(boot0_data, args.flash, block_index=args.block)
        print(f"[mksprd_img] Successfully injected BOOT0 ({len(boot0_data)} bytes) into '{args.flash}' at Block {args.block} (Offset 0x{args.block * BLOCK_SIZE:08X})")

    elif args.command == "extract":
        extracted = extract_boot0_from_flash(args.flash, block_index=args.block)
        with open(args.output, "wb") as f:
            f.write(extracted)
        print(f"[mksprd_img] Successfully extracted BOOT0 ({len(extracted)} bytes) from '{args.flash}' Block {args.block} -> '{args.output}'")

if __name__ == "__main__":
    main()
