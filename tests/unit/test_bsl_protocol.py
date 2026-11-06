#!/usr/bin/env python3
"""
Unit tests for Spreadtrum / Unisoc BSL (Boot Serial Loader) protocol engine.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import unittest
import struct
from tools.bsl_engine import (
    spd_crc16, spd_checksum, spd_transcode, spd_untranscode,
    encode_packet, decode_packet, BslPacketStream, BslServer, BslClient,
    BSL_CMD_CONNECT, BSL_CMD_START_DATA, BSL_CMD_MIDST_DATA,
    BSL_CMD_END_DATA, BSL_CMD_EXEC_DATA, BSL_CMD_CHECK_BAUD,
    BSL_CMD_READ_CHIP_TYPE,
    BSL_REP_ACK, BSL_REP_VER, BSL_REP_CHECKSUM_ERROR, HDLC_HEADER, HDLC_ESCAPE
)

class TestBslProtocol(unittest.TestCase):

    def test_crc16_calculation(self):
        """Test CCITT-16 CRC calculation matching Spreadtrum BootROM."""
        data = b"SPRD3\x00\x01\x02\x03Hello"
        crc = spd_crc16(0, data)
        self.assertEqual(crc, 0xEBB5)

        # Empty data should return 0
        self.assertEqual(spd_crc16(0, b""), 0)

        # 4 zeroes test vector
        self.assertEqual(spd_crc16(0, b"\x00\x00\x00\x00"), 0x0000)

    def test_checksum_calculation(self):
        """Test Spreadtrum 16-bit addition checksum calculation."""
        data = b"\x00\x01\x00\x08\x00\x00\x62\x00\x00\x00\x10\x00"
        chk = spd_checksum(0, data)
        self.assertEqual(chk, 0xF68D)

    def test_transcode_and_untranscode(self):
        """Test HDLC byte stuffing and escape reversal."""
        raw = bytes([HDLC_HEADER, 0x10, HDLC_ESCAPE, 0x20, 0x00, HDLC_HEADER, HDLC_ESCAPE])
        transcoded = spd_transcode(raw)

        # Ensure no unescaped 0x7E or 0x7D remain in the body
        self.assertNotIn(HDLC_HEADER, transcoded)
        # Check that escapes are followed by byte ^ 0x20
        self.assertEqual(transcoded[0], HDLC_ESCAPE)
        self.assertEqual(transcoded[1], HDLC_HEADER ^ 0x20)

        untranscoded = spd_untranscode(transcoded)
        self.assertEqual(untranscoded, raw)

    def test_encode_and_decode_connect(self):
        """Test encoding and decoding of BSL_CMD_CONNECT packet."""
        pkt = encode_packet(BSL_CMD_CONNECT, b"", use_crc16=True)
        self.assertTrue(pkt.startswith(bytes([HDLC_HEADER])))
        self.assertTrue(pkt.endswith(bytes([HDLC_HEADER])))

        cmd, payload, crc_ok = decode_packet(pkt, use_crc16=True)
        self.assertEqual(cmd, BSL_CMD_CONNECT)
        self.assertEqual(payload, b"")
        self.assertTrue(crc_ok)

    def test_encode_and_decode_start_data(self):
        """Test BSL_CMD_START_DATA with load address and file size."""
        load_addr = 0x00006200
        file_size = 0x00004500
        payload = struct.pack(">II", load_addr, file_size)

        pkt = encode_packet(BSL_CMD_START_DATA, payload, use_crc16=True)
        cmd, out_payload, crc_ok = decode_packet(pkt, use_crc16=True)

        self.assertEqual(cmd, BSL_CMD_START_DATA)
        self.assertTrue(crc_ok)
        out_addr, out_size = struct.unpack(">II", out_payload)
        self.assertEqual(out_addr, load_addr)
        self.assertEqual(out_size, file_size)

    def test_decode_corrupted_packet(self):
        """Test that corrupted CRC is flagged properly."""
        pkt = bytearray(encode_packet(BSL_CMD_CONNECT, b"", use_crc16=True))
        # Corrupt a byte in the packet body
        pkt[2] ^= 0xFF
        cmd, payload, crc_ok = decode_packet(bytes(pkt), use_crc16=True)
        self.assertFalse(crc_ok)

    def test_packet_stream_fragmentation(self):
        """Test BslPacketStream reassembly with arbitrary fragmentation."""
        stream = BslPacketStream()
        pkt1 = encode_packet(BSL_CMD_CONNECT, b"")
        pkt2 = encode_packet(BSL_REP_ACK, b"")
        combined = pkt1 + pkt2

        # Feed in 2-byte fragments
        frames = []
        for i in range(0, len(combined), 2):
            frames.extend(stream.feed(combined[i : i + 2]))

        self.assertEqual(len(frames), 2)
        cmd1, _, ok1 = decode_packet(frames[0])
        cmd2, _, ok2 = decode_packet(frames[1])
        self.assertEqual(cmd1, BSL_CMD_CONNECT)
        self.assertTrue(ok1)
        self.assertEqual(cmd2, BSL_REP_ACK)
        self.assertTrue(ok2)

    def test_bsl_client_server_flow(self):
        """Test complete client-server BSL loading session."""
        memory = bytearray(64 * 1024)

        def mem_writer(addr, data):
            memory[addr : addr + len(data)] = data

        server = BslServer(write_mem_fn=mem_writer, version_string="SPRD3\x00")
        client = BslClient(use_crc16=True)

        # 1. Baud check
        check_baud = client.make_check_baud(1)
        resps = server.process_incoming(check_baud)
        self.assertEqual(len(resps), 1)
        cmd, ver, ok = decode_packet(resps[0])
        self.assertEqual(cmd, BSL_REP_VER)
        self.assertEqual(ver, b"SPRD3\x00")
        self.assertTrue(ok)

        # 2. Upload a 256-byte mock FDL1 payload to 0x00006200
        test_payload = bytes([(i * 7) & 0xFF for i in range(256)])
        stream = client.build_file_transfer_stream(test_payload, target_addr=0x00006200, chunk_size=64)

        for pkt in stream:
            resps = server.process_incoming(pkt)
            self.assertEqual(len(resps), 1)
            cmd, _, ok = decode_packet(resps[0])
            self.assertEqual(cmd, BSL_REP_ACK)
            self.assertTrue(ok)

        # Verify that memory at 0x00006200 matches test_payload
        self.assertEqual(memory[0x00006200 : 0x00006200 + 256], test_payload)
        # Verify that execution address was set
        self.assertEqual(server.execution_address, 0x00006200)

    def test_chip_type_query(self):
        """Test BSL_CMD_READ_CHIP_TYPE command."""
        server = BslServer(write_mem_fn=lambda a, d: None)
        pkt = encode_packet(BSL_CMD_READ_CHIP_TYPE, b"")
        resps = server.process_incoming(pkt)
        self.assertEqual(len(resps), 1)
        cmd, payload, ok = decode_packet(resps[0])
        self.assertEqual(cmd, BSL_REP_ACK)
        self.assertTrue(ok)
        chip_id = struct.unpack(">I", payload)[0]
        self.assertEqual(chip_id, 0x91170001)

    def test_unicorn_vm_fdl_loading(self):
        """Test FDL1 and FDL2 loading methods in T107UnicornVM."""
        import tempfile
        import os
        from tools.unicorn_runner import T107UnicornVM

        vm = T107UnicornVM()
        with tempfile.NamedTemporaryFile(delete=False) as f1, tempfile.NamedTemporaryFile(delete=False) as f2:
            fdl1_data = b"\x01\x02\x03\x04" * 16
            fdl2_data = b"\xAA\xBB\xCC\xDD" * 16
            f1.write(fdl1_data)
            f1.flush()
            f2.write(fdl2_data)
            f2.flush()
            f1_path = f1.name
            f2_path = f2.name

        try:
            # Test direct load
            vm.load_fdl1(f1_path, load_addr=0x00006200, simulate_bsl=False)
            read_back_fdl1 = vm.uc.mem_read(0x00006200, len(fdl1_data))
            self.assertEqual(bytes(read_back_fdl1), fdl1_data)

            # Test BSL simulation load
            vm.load_fdl1(f1_path, load_addr=0x00006200, simulate_bsl=True)
            read_back_fdl1_bsl = vm.uc.mem_read(0x00006200, len(fdl1_data))
            self.assertEqual(bytes(read_back_fdl1_bsl), fdl1_data)

            # Test FDL2 load into DRAM
            vm.load_fdl2(f2_path, load_addr=0x80100000)
            read_back_fdl2 = vm.uc.mem_read(0x80100000, len(fdl2_data))
            self.assertEqual(bytes(read_back_fdl2), fdl2_data)
        finally:
            os.remove(f1_path)
            os.remove(f2_path)

if __name__ == "__main__":
    unittest.main()
