"""
Unit tests for Unisoc / Spreadtrum .PAC firmware packaging and extraction.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import unittest
import os
import sys
import tempfile
import subprocess

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from tools.pac_extractor import PacPacker, PacExtractor, encode_pac_str, decode_pac_str

class TestPacExtractor(unittest.TestCase):

    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.td = self.temp_dir.name

        self.fdl1_payload = b"\x2A\x00\xA0\xE3\xFE\xFF\xFF\xEA"  # mov r0, #42; b .
        self.fdl2_payload = b"\x55\xAA" * 128
        self.boot0_payload = b"BOOT0_TEST_IMAGE_PAYLOAD_BYTES_12345"
        self.xml_content = (
            b'<?xml version="1.0" encoding="utf-8"?>\n'
            b'<Partitions>\n'
            b'  <Partition id="FDL1" size="64K" load_addr="0x00006200"/>\n'
            b'  <Partition id="FDL2" size="512K" load_addr="0x80100000"/>\n'
            b'  <Partition id="BOOT0" size="128K" offset="0x00000000"/>\n'
            b'</Partitions>\n'
        )

        self.fdl1_path = os.path.join(self.td, "fdl1.bin")
        self.fdl2_path = os.path.join(self.td, "fdl2.bin")
        self.boot0_path = os.path.join(self.td, "boot0.bin")
        self.xml_path = os.path.join(self.td, "partition.xml")

        with open(self.fdl1_path, "wb") as f:
            f.write(self.fdl1_payload)
        with open(self.fdl2_path, "wb") as f:
            f.write(self.fdl2_payload)
        with open(self.boot0_path, "wb") as f:
            f.write(self.boot0_payload)
        with open(self.xml_path, "wb") as f:
            f.write(self.xml_content)

        self.pac_path = os.path.join(self.td, "test_nokia.pac")
        self.files_to_pack = [
            {"id": "FDL1", "path": self.fdl1_path, "load_addr": 0x00006200},
            {"id": "FDL2", "path": self.fdl2_path, "load_addr": 0x80100000},
            {"id": "BOOT0", "path": self.boot0_path, "load_addr": 0x00006200},
            {"id": "PARTITION_XML", "path": self.xml_path, "load_addr": 0}
        ]
        PacPacker.pack(
            self.pac_path,
            product_name="Nokia 2660 Flip",
            product_version="TA-1480_v20.00",
            files_to_pack=self.files_to_pack
        )

    def tearDown(self):
        self.temp_dir.cleanup()

    def test_pac_string_encoding_and_decoding(self):
        """Test UTF-16LE string encoding and null-termination decoding."""
        test_str = "UMS9117_Nokia_TA1480"
        raw = encode_pac_str(test_str, 512)
        self.assertEqual(len(raw), 512)
        decoded = decode_pac_str(raw)
        self.assertEqual(decoded, test_str)

    def test_pac_pack_and_parse_header(self):
        """Test packing a PAC container and parsing its header fields."""
        ext = PacExtractor(self.pac_path)
        count = ext.parse()
        self.assertEqual(count, 4)
        self.assertEqual(ext.product_name, "Nokia 2660 Flip")
        self.assertEqual(ext.product_version, "TA-1480_v20.00")
        self.assertEqual(ext.file_count, 4)
        self.assertGreater(ext.total_size, 0)

        # Verify partition records
        file_ids = [p["id"] for p in ext.files]
        self.assertIn("FDL1", file_ids)
        self.assertIn("FDL2", file_ids)
        self.assertIn("BOOT0", file_ids)
        self.assertIn("PARTITION_XML", file_ids)

        fdl1_part = next(p for p in ext.files if p["id"] == "FDL1")
        self.assertEqual(fdl1_part["load_addr"], 0x00006200)
        self.assertEqual(fdl1_part["size"], len(self.fdl1_payload))

        fdl2_part = next(p for p in ext.files if p["id"] == "FDL2")
        self.assertEqual(fdl2_part["load_addr"], 0x80100000)
        self.assertEqual(fdl2_part["size"], len(self.fdl2_payload))

    def test_pac_extract_all(self):
        """Test extracting all partitions and verifying byte-for-byte match."""
        out_dir = os.path.join(self.td, "extracted_all")
        ext = PacExtractor(self.pac_path)
        ext.extract_all(out_dir)

        with open(os.path.join(out_dir, "fdl1.bin"), "rb") as f:
            self.assertEqual(f.read(), self.fdl1_payload)
        with open(os.path.join(out_dir, "fdl2.bin"), "rb") as f:
            self.assertEqual(f.read(), self.fdl2_payload)
        with open(os.path.join(out_dir, "boot0.bin"), "rb") as f:
            self.assertEqual(f.read(), self.boot0_payload)
        with open(os.path.join(out_dir, "partition.xml"), "rb") as f:
            self.assertEqual(f.read(), self.xml_content)

    def test_pac_extract_single_file(self):
        """Test extracting a single partition by ID."""
        ext = PacExtractor(self.pac_path)
        single_out = os.path.join(self.td, "single_fdl1.bin")
        success = ext.extract_file("FDL1", single_out)
        self.assertTrue(success)
        with open(single_out, "rb") as f:
            self.assertEqual(f.read(), self.fdl1_payload)

        # Test non-existent partition raises KeyError
        with self.assertRaises(KeyError):
            ext.extract_file("NONEXISTENT_PART", os.path.join(self.td, "fail.bin"))

    def test_pac_parse_partition_xml(self):
        """Test parsing embedded partition layout XML."""
        ext = PacExtractor(self.pac_path)
        xml_data = ext.parse_partition_xml()
        self.assertIn("partitions", xml_data)
        part_ids = [p["id"] for p in xml_data["partitions"]]
        self.assertIn("FDL1", part_ids)
        self.assertIn("FDL2", part_ids)
        self.assertIn("BOOT0", part_ids)

    def test_pac_invalid_file_handling(self):
        """Test error handling for non-existent and corrupted PAC files."""
        with self.assertRaises(FileNotFoundError):
            ext = PacExtractor(os.path.join(self.td, "does_not_exist.pac"))
            ext.parse()

        # Truncated file
        trunc_path = os.path.join(self.td, "truncated.pac")
        with open(trunc_path, "wb") as f:
            f.write(b"SHORT")
        with self.assertRaises(ValueError):
            ext = PacExtractor(trunc_path)
            ext.parse()

    def test_n2660emu_cli_direct_pac_execution(self):
        """Test running n2660emu CLI directly on a .pac firmware container."""
        n2660emu_bin = os.path.join(PROJECT_ROOT, "bin", "n2660emu")
        res = subprocess.run(
            [sys.executable, n2660emu_bin, self.pac_path, "--bsl", "--timeout", "1"],
            capture_output=True,
            text=True
        )
        self.assertEqual(res.returncode, 0)
        self.assertIn("Detected Unisoc .PAC firmware container", res.stdout)
        self.assertIn("Simulating Spreadtrum BSL Protocol Handshake over UART0", res.stdout)
        self.assertIn("Emulation finished cleanly.", res.stdout)

if __name__ == "__main__":
    unittest.main()
