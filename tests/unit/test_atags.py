"""
Unit tests for ARM Boot Tags (ATAGS) generation, parsing, and kernel handoff.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import unittest
import os
import sys
import struct
import tempfile
import subprocess

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from tools.atags import (
    ATAG_NONE, ATAG_CORE, ATAG_MEM, ATAG_CMDLINE, ATAG_SERIAL, ATAG_REVISION, ATAG_INITRD2,
    DEFAULT_BOARD_ID, DEFAULT_ATAGS_ADDR, DEFAULT_DRAM_BASE, DEFAULT_DRAM_SIZE, DEFAULT_CMDLINE,
    AtagsBuilder, AtagsParser, create_standard_n2660_atags
)
from tools.unicorn_runner import T107UnicornVM, UC_ARM_REG_R0, UC_ARM_REG_R1, UC_ARM_REG_R2

class TestAtagsAndKernelHandoff(unittest.TestCase):

    def test_builder_individual_tags(self):
        """Test builder tag generation and binary layout."""
        builder = AtagsBuilder()
        builder.add_core(flags=0x12, pagesize=8192, rootdev=0x0102)
        builder.add_mem(size=32 * 1024 * 1024, start=0x80000000)
        builder.add_revision(0x00000002)
        builder.add_serial(0x11223344, 0x55667788)
        builder.add_initrd2(start=0x80800000, size=0x00200000)
        raw = builder.build()

        tags = AtagsParser.parse(raw, base_addr=0x80000100)
        tag_ids = [t["id"] for t in tags]
        self.assertEqual(tag_ids, ["ATAG_CORE", "ATAG_MEM", "ATAG_REVISION", "ATAG_SERIAL", "ATAG_INITRD2", "ATAG_NONE"])

        core = tags[0]
        self.assertEqual(core["pagesize"], 8192)
        self.assertEqual(core["flags"], 0x12)
        self.assertEqual(core["rootdev"], 0x0102)

        mem = tags[1]
        self.assertEqual(mem["start"], 0x80000000)
        self.assertEqual(mem["size"], 32 * 1024 * 1024)

        rev = tags[2]
        self.assertEqual(rev["revision"], 2)

        ser = tags[3]
        self.assertEqual(ser["serial"], (0x55667788 << 32) | 0x11223344)

        initrd = tags[4]
        self.assertEqual(initrd["start"], 0x80800000)
        self.assertEqual(initrd["size"], 0x00200000)

    def test_cmdline_padding_and_alignment(self):
        """Verify ATAG_CMDLINE strings are properly 4-byte word aligned and null terminated."""
        builder = AtagsBuilder()
        builder.add_core()
        builder.add_cmdline("test=1")  # len("test=1\0") = 7 bytes -> should pad to 8 bytes (2 words)
        raw = builder.build()

        # Check raw length is a multiple of 4
        self.assertEqual(len(raw) % 4, 0)

        tags = AtagsParser.parse(raw)
        cmd_tag = [t for t in tags if t["id"] == "ATAG_CMDLINE"][0]
        self.assertEqual(cmd_tag["cmdline"], "test=1")
        # Header (2 words) + 2 words payload = 4 words = 16 bytes
        self.assertEqual(cmd_tag["size_words"], 4)

    def test_standard_n2660_atags(self):
        """Test standard Nokia 2660 Flip ATAGS synthesizer."""
        raw = create_standard_n2660_atags(
            dram_base=DEFAULT_DRAM_BASE,
            dram_size=DEFAULT_DRAM_SIZE,
            cmdline=DEFAULT_CMDLINE,
            revision=1,
            serial=0x2660148000000001
        )
        self.assertGreater(len(raw), 0)
        self.assertEqual(len(raw) % 4, 0)

        tags = AtagsParser.parse(raw)
        self.assertEqual(tags[0]["id"], "ATAG_CORE")
        self.assertEqual(tags[1]["id"], "ATAG_MEM")
        self.assertEqual(tags[1]["start"], 0x80000000)
        self.assertEqual(tags[1]["size"], 48 * 1024 * 1024)
        self.assertEqual(tags[2]["id"], "ATAG_CMDLINE")
        self.assertEqual(tags[2]["cmdline"], DEFAULT_CMDLINE)
        self.assertEqual(tags[3]["id"], "ATAG_REVISION")
        self.assertEqual(tags[3]["revision"], 1)
        self.assertEqual(tags[4]["id"], "ATAG_SERIAL")
        self.assertEqual(tags[4]["serial"], 0x2660148000000001)
        self.assertEqual(tags[5]["id"], "ATAG_NONE")

    def test_vm_staging_and_register_setup(self):
        """Verify VM setup_atags stages data into DRAM and initializes r0, r1, r2 registers."""
        vm = T107UnicornVM()
        staged_addr = 0x80030000
        test_cmd = "console=ttyS0,115200 root=/dev/mtdblock3 earlycon"
        length = vm.setup_atags(
            ram_base=0x80000000,
            ram_size=48 * 1024 * 1024,
            cmdline=test_cmd,
            atags_addr=staged_addr,
            board_id=DEFAULT_BOARD_ID
        )
        self.assertGreater(length, 0)

        # Verify CPU registers
        r0 = vm.uc.reg_read(UC_ARM_REG_R0)
        r1 = vm.uc.reg_read(UC_ARM_REG_R1)
        r2 = vm.uc.reg_read(UC_ARM_REG_R2)
        self.assertEqual(r0, 0, "r0 must be 0 for ARM Linux kernel handoff")
        self.assertEqual(r1, DEFAULT_BOARD_ID, "r1 must contain board architecture ID 0x9117")
        self.assertEqual(r2, staged_addr, "r2 must point to physical address of ATAGS")

        # Read back staged memory from Unicorn VM DRAM
        mem_data = bytes(vm.uc.mem_read(staged_addr, length))
        tags = AtagsParser.parse(mem_data, base_addr=staged_addr)
        self.assertEqual(tags[0]["id"], "ATAG_CORE")
        self.assertEqual(tags[1]["id"], "ATAG_MEM")
        self.assertEqual(tags[1]["size"], 48 * 1024 * 1024)
        self.assertEqual(tags[2]["id"], "ATAG_CMDLINE")
        self.assertEqual(tags[2]["cmdline"], test_cmd)
        self.assertEqual(tags[-1]["id"], "ATAG_NONE")

    def test_cli_build_and_inspect(self):
        """Test tools/atags.py CLI tool build and inspect subcommands."""
        with tempfile.NamedTemporaryFile(suffix=".bin", delete=False) as tf:
            out_file = tf.name

        try:
            # Build
            cmd_build = [
                sys.executable,
                os.path.join(PROJECT_ROOT, "tools", "atags.py"),
                "build",
                "-o", out_file,
                "--cmdline", "test=boot"
            ]
            res_build = subprocess.run(cmd_build, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            self.assertEqual(res_build.returncode, 0)
            self.assertTrue(os.path.isfile(out_file))

            # Inspect
            cmd_inspect = [
                sys.executable,
                os.path.join(PROJECT_ROOT, "tools", "atags.py"),
                "inspect",
                out_file
            ]
            res_inspect = subprocess.run(cmd_inspect, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            self.assertEqual(res_inspect.returncode, 0)
            self.assertIn("ATAG_CORE", res_inspect.stdout)
            self.assertIn("ATAG_MEM", res_inspect.stdout)
            self.assertIn("test=boot", res_inspect.stdout)
            self.assertIn("ATAG_NONE", res_inspect.stdout)
        finally:
            if os.path.exists(out_file):
                os.unlink(out_file)

if __name__ == "__main__":
    unittest.main()
