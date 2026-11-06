#!/usr/bin/env python3
"""
ARM Boot Tags (ATAGS) Synthesizer and Parser for Unisoc T107 (UMS9117)
Builds and inspects standard ARMv7-A ATAGS lists for kernel handoff.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import struct
import argparse

# Standard ARM ATAG Identifiers
ATAG_NONE      = 0x00000000
ATAG_CORE      = 0x54410001
ATAG_MEM       = 0x54410002
ATAG_VIDEOTEXT = 0x54410003
ATAG_RAMDISK   = 0x54410004
ATAG_INITRD2   = 0x54420005
ATAG_SERIAL    = 0x54410006
ATAG_REVISION  = 0x54410007
ATAG_VIDEOLFB  = 0x54410008
ATAG_CMDLINE   = 0x54410009

DEFAULT_BOARD_ID   = 0x9117        # Unisoc UMS9117 / T107
DEFAULT_ATAGS_ADDR = 0x80000100    # Standard offset (256 bytes into DRAM)
DEFAULT_DRAM_BASE  = 0x80000000    # High DRAM base
DEFAULT_DRAM_SIZE  = 48 * 1024 * 1024  # 48 MB for Nokia 2660 Flip
DEFAULT_CMDLINE    = "console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M earlycon"

class AtagsBuilder:
    """Constructs a contiguous binary block of ARM Boot Tags."""

    def __init__(self):
        self.tags = bytearray()

    def add_core(self, flags: int = 0, pagesize: int = 4096, rootdev: int = 0):
        """Appends ATAG_CORE tag."""
        # size = 5 words (header: 2 words + 3 words payload)
        self.tags.extend(struct.pack("<IIIII", 5, ATAG_CORE, flags, pagesize, rootdev))
        return self

    def add_mem(self, size: int, start: int):
        """Appends ATAG_MEM tag defining physical RAM window."""
        # size = 4 words (header: 2 words + size + start)
        self.tags.extend(struct.pack("<IIII", 4, ATAG_MEM, size, start))
        return self

    def add_cmdline(self, cmdline: str):
        """Appends ATAG_CMDLINE tag with kernel boot arguments."""
        cmd_bytes = cmdline.encode("ascii") + b"\x00"
        # Pad to 4-byte word boundary
        rem = len(cmd_bytes) % 4
        if rem != 0:
            cmd_bytes += b"\x00" * (4 - rem)
        words_count = 2 + (len(cmd_bytes) // 4)
        self.tags.extend(struct.pack("<II", words_count, ATAG_CMDLINE))
        self.tags.extend(cmd_bytes)
        return self

    def add_initrd2(self, start: int, size: int):
        """Appends ATAG_INITRD2 tag defining initial ramdisk."""
        self.tags.extend(struct.pack("<IIII", 4, ATAG_INITRD2, start, size))
        return self

    def add_serial(self, low: int, high: int):
        """Appends ATAG_SERIAL tag with 64-bit board serial."""
        self.tags.extend(struct.pack("<IIII", 4, ATAG_SERIAL, low, high))
        return self

    def add_revision(self, rev: int):
        """Appends ATAG_REVISION tag."""
        self.tags.extend(struct.pack("<III", 3, ATAG_REVISION, rev))
        return self

    def build(self) -> bytes:
        """Appends ATAG_NONE terminator and returns complete bytes."""
        out = bytearray(self.tags)
        # ATAG_NONE: size = 0, tag = 0
        out.extend(struct.pack("<II", 0, ATAG_NONE))
        return bytes(out)


class AtagsParser:
    """Parses binary ARM ATAGS lists."""

    @staticmethod
    def parse(data: bytes, base_addr: int = DEFAULT_ATAGS_ADDR) -> list:
        tags = []
        offset = 0
        while offset + 8 <= len(data):
            size_words, tag_id = struct.unpack_from("<II", data, offset)
            if size_words == 0 and tag_id == ATAG_NONE:
                tags.append({
                    "id": "ATAG_NONE",
                    "tag": ATAG_NONE,
                    "size_words": 0,
                    "offset": offset,
                    "addr": base_addr + offset
                })
                break

            tag_bytes_len = size_words * 4
            if offset + tag_bytes_len > len(data):
                break

            payload = data[offset + 8 : offset + tag_bytes_len]
            entry = {
                "tag": tag_id,
                "size_words": size_words,
                "offset": offset,
                "addr": base_addr + offset
            }

            if tag_id == ATAG_CORE:
                entry["id"] = "ATAG_CORE"
                if len(payload) >= 12:
                    flags, pagesize, rootdev = struct.unpack("<III", payload[:12])
                    entry["flags"] = flags
                    entry["pagesize"] = pagesize
                    entry["rootdev"] = rootdev
            elif tag_id == ATAG_MEM:
                entry["id"] = "ATAG_MEM"
                if len(payload) >= 8:
                    size, start = struct.unpack("<II", payload[:8])
                    entry["size"] = size
                    entry["start"] = start
            elif tag_id == ATAG_CMDLINE:
                entry["id"] = "ATAG_CMDLINE"
                entry["cmdline"] = payload.split(b"\x00")[0].decode("latin-1", errors="replace")
            elif tag_id == ATAG_INITRD2:
                entry["id"] = "ATAG_INITRD2"
                if len(payload) >= 8:
                    start, size = struct.unpack("<II", payload[:8])
                    entry["start"] = start
                    entry["size"] = size
            elif tag_id == ATAG_SERIAL:
                entry["id"] = "ATAG_SERIAL"
                if len(payload) >= 8:
                    low, high = struct.unpack("<II", payload[:8])
                    entry["serial"] = (high << 32) | low
            elif tag_id == ATAG_REVISION:
                entry["id"] = "ATAG_REVISION"
                if len(payload) >= 4:
                    entry["revision"] = struct.unpack("<I", payload[:4])[0]
            else:
                entry["id"] = f"ATAG_0x{tag_id:08X}"
                entry["raw"] = payload

            tags.append(entry)
            offset += tag_bytes_len

        return tags

    @staticmethod
    def dump_summary(tags: list):
        print("=" * 70)
        print(" ARM Boot Tags (ATAGS) Inspection")
        print("=" * 70)
        for t in tags:
            addr_str = f"0x{t['addr']:08X}"
            tid = t.get("id", "UNKNOWN")
            if tid == "ATAG_CORE":
                print(f"[{addr_str}] ATAG_CORE    : pagesize={t.get('pagesize')}, flags=0x{t.get('flags',0):08X}")
            elif tid == "ATAG_MEM":
                print(f"[{addr_str}] ATAG_MEM     : start=0x{t.get('start',0):08X}, size={t.get('size',0):,} bytes ({t.get('size',0)//(1024*1024)} MB)")
            elif tid == "ATAG_CMDLINE":
                print(f"[{addr_str}] ATAG_CMDLINE : \"{t.get('cmdline')}\"")
            elif tid == "ATAG_SERIAL":
                print(f"[{addr_str}] ATAG_SERIAL  : 0x{t.get('serial',0):016X}")
            elif tid == "ATAG_REVISION":
                print(f"[{addr_str}] ATAG_REVISION: 0x{t.get('revision',0):08X}")
            elif tid == "ATAG_INITRD2":
                print(f"[{addr_str}] ATAG_INITRD2 : start=0x{t.get('start',0):08X}, size={t.get('size',0)} bytes")
            elif tid == "ATAG_NONE":
                print(f"[{addr_str}] ATAG_NONE    : (Terminator)")
        print("=" * 70)


def create_standard_n2660_atags(dram_base: int = DEFAULT_DRAM_BASE,
                                dram_size: int = DEFAULT_DRAM_SIZE,
                                cmdline: str = DEFAULT_CMDLINE,
                                revision: int = 0x00000001,
                                serial: int = 0x2660148000000001) -> bytes:
    """Builds a complete, standard ATAGS list for Nokia 2660 Flip / Unisoc T107."""
    low_ser = serial & 0xFFFFFFFF
    high_ser = (serial >> 32) & 0xFFFFFFFF
    builder = AtagsBuilder()
    builder.add_core(flags=0, pagesize=4096, rootdev=0)
    builder.add_mem(size=dram_size, start=dram_base)
    builder.add_cmdline(cmdline)
    builder.add_revision(revision)
    builder.add_serial(low_ser, high_ser)
    return builder.build()


def main():
    parser = argparse.ArgumentParser(description="ARM Boot Tags (ATAGS) Generator and Inspector")
    subparsers = parser.add_subparsers(dest="subcommand", help="Subcommand")

    build_p = subparsers.add_parser("build", help="Generate binary ATAGS file")
    build_p.add_argument("-o", "--output", required=True, help="Output binary file")
    build_p.add_argument("--dram-base", type=lambda x: int(x, 0), default=DEFAULT_DRAM_BASE)
    build_p.add_argument("--dram-size", type=lambda x: int(x, 0), default=DEFAULT_DRAM_SIZE)
    build_p.add_argument("--cmdline", default=DEFAULT_CMDLINE)

    inspect_p = subparsers.add_parser("inspect", help="Inspect binary ATAGS file")
    inspect_p.add_argument("file", help="Path to binary file containing ATAGS")
    inspect_p.add_argument("--base", type=lambda x: int(x, 0), default=DEFAULT_ATAGS_ADDR)

    args = parser.parse_args()

    if args.subcommand == "build":
        data = create_standard_n2660_atags(dram_base=args.dram_base, dram_size=args.dram_size, cmdline=args.cmdline)
        with open(args.output, "wb") as f:
            f.write(data)
        print(f"[ATAGS] Wrote {len(data)} bytes of ATAGS to '{args.output}'.")
    elif args.subcommand == "inspect":
        with open(args.file, "rb") as f:
            data = f.read()
        tags = AtagsParser.parse(data, base_addr=args.base)
        AtagsParser.dump_summary(tags)

if __name__ == "__main__":
    main()
