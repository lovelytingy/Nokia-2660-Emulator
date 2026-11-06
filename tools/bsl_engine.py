#!/usr/bin/env python3
"""
Spreadtrum / Unisoc Boot Serial Loader (BSL) Protocol Engine
Implements HDLC packet framing, byte-stuffing/transcoding, CRC16 calculation,
and BSL command/response message structures for Nokia 2660 Flip (UMS9117).

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import struct
from typing import Tuple, Optional, List, Dict, Any, Callable

# HDLC Protocol Constants
HDLC_HEADER = 0x7E  # Frame boundary delimiter flag
HDLC_ESCAPE = 0x7D  # Escape prefix character

# BSL Host Command Opcodes
BSL_CMD_CONNECT         = 0x00  # Initial handshake / link establish
BSL_CMD_START_DATA      = 0x01  # Begin data transfer (addr, size)
BSL_CMD_MIDST_DATA      = 0x02  # Stream data payload chunk
BSL_CMD_END_DATA        = 0x03  # Finalize data transfer & verify
BSL_CMD_EXEC_DATA       = 0x04  # Jump CPU execution to specified address
BSL_CMD_NORMAL_RESET    = 0x05  # Reset target to normal mode
BSL_CMD_READ_FLASH      = 0x06  # Read flash contents
BSL_CMD_READ_CHIP_TYPE  = 0x07  # Read SoC / Chip ID
BSL_CMD_READ_NVITEM     = 0x08  # Lookup NVRAM item
BSL_CMD_CHANGE_BAUD     = 0x09  # Change UART baud rate
BSL_CMD_ERASE_FLASH     = 0x0A  # Erase flash partition / region
BSL_CMD_REPARTITION     = 0x0B  # Repartition NAND flash
BSL_CMD_READ_FLASH_TYPE = 0x0C  # Read flash type (SPI NAND / NOR)
BSL_CMD_READ_FLASH_INFO = 0x0D  # Read flash geometry parameters
BSL_CMD_CHECK_BAUD      = 0x7E  # Autobaud training sequence

# BSL Target Response Opcodes
BSL_REP_ACK             = 0x80  # Positive acknowledgment / success
BSL_REP_VER             = 0x81  # BootROM / loader version string
BSL_REP_INVALID_CMD     = 0x82  # Invalid command opcode
BSL_REP_UNKNOWN_CMD     = 0x83  # Unknown / unsupported command
BSL_REP_OPERATION_FAILED= 0x84  # Command execution failed
BSL_REP_CHECKSUM_ERROR  = 0x85  # CRC / Checksum verification error
BSL_REP_NOT_SUPPORT     = 0x90  # Feature not supported by current loader

# Command Name Mapping
BSL_CMD_NAMES = {
    BSL_CMD_CONNECT:         "BSL_CMD_CONNECT",
    BSL_CMD_START_DATA:      "BSL_CMD_START_DATA",
    BSL_CMD_MIDST_DATA:      "BSL_CMD_MIDST_DATA",
    BSL_CMD_END_DATA:        "BSL_CMD_END_DATA",
    BSL_CMD_EXEC_DATA:       "BSL_CMD_EXEC_DATA",
    BSL_CMD_NORMAL_RESET:    "BSL_CMD_NORMAL_RESET",
    BSL_CMD_READ_FLASH:      "BSL_CMD_READ_FLASH",
    BSL_CMD_READ_CHIP_TYPE:  "BSL_CMD_READ_CHIP_TYPE",
    BSL_CMD_READ_NVITEM:     "BSL_CMD_READ_NVITEM",
    BSL_CMD_CHANGE_BAUD:     "BSL_CMD_CHANGE_BAUD",
    BSL_CMD_ERASE_FLASH:     "BSL_CMD_ERASE_FLASH",
    BSL_CMD_REPARTITION:     "BSL_CMD_REPARTITION",
    BSL_CMD_READ_FLASH_TYPE: "BSL_CMD_READ_FLASH_TYPE",
    BSL_CMD_READ_FLASH_INFO: "BSL_CMD_READ_FLASH_INFO",
    BSL_CMD_CHECK_BAUD:      "BSL_CMD_CHECK_BAUD",
    BSL_REP_ACK:             "BSL_REP_ACK",
    BSL_REP_VER:             "BSL_REP_VER",
    BSL_REP_INVALID_CMD:     "BSL_REP_INVALID_CMD",
    BSL_REP_UNKNOWN_CMD:     "BSL_REP_UNKNOWN_CMD",
    BSL_REP_OPERATION_FAILED:"BSL_REP_OPERATION_FAILED",
    BSL_REP_CHECKSUM_ERROR:  "BSL_REP_CHECKSUM_ERROR",
    BSL_REP_NOT_SUPPORT:     "BSL_REP_NOT_SUPPORT",
}

def get_cmd_name(cmd: int) -> str:
    """Returns human-readable name for BSL opcode."""
    return BSL_CMD_NAMES.get(cmd, f"BSL_UNKNOWN_0x{cmd:02X}")

def spd_crc16(crc: int, data: bytes) -> int:
    """
    Calculates CCITT-16 CRC matching Spreadtrum/Unisoc BootROM implementation.
    Polynomial: x^16 + x^12 + x^5 + 1 (0x1021 / 0x11021).
    """
    crc &= 0xFFFF
    for b in data:
        crc ^= (b << 8)
        for _ in range(8):
            if crc & 0x8000:
                crc = ((crc << 1) ^ 0x1021) & 0xFFFF
            else:
                crc = (crc << 1) & 0xFFFF
    return crc

def spd_checksum(crc: int, data: bytes, final: int = 1) -> int:
    """
    Calculates 16-bit addition checksum matching Spreadtrum CHK_FIXZERO mode.
    Used by older loaders or when CRC16 is inactive.
    """
    length = len(data)
    i = 0
    while length > 1:
        crc += (data[i + 1] << 8) | data[i]
        i += 2
        length -= 2
    if length:
        crc += data[i]
    if final:
        crc = (crc >> 16) + (crc & 0xFFFF)
        crc += (crc >> 16)
        crc = (~crc) & 0xFFFF
        if len(data) < final:
            crc = ((crc >> 8) & 0xFF) | ((crc & 0xFF) << 8)
    return crc & 0xFFFF

def spd_transcode(data: bytes) -> bytes:
    """
    HDLC byte stuffing / escaping.
    Replaces 0x7E with 0x7D 0x5E, and 0x7D with 0x7D 0x5D.
    """
    out = bytearray()
    for b in data:
        if b in (HDLC_HEADER, HDLC_ESCAPE):
            out.append(HDLC_ESCAPE)
            out.append(b ^ 0x20)
        else:
            out.append(b)
    return bytes(out)

def spd_untranscode(data: bytes) -> bytes:
    """
    HDLC byte unstuffing / unescaping.
    Reverses byte stuffing for 0x7D escape characters.
    """
    out = bytearray()
    i = 0
    length = len(data)
    while i < length:
        b = data[i]
        if b == HDLC_ESCAPE:
            i += 1
            if i < length:
                out.append(data[i] ^ 0x20)
        else:
            out.append(b)
        i += 1
    return bytes(out)

def encode_packet(cmd_type: int, payload: bytes = b"", use_crc16: bool = True, transcode: bool = True) -> bytes:
    """
    Encodes a BSL command or response into an HDLC packet frame.
    Format: [0x7E] [Transcoded: TYPE(2) + LEN(2) + PAYLOAD + CRC(2)] [0x7E]
    """
    if cmd_type == BSL_CMD_CHECK_BAUD:
        return bytes([HDLC_HEADER] * max(1, len(payload) if payload else 1))

    raw = struct.pack(">HH", cmd_type, len(payload)) + payload
    if use_crc16:
        chk = spd_crc16(0, raw)
    else:
        chk = spd_checksum(0, raw)
    raw += struct.pack(">H", chk)

    if transcode:
        body = spd_transcode(raw)
    else:
        body = raw

    return bytes([HDLC_HEADER]) + body + bytes([HDLC_HEADER])

def decode_packet(frame: bytes, use_crc16: bool = True) -> Tuple[int, bytes, bool]:
    """
    Decodes an HDLC framed packet.
    Returns: (cmd_type, payload, crc_valid)
    """
    # Strip boundary flags
    if frame.startswith(bytes([HDLC_HEADER])):
        frame = frame[1:]
    if frame.endswith(bytes([HDLC_HEADER])):
        frame = frame[:-1]

    if not frame:
        return 0, b"", False

    raw = spd_untranscode(frame)
    if len(raw) < 6:  # 2 bytes type + 2 bytes len + 2 bytes crc
        return 0, b"", False

    cmd_type, length = struct.unpack(">HH", raw[:4])
    payload = raw[4 : 4 + length]
    chk_received = struct.unpack(">H", raw[4 + length : 6 + length])[0]

    content_for_chk = raw[: 4 + length]
    if use_crc16:
        chk_calculated = spd_crc16(0, content_for_chk)
    else:
        chk_calculated = spd_checksum(0, content_for_chk)

    crc_valid = (chk_received == chk_calculated)
    return cmd_type, payload, crc_valid

class BslPacketStream:
    """
    Stream buffer parser for receiving fragmented BSL HDLC packets over serial.
    """
    def __init__(self):
        self.buffer = bytearray()

    def feed(self, data: bytes) -> List[bytes]:
        """Feeds raw incoming serial bytes and returns list of extracted frames."""
        self.buffer.extend(data)
        frames = []

        while True:
            # Locate first HDLC_HEADER
            start = self.buffer.find(bytes([HDLC_HEADER]))
            if start == -1:
                self.buffer.clear()
                break

            # Discard any junk before start header
            if start > 0:
                del self.buffer[:start]

            # Find next header (end of frame)
            end = self.buffer.find(bytes([HDLC_HEADER]), 1)
            if end == -1:
                # Incomplete frame, wait for more data
                break

            # If adjacent headers (e.g. 0x7E 0x7E), discard first and keep searching
            if end == 1:
                del self.buffer[:1]
                continue

            # Extract frame including boundary flags
            frame = bytes(self.buffer[:end + 1])
            del self.buffer[:end + 1]
            frames.append(frame)

        return frames

class BslServer:
    """
    Unisoc T107 BootROM / BSL Target Server Simulation.
    Receives BSL commands over UART, handles FDL1 memory loading into IRAM,
    and returns appropriate acknowledgment packets.
    """
    def __init__(self, write_mem_fn: Callable[[int, bytes], None], version_string: str = "SPRD3\x00"):
        self.write_mem_fn = write_mem_fn
        self.version_string = version_string
        self.stream = BslPacketStream()
        self.connected = False
        self.target_address = 0
        self.expected_size = 0
        self.bytes_written = 0
        self.execution_address: Optional[int] = None
        self.use_crc16 = True

    def process_incoming(self, raw_bytes: bytes) -> List[bytes]:
        """Processes raw bytes received on UART RX, returns outgoing UART TX frames."""
        # Check for baud rate synchronization (train of 0x7E bytes)
        if not self.connected and (raw_bytes.count(b'\x7E') == len(raw_bytes)) and len(raw_bytes) > 0:
            # Send version response
            ver_payload = self.version_string.encode('ascii') if isinstance(self.version_string, str) else self.version_string
            return [encode_packet(BSL_REP_VER, ver_payload, use_crc16=self.use_crc16)]

        frames = self.stream.feed(raw_bytes)
        responses = []

        for f in frames:
            cmd, payload, crc_ok = decode_packet(f, use_crc16=self.use_crc16)
            if not crc_ok:
                responses.append(encode_packet(BSL_REP_CHECKSUM_ERROR, b"", use_crc16=self.use_crc16))
                continue

            resp = self._handle_command(cmd, payload)
            if resp:
                responses.append(resp)

        return responses

    def _handle_command(self, cmd: int, payload: bytes) -> Optional[bytes]:
        if cmd == BSL_CMD_CONNECT:
            self.connected = True
            return encode_packet(BSL_REP_ACK, b"", use_crc16=self.use_crc16)

        elif cmd == BSL_CMD_START_DATA:
            if len(payload) >= 8:
                self.target_address, self.expected_size = struct.unpack(">II", payload[:8])
                self.bytes_written = 0
                return encode_packet(BSL_REP_ACK, b"", use_crc16=self.use_crc16)
            else:
                return encode_packet(BSL_REP_OPERATION_FAILED, b"", use_crc16=self.use_crc16)

        elif cmd == BSL_CMD_MIDST_DATA:
            if self.write_mem_fn and len(payload) > 0:
                dest = self.target_address + self.bytes_written
                self.write_mem_fn(dest, payload)
                self.bytes_written += len(payload)
                return encode_packet(BSL_REP_ACK, b"", use_crc16=self.use_crc16)
            return encode_packet(BSL_REP_OPERATION_FAILED, b"", use_crc16=self.use_crc16)

        elif cmd == BSL_CMD_END_DATA:
            return encode_packet(BSL_REP_ACK, b"", use_crc16=self.use_crc16)

        elif cmd == BSL_CMD_EXEC_DATA:
            self.execution_address = self.target_address
            return encode_packet(BSL_REP_ACK, b"", use_crc16=self.use_crc16)

        elif cmd == BSL_CMD_READ_CHIP_TYPE:
            # Return Unisoc UMS9117 / T107 chip identification
            chip_info = struct.pack(">I", 0x91170001)
            return encode_packet(BSL_REP_ACK, chip_info, use_crc16=self.use_crc16)

        else:
            return encode_packet(BSL_REP_UNKNOWN_CMD, b"", use_crc16=self.use_crc16)

class BslClient:
    """
    Host Flasher Client.
    Generates BSL packets to transmit to target emulator or hardware.
    """
    def __init__(self, use_crc16: bool = True):
        self.use_crc16 = use_crc16

    def make_check_baud(self, count: int = 1) -> bytes:
        return bytes([HDLC_HEADER] * count)

    def make_connect(self) -> bytes:
        return encode_packet(BSL_CMD_CONNECT, b"", use_crc16=self.use_crc16)

    def make_start_data(self, load_addr: int, total_size: int) -> bytes:
        payload = struct.pack(">II", load_addr, total_size)
        return encode_packet(BSL_CMD_START_DATA, payload, use_crc16=self.use_crc16)

    def make_midst_data(self, chunk: bytes) -> bytes:
        return encode_packet(BSL_CMD_MIDST_DATA, chunk, use_crc16=self.use_crc16)

    def make_end_data(self) -> bytes:
        return encode_packet(BSL_CMD_END_DATA, b"", use_crc16=self.use_crc16)

    def make_exec_data(self) -> bytes:
        return encode_packet(BSL_CMD_EXEC_DATA, b"", use_crc16=self.use_crc16)

    def build_file_transfer_stream(self, data: bytes, target_addr: int, chunk_size: int = 1024) -> List[bytes]:
        """Builds full sequence of packets to transfer a file and trigger execution."""
        packets = []
        # 1. Connect
        packets.append(self.make_connect())
        # 2. Start Data
        packets.append(self.make_start_data(target_addr, len(data)))
        # 3. Stream Chunks
        for i in range(0, len(data), chunk_size):
            chunk = data[i : i + chunk_size]
            packets.append(self.make_midst_data(chunk))
        # 4. End Data
        packets.append(self.make_end_data())
        # 5. Execute
        packets.append(self.make_exec_data())
        return packets
