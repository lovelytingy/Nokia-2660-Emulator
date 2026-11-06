#!/usr/bin/env python3
"""
Unisoc / Spreadtrum .PAC Firmware Container Extractor & Builder
Parses, extracts, inspects, and packages multi-partition firmware archives
for Unisoc T107 (UMS9117) and feature phones (Nokia 2660 Flip).

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import os
import sys
import struct
import argparse
import xml.etree.ElementTree as ET

PAC_MAGIC = 0xFFFAFFFA
DEFAULT_PAC_VERSION = "BP_R1.0.0"
PAC_HEADER_RECORD_SIZE = 2124  # Standard file entry record size in Unisoc tools
PAC_STRING_LEN = 256           # 256 wchar_t (512 bytes)

def encode_pac_str(s: str, byte_len: int = 512) -> bytes:
    """Encodes a string to null-padded UTF-16LE bytes."""
    encoded = s.encode("utf-16le")
    if len(encoded) >= byte_len:
        return encoded[: byte_len - 2] + b"\x00\x00"
    return encoded + b"\x00" * (byte_len - len(encoded))

def decode_pac_str(raw: bytes) -> str:
    """Decodes null-terminated UTF-16LE bytes to a Python string."""
    try:
        decoded = raw.decode("utf-16le", errors="ignore")
        return decoded.split("\x00")[0]
    except Exception:
        return ""

class PacExtractor:
    """Parses and extracts Unisoc / Spreadtrum .PAC firmware packages."""

    def __init__(self, pac_path: str):
        self.pac_path = pac_path
        self.files = []
        self.product_name = ""
        self.product_version = ""
        self.pac_version = ""
        self.total_size = 0
        self.file_count = 0
        self.file_offset = 0
        self.flash_type = 0
        self.is_parsed = False

    def parse(self):
        """Reads and validates the PAC header and file entry table."""
        if not os.path.isfile(self.pac_path):
            raise FileNotFoundError(f"PAC file not found: {self.pac_path}")

        file_size = os.path.getsize(self.pac_path)
        if file_size < 1024:
            raise ValueError(f"File too small to be a valid PAC archive ({file_size} bytes)")

        with open(self.pac_path, "rb") as f:
            # Parse main PAC header
            # Offset 0: szVersion (96 bytes = 48 wchar) or 48 bytes
            version_raw = f.read(96)
            self.pac_version = decode_pac_str(version_raw)

            # Offset 96: dwSize (4 bytes)
            self.total_size = struct.unpack("<I", f.read(4))[0]

            # Offset 100: szPrdName (512 bytes = 256 wchar)
            self.product_name = decode_pac_str(f.read(512))

            # Offset 612: szPrdVersion (512 bytes = 256 wchar)
            self.product_version = decode_pac_str(f.read(512))

            # Offset 1124: nFileCount (4 bytes)
            self.file_count = struct.unpack("<I", f.read(4))[0]

            # Offset 1128: dwFileOffset (4 bytes)
            self.file_offset = struct.unpack("<I", f.read(4))[0]

            # Offset 1132: dwMode (4 bytes)
            mode = struct.unpack("<I", f.read(4))[0]

            # Offset 1136: dwFlashType (4 bytes)
            self.flash_type = struct.unpack("<I", f.read(4))[0]

            if self.file_count > 1024 or self.file_offset >= file_size:
                # Attempt alternative header offset heuristic (older tools with 48-byte version)
                f.seek(48)
                alt_total = struct.unpack("<I", f.read(4))[0]
                alt_prd = decode_pac_str(f.read(512))
                alt_ver = decode_pac_str(f.read(512))
                alt_count = struct.unpack("<I", f.read(4))[0]
                alt_offset = struct.unpack("<I", f.read(4))[0]
                if 0 < alt_count < 1024 and alt_offset < file_size:
                    self.total_size = alt_total
                    self.product_name = alt_prd
                    self.product_version = alt_ver
                    self.file_count = alt_count
                    self.file_offset = alt_offset

            # Seek to file table and parse records
            f.seek(self.file_offset)
            self.files = []
            for i in range(self.file_count):
                rec_start = f.tell()
                if rec_start + 4 > file_size:
                    break

                rec_size = struct.unpack("<I", f.read(4))[0]
                if rec_size == 0 or rec_size > 65536:
                    rec_size = PAC_HEADER_RECORD_SIZE

                file_id = decode_pac_str(f.read(512))
                file_name = decode_pac_str(f.read(512))
                file_ver = decode_pac_str(f.read(512))

                # nFileSize: 8 bytes (lo 4 bytes, hi 4 bytes)
                lo_size, hi_size = struct.unpack("<II", f.read(8))
                data_size = lo_size | (hi_size << 32)

                # nFileFlag (4 bytes), nCheckFlag (4 bytes), dwDataOffset (4 bytes)
                file_flag, check_flag, data_offset = struct.unpack("<III", f.read(12))

                # dwCanOmitFlag (4 bytes), dwAddrNum (4 bytes)
                can_omit, addr_num = struct.unpack("<II", f.read(8))

                # dwLoadAddr[5] (5 * 4 bytes = 20 bytes)
                load_addrs = list(struct.unpack("<5I", f.read(20)))
                primary_load_addr = load_addrs[0] if addr_num > 0 else 0

                # Seek to start of next record
                f.seek(rec_start + rec_size)

                self.files.append({
                    "index": i,
                    "id": file_id or f"PART_{i}",
                    "filename": os.path.basename(file_name) if file_name else f"part_{i}.bin",
                    "orig_path": file_name,
                    "version": file_ver,
                    "size": data_size,
                    "offset": data_offset,
                    "load_addr": primary_load_addr,
                    "load_addrs": load_addrs,
                    "active": (file_flag == 1 or data_size > 0),
                    "can_omit": can_omit
                })

        self.is_parsed = True
        return len(self.files)

    def list_partitions(self, verbose: bool = False):
        """Prints a human-readable summary of the PAC package and its partitions."""
        if not self.is_parsed:
            self.parse()

        flash_types = {0: "NAND Flash", 1: "NOR Flash", 2: "eMMC Flash", 3: "SPI NAND Flash"}
        flash_desc = flash_types.get(self.flash_type, f"Type {self.flash_type}")

        print("=" * 80)
        print(f" Unisoc .PAC Archive : {os.path.basename(self.pac_path)}")
        print(f" Target Product      : {self.product_name or 'Nokia 2660 Flip (UMS9117)'}")
        print(f" Product Version     : {self.product_version or 'N/A'}")
        print(f" Archive Version     : {self.pac_version or 'BP_R1.0.0'}")
        print(f" Target Flash Type   : {flash_desc}")
        print(f" Total File Size     : {os.path.getsize(self.pac_path):,} bytes")
        print(f" Partition Count     : {len(self.files)}")
        print("=" * 80)
        print(f"{'Idx':<4} {'Partition ID':<18} {'Filename':<24} {'Size':<12} {'Load Addr':<12} {'Offset':<10}")
        print("-" * 80)
        for part in self.files:
            load_str = f"0x{part['load_addr']:08X}" if part['load_addr'] else "-"
            offset_str = f"0x{part['offset']:08X}" if part['offset'] else "-"
            size_str = f"{part['size']:,} B" if part['size'] > 0 else "0 B (omitted)"
            print(f"{part['index']:<4} {part['id']:<18} {part['filename']:<24} {size_str:<12} {load_str:<12} {offset_str:<10}")
        print("-" * 80)

    def extract_file(self, target_id_or_name: str, out_path: str) -> bool:
        """Extracts a single partition identified by ID or filename."""
        if not self.is_parsed:
            self.parse()

        target = None
        for part in self.files:
            if part["id"].lower() == target_id_or_name.lower() or part["filename"].lower() == target_id_or_name.lower():
                target = part
                break

        if not target:
            raise KeyError(f"Partition '{target_id_or_name}' not found in PAC file.")

        if target["size"] == 0 or target["offset"] == 0:
            print(f"[PAC Warning] Partition '{target['id']}' has 0 bytes or omitted offset.")
            return False

        os.makedirs(os.path.dirname(os.path.abspath(out_path)) or ".", exist_ok=True)
        with open(self.pac_path, "rb") as f, open(out_path, "wb") as out_f:
            f.seek(target["offset"])
            data = f.read(target["size"])
            out_f.write(data)

        print(f"[PAC] Extracted '{target['id']}' ({target['size']:,} bytes) -> '{out_path}'")
        return True

    def extract_all(self, out_dir: str):
        """Extracts all active partitions to the specified directory."""
        if not self.is_parsed:
            self.parse()

        os.makedirs(out_dir, exist_ok=True)
        extracted_count = 0
        with open(self.pac_path, "rb") as f:
            for part in self.files:
                if part["size"] == 0 or part["offset"] == 0:
                    continue
                out_path = os.path.join(out_dir, part["filename"])
                f.seek(part["offset"])
                data = f.read(part["size"])
                with open(out_path, "wb") as out_f:
                    out_f.write(data)
                print(f"[PAC] Extracted {part['id']:<16} ({part['size']:>10,} bytes) -> '{part['filename']}'")
                extracted_count += 1

        print(f"[PAC] Extraction complete: {extracted_count} partition files saved to '{out_dir}'.")

    def parse_partition_xml(self) -> dict:
        """Finds and parses embedded partition XML (if available) to reconstruct flash layout."""
        if not self.is_parsed:
            self.parse()

        xml_part = None
        for p in self.files:
            if "xml" in p["id"].lower() or p["filename"].lower().endswith(".xml"):
                xml_part = p
                break

        if not xml_part or xml_part["size"] == 0:
            return {}

        with open(self.pac_path, "rb") as f:
            f.seek(xml_part["offset"])
            xml_raw = f.read(xml_part["size"])

        try:
            root = ET.fromstring(xml_raw.decode("utf-8", errors="ignore"))
            partitions = []
            for elem in root.iter():
                if "id" in elem.attrib or "name" in elem.attrib:
                    part_id = elem.attrib.get("id") or elem.attrib.get("name")
                    size = elem.attrib.get("size", "0")
                    partitions.append({"id": part_id, "size": size, "attribs": elem.attrib})
            return {"partitions": partitions, "raw_xml": xml_raw.decode("utf-8", errors="ignore")}
        except Exception as e:
            return {"error": str(e), "raw_xml": xml_raw.decode("utf-8", errors="ignore")}


class PacPacker:
    """Builds valid Unisoc / Spreadtrum .PAC firmware containers."""

    @staticmethod
    def pack(out_pac_path: str, product_name: str, product_version: str,
             files_to_pack: list, pac_version: str = DEFAULT_PAC_VERSION,
             flash_type: int = 3):
        """
        Creates a compliant .PAC archive.
        files_to_pack: list of dicts:
          [
            {"id": "FDL1", "path": "path/to/fdl1.bin", "load_addr": 0x00006200},
            {"id": "BOOT0", "path": "path/to/boot0.bin", "load_addr": 0x00006200},
            ...
          ]
        """
        file_count = len(files_to_pack)

        # Calculate layout:
        # Header size: 2124 bytes
        # File records size: file_count * PAC_HEADER_RECORD_SIZE (2124 bytes)
        # Payloads start immediately after file table
        header_size = 2124
        file_table_offset = header_size
        file_table_size = file_count * PAC_HEADER_RECORD_SIZE
        payload_start_offset = file_table_offset + file_table_size

        # Prepare payload offsets
        current_offset = payload_start_offset
        prepared_files = []
        for item in files_to_pack:
            file_path = item["path"]
            file_size = os.path.getsize(file_path) if os.path.isfile(file_path) else 0
            file_id = item.get("id", os.path.splitext(os.path.basename(file_path))[0].upper())
            load_addr = item.get("load_addr", 0)

            prepared_files.append({
                "id": file_id,
                "filename": os.path.basename(file_path),
                "path": file_path,
                "size": file_size,
                "offset": current_offset if file_size > 0 else 0,
                "load_addr": load_addr
            })
            if file_size > 0:
                # 4-byte align payload offsets
                aligned_size = (file_size + 3) & ~3
                current_offset += aligned_size

        total_pac_size = current_offset

        # Build Main PAC Header (2124 bytes)
        hdr = bytearray(header_size)
        # szVersion (96 bytes UTF-16LE)
        hdr[0:96] = encode_pac_str(pac_version, 96)
        # dwSize (uint32)
        struct.pack_into("<I", hdr, 96, total_pac_size)
        # szPrdName (512 bytes UTF-16LE)
        hdr[100:612] = encode_pac_str(product_name, 512)
        # szPrdVersion (512 bytes UTF-16LE)
        hdr[612:1124] = encode_pac_str(product_version, 512)
        # nFileCount (uint32)
        struct.pack_into("<I", hdr, 1124, file_count)
        # dwFileOffset (uint32)
        struct.pack_into("<I", hdr, 1128, file_table_offset)
        # dwMode (uint32)
        struct.pack_into("<I", hdr, 1132, 0)
        # dwFlashType (uint32, 3 = SPI NAND)
        struct.pack_into("<I", hdr, 1136, flash_type)
        # dwMagic at offset 1952 (0x7A0)
        struct.pack_into("<I", hdr, 1952, PAC_MAGIC)

        os.makedirs(os.path.dirname(os.path.abspath(out_pac_path)) or ".", exist_ok=True)
        with open(out_pac_path, "wb") as f_out:
            # Write Main Header
            f_out.write(hdr)

            # Write File Entries Table
            for item in prepared_files:
                rec = bytearray(PAC_HEADER_RECORD_SIZE)
                # dwSize
                struct.pack_into("<I", rec, 0, PAC_HEADER_RECORD_SIZE)
                # szFileID (512 bytes)
                rec[4:516] = encode_pac_str(item["id"], 512)
                # szFileName (512 bytes)
                rec[516:1028] = encode_pac_str(item["filename"], 512)
                # szFileVersion (512 bytes)
                rec[1028:1540] = encode_pac_str(product_version, 512)
                # nFileSize (uint64)
                lo = item["size"] & 0xFFFFFFFF
                hi = (item["size"] >> 32) & 0xFFFFFFFF
                struct.pack_into("<II", rec, 1540, lo, hi)
                # nFileFlag, nCheckFlag, dwDataOffset
                struct.pack_into("<III", rec, 1548, 1 if item["size"] > 0 else 0, 0, item["offset"])
                # dwCanOmitFlag, dwAddrNum
                addr_num = 1 if item["load_addr"] > 0 else 0
                struct.pack_into("<II", rec, 1560, 0, addr_num)
                # dwLoadAddr[5]
                struct.pack_into("<5I", rec, 1568, item["load_addr"], 0, 0, 0, 0)

                f_out.write(rec)

            # Write Payloads
            for item in prepared_files:
                if item["size"] > 0 and os.path.isfile(item["path"]):
                    with open(item["path"], "rb") as f_in:
                        data = f_in.read()
                        f_out.write(data)
                        # Padding for 4-byte alignment
                        rem = len(data) % 4
                        if rem != 0:
                            f_out.write(b"\x00" * (4 - rem))

        print(f"[PacPacker] Successfully generated PAC archive '{out_pac_path}':")
        print(f"  Product     : {product_name} ({product_version})")
        print(f"  Partitions  : {file_count}")
        print(f"  Total Size  : {total_pac_size:,} bytes")
        return total_pac_size


def main():
    parser = argparse.ArgumentParser(description="Unisoc .PAC Firmware Container Extractor & Builder")
    subparsers = parser.add_subparsers(dest="subcommand", help="Subcommand (extract / list / pack / dump-xml)")

    # Extraction mode (default if no subcommand)
    extract_parser = subparsers.add_parser("extract", help="Extract partitions from a .PAC firmware archive")
    extract_parser.add_argument("pac_file", help="Path to input .PAC firmware container")
    extract_parser.add_argument("-o", "--output", default="extracted_fw", help="Destination directory for extracted images")
    extract_parser.add_argument("--id", help="Extract only specific partition by ID (e.g. FDL1, BOOT0, KERNEL)")

    # Listing mode
    list_parser = subparsers.add_parser("list", help="List partition table inside a .PAC archive")
    list_parser.add_argument("pac_file", help="Path to input .PAC firmware container")
    list_parser.add_argument("-v", "--verbose", action="store_true", help="Show verbose partition details")

    # Dump XML mode
    xml_parser = subparsers.add_parser("dump-xml", help="Dump embedded partition layout XML")
    xml_parser.add_argument("pac_file", help="Path to input .PAC firmware container")

    # Packaging mode
    pack_parser = subparsers.add_parser("pack", help="Create a new .PAC firmware archive")
    pack_parser.add_argument("-o", "--output", required=True, help="Path to output .PAC file")
    pack_parser.add_argument("--name", default="Nokia 2660 Flip", help="Product name string")
    pack_parser.add_argument("--version", default="TA-1480_v1.0", help="Firmware release version")
    pack_parser.add_argument("-f", "--file", action="append", required=True,
                             help="Add partition in format: ID:PATH[:LOAD_ADDR] (e.g. FDL1:fdl1.bin:0x00006200)")

    # If first argument looks like a .pac file or -l without subcommand, dispatch nicely
    if len(sys.argv) > 1 and not sys.argv[1].startswith("-") and sys.argv[1] not in ("extract", "list", "pack", "dump-xml"):
        # Legacy positional invocation: pac_extractor.py <file.pac> [-l] [-o <dir>]
        legacy_parser = argparse.ArgumentParser()
        legacy_parser.add_argument("pac_file")
        legacy_parser.add_argument("-l", "--list", action="store_true")
        legacy_parser.add_argument("-o", "--output", default="extracted_fw")
        legacy_parser.add_argument("--id", default=None)
        args = legacy_parser.parse_args()

        ext = PacExtractor(args.pac_file)
        ext.parse()
        if args.list:
            ext.list_partitions()
        elif args.id:
            out_file = os.path.join(args.output, f"{args.id.lower()}.bin")
            ext.extract_file(args.id, out_file)
        else:
            ext.list_partitions()
            ext.extract_all(args.output)
        return

    args = parser.parse_args()

    if args.subcommand == "list":
        ext = PacExtractor(args.pac_file)
        ext.parse()
        ext.list_partitions(verbose=args.verbose)

    elif args.subcommand == "extract":
        ext = PacExtractor(args.pac_file)
        ext.parse()
        if args.id:
            out_file = os.path.join(args.output, f"{args.id.lower()}.bin")
            ext.extract_file(args.id, out_file)
        else:
            ext.list_partitions()
            ext.extract_all(args.output)

    elif args.subcommand == "dump-xml":
        ext = PacExtractor(args.pac_file)
        ext.parse()
        info = ext.parse_partition_xml()
        if "raw_xml" in info:
            print(info["raw_xml"])
        else:
            print("No partition XML found in archive.")

    elif args.subcommand == "pack":
        files = []
        for item in args.file:
            parts = item.split(":")
            if len(parts) >= 2:
                file_id = parts[0]
                file_path = parts[1]
                load_addr = int(parts[2], 0) if len(parts) >= 3 else 0
                files.append({"id": file_id, "path": file_path, "load_addr": load_addr})
            else:
                print(f"Error: Invalid file specification '{item}'. Use ID:PATH[:LOAD_ADDR]")
                sys.exit(1)

        PacPacker.pack(args.output, args.name, args.version, files)

if __name__ == "__main__":
    main()
