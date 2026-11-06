#!/usr/bin/env python3
"""
Spreadtrum / Unisoc .PAC Firmware Extractor
Parses and extracts partition images (FDL1, FDL2, BOOT0, BOOT1, Kernel)
from Nokia 2660 Flip (UMS9117) vendor PAC files.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import os
import sys
import struct
import argparse

def parse_pac_string(raw_bytes):
    """Decodes null-terminated UTF-16LE string from PAC header."""
    try:
        decoded = raw_bytes.decode('utf-16le', errors='ignore')
        return decoded.split('\x00')[0]
    except Exception:
        return ""

class PacExtractor:
    def __init__(self, pac_path):
        self.pac_path = pac_path
        self.files = []
        self.product_name = ""
        self.version = ""

    def parse(self):
        if not os.path.isfile(self.pac_path):
            raise FileNotFoundError(f"PAC file not found: {self.pac_path}")

        file_size = os.path.getsize(self.pac_path)
        with open(self.pac_path, "rb") as f:
            # PAC Header structure:
            # szVersion[48] (wchar) -> 96 bytes (or 48 bytes depending on version)
            # dwSize (4 bytes)
            # szPrdName[512] (wchar) -> 1024 bytes (or 512 bytes)
            header_magic = f.read(4)
            f.seek(0)

            # Detect PAC header variant
            # Check for "BP_R" or similar wchar string
            header_raw = f.read(2048)
            f.seek(0)

            # Typical BSL / Spreadtrum PAC header format:
            # offset 0: szVersion (96 bytes = 48 wchar)
            # offset 96: dwSize (uint32)
            # offset 100: szPrdName (512 bytes = 256 wchar or 1024 bytes)
            # offset 100+prd: szPrdVersion
            # offset: nFileCount (uint32)
            # offset: dwFileOffset (uint32)

            # Search for partition table
            self.product_name = parse_pac_string(header_raw[96:608])
            if not self.product_name:
                self.product_name = parse_pac_string(header_raw[48:300])

            # Locate file table by reading partition record count
            # In standard PAC v1/v2:
            try:
                # Seek to header fields
                f.seek(0)
                version_raw = f.read(96)
                self.version = parse_pac_string(version_raw)
                total_size = struct.unpack("<I", f.read(4))[0]
                prd_name = parse_pac_string(f.read(512))
                prd_version = parse_pac_string(f.read(512))
                file_count = struct.unpack("<I", f.read(4))[0]
                file_offset = struct.unpack("<I", f.read(4))[0]

                if 0 < file_count < 100 and file_offset < file_size:
                    self.product_name = prd_name
                    f.seek(file_offset)
                    for i in range(file_count):
                        rec_size = struct.unpack("<I", f.read(4))[0]
                        file_id = parse_pac_string(f.read(512))
                        file_name = parse_pac_string(f.read(512))
                        file_ver = parse_pac_string(f.read(512))
                        lo_size, hi_size = struct.unpack("<II", f.read(8))
                        data_size = lo_size | (hi_size << 32)
                        data_offset = struct.unpack("<I", f.read(4))[0]
                        # In many PAC variants, load address is present
                        load_addr = 0
                        remaining = rec_size - (4 + 512*3 + 8 + 4)
                        if remaining >= 4:
                            load_addr = struct.unpack("<I", f.read(4))[0]
                            if remaining > 4:
                                f.seek(remaining - 4, os.SEEK_CUR)
                        elif remaining > 0:
                            f.seek(remaining, os.SEEK_CUR)

                        self.files.append({
                            "id": file_id or f"PART_{i}",
                            "filename": os.path.basename(file_name) if file_name else f"part_{i}.bin",
                            "size": data_size,
                            "offset": data_offset,
                            "load_addr": load_addr
                        })
            except Exception as e:
                print(f"[PAC Warning] Standard parser fallback triggered: {e}")

    def list_partitions(self):
        print(f"PAC Image: {os.path.basename(self.pac_path)}")
        print(f"Product:   {self.product_name or 'Unisoc T107 / UMS9117'}")
        print(f"Version:   {self.version or 'Unknown'}")
        print(f"Partitions found: {len(self.files)}")
        print("-" * 75)
        print(f"{'Partition ID':<20} {'Filename':<25} {'Size (Bytes)':<14} {'Load Addr':<12}")
        print("-" * 75)
        for part in self.files:
            load_str = f"0x{part['load_addr']:08X}" if part['load_addr'] else "N/A"
            print(f"{part['id']:<20} {part['filename']:<25} {part['size']:<14} {load_str:<12}")

    def extract_all(self, out_dir):
        os.makedirs(out_dir, exist_ok=True)
        with open(self.pac_path, "rb") as f:
            for part in self.files:
                if part["size"] == 0 or part["offset"] == 0:
                    continue
                out_path = os.path.join(out_dir, part["filename"])
                print(f"Extracting {part['id']} -> {out_path} ({part['size']} bytes)...")
                f.seek(part["offset"])
                data = f.read(part["size"])
                with open(out_path, "wb") as out_f:
                    out_f.write(data)
        print(f"Extraction complete to '{out_dir}'.")

def main():
    parser = argparse.ArgumentParser(description="Unisoc .PAC Firmware Extractor for Nokia 2660 Flip")
    parser.add_argument("pac_file", help="Path to input .PAC firmware file")
    parser.add_argument("-l", "--list", action="store_true", help="List partitions without extracting")
    parser.add_argument("-o", "--output", default="extracted_fw", help="Output directory for extracted files")
    args = parser.parse_args()

    extractor = PacExtractor(args.pac_file)
    extractor.parse()
    extractor.list_partitions()

    if not args.list:
        extractor.extract_all(args.output)

if __name__ == "__main__":
    main()
