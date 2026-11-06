# Spreadtrum / Unisoc Boot Serial Loader (BSL) Protocol

## 1. Overview

The **Spreadtrum / Unisoc Boot Serial Loader (BSL)** protocol is the low-level serial communication protocol implemented in the BootROM (BROM) and secondary downloaders (FDL1, FDL2) across Spreadtrum and Unisoc mobile chipsets, including the **Unisoc T107 / UMS9117** powering the **Nokia 2660 Flip (TA-1480)**.

The protocol serves two primary purposes:
1. **BootROM Handshake**: BootROM listens on UART0 or USB-Serial for flashing tools (such as `spd_dump`, `ResearchDownload`, or `N2660Emu`), receiving `FDL1` into Internal SRAM (`0x00006200`).
2. **Firmware Downloader (FDL) Control**: Once executed, `FDL1` initializes system clocks and DDR memory, then uses BSL to receive `FDL2` into DRAM (`0x80100000`) for full partition flashing and diagnostics.

---

## 2. Packet Framing & Encoding

BSL uses an **HDLC-like byte-oriented framing** mechanism over UART/serial channels.

### Frame Structure

```text
+----------+-----------------------------------------------------------+----------+
| 0x7E     | Transcoded Body                                           | 0x7E     |
| (Header) | [ Type (2B) | Length (2B) | Payload (NB) | Checksum (2B) ]| (Tail)   |
+----------+-----------------------------------------------------------+----------+
```

| Field | Size | Endianness | Description |
|---|---|---|---|
| `Header` | 1 byte | - | Frame boundary flag: `0x7E` |
| `Type` | 2 bytes | Big-Endian (`>H`) | Command or Response Opcode |
| `Length` | 2 bytes | Big-Endian (`>H`) | Length of data payload $N$ in bytes |
| `Payload` | $N$ bytes | Raw | Command parameters or binary chunk |
| `Checksum` | 2 bytes | Big-Endian (`>H`) | CCITT-16 CRC or 16-bit sum over `[Type + Length + Payload]` |
| `Tail` | 1 byte | - | Frame boundary flag: `0x7E` |

---

### Byte Stuffing / Transcoding Rules

To prevent payload bytes from colliding with the frame delimiter flag (`0x7E`), BSL applies character escaping prior to transmission:

* **Escape Prefix**: `0x7D`
* **Transcode Rule**: Any occurrence of `0x7E` or `0x7D` in the message body is replaced with `0x7D` followed by `byte ^ 0x20`:
  * `0x7E` $\rightarrow$ `0x7D 0x5E`
  * `0x7D` $\rightarrow$ `0x7D 0x5D`
* **Untranscode Rule**: When receiving, any sequence `0x7D [b]` is decoded as `[b ^ 0x20]`.

---

## 3. Checksum & CRC-16 Algorithms

### CRC-16 (Standard in Modern BootROM / BSL)
Modern Unisoc chipsets use standard CCITT-16 CRC:
* **Polynomial**: $x^{16} + x^{12} + x^5 + 1$ (`0x1021`)
* **Initial Value**: `0x0000`

```rust
pub fn crc16_ccitt(mut crc: u16, data: &[u8]) -> u16 {
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}
```

---

## 4. BSL Command & Response Opcodes

### Host Commands

| Opcode | Name | Payload | Description |
|---|---|---|---|
| `0x00` | `BSL_CMD_CONNECT` | Empty | Initial handshake to establish connection |
| `0x01` | `BSL_CMD_START_DATA` | `[Addr: 4B][Size: 4B]` (BE) | Begins binary transfer; sets load address and total length |
| `0x02` | `BSL_CMD_MIDST_DATA` | `[Chunk: N Bytes]` | Streams file chunk into memory buffer |
| `0x03` | `BSL_CMD_END_DATA` | Empty | Concludes data transfer; verifies payload integrity |
| `0x04` | `BSL_CMD_EXEC_DATA` | Empty | Instructs CPU to branch to previously set load address |
| `0x05` | `BSL_CMD_NORMAL_RESET`| Empty | Triggers hardware reset into normal operating mode |
| `0x06` | `BSL_CMD_READ_FLASH` | `[Offset: 4B][Size: 4B]` | Reads raw flash memory bytes |
| `0x07` | `BSL_CMD_READ_CHIP_TYPE`| Empty | Queries SoC / ASIC hardware identification |
| `0x0A` | `BSL_CMD_ERASE_FLASH` | `[Offset: 4B][Size: 4B]` | Erases target flash blocks |
| `0x7E` | `BSL_CMD_CHECK_BAUD` | Train of `0x7E` | Autobaud synchronization byte sequence |

### Target Responses

| Opcode | Name | Payload | Description |
|---|---|---|---|
| `0x80` | `BSL_REP_ACK` | Optional data | Positive acknowledgment / command succeeded |
| `0x81` | `BSL_REP_VER` | ASCII string | Version string of BootROM / FDL loader (e.g. `"SPRD3\0"`) |
| `0x82` | `BSL_REP_INVALID_CMD` | Empty | Invalid or unrecognized command opcode |
| `0x83` | `BSL_REP_UNKNOWN_CMD` | Empty | Command not supported in current boot stage |
| `0x84` | `BSL_REP_OPERATION_FAILED` | Empty | Flash operation or memory write failed |
| `0x85` | `BSL_REP_CHECKSUM_ERROR` | Empty | Packet CRC or checksum mismatch |
| `0x90` | `BSL_REP_NOT_SUPPORT` | Empty | Feature unsupported |

---

## 5. FDL Multi-Stage Loading Sequence

The complete boot sequence from cold reset to MOCOR OS execution proceeds through four distinct stages:

```
[Host PC]                                   [Nokia 2660 / UMS9117]
   |                                                    |
   | --- BSL_CMD_CHECK_BAUD (0x7E train) -------------> |  (BootROM)
   | <--- BSL_REP_VER ("SPRD3\0") --------------------- |
   |                                                    |
   | --- BSL_CMD_CONNECT (0x00) ----------------------> |
   | <--- BSL_REP_ACK (0x80) -------------------------- |
   |                                                    |
   | --- BSL_CMD_START_DATA (0x00006200, len) --------> |
   | <--- BSL_REP_ACK (0x80) -------------------------- |
   |                                                    |
   | --- BSL_CMD_MIDST_DATA (chunk 1) ----------------> |  (Written to SRAM)
   | <--- BSL_REP_ACK (0x80) -------------------------- |
   |     ... (repeat for all chunks) ...                |
   |                                                    |
   | --- BSL_CMD_END_DATA (0x03) ---------------------> |
   | <--- BSL_REP_ACK (0x80) -------------------------- |
   |                                                    |
   | --- BSL_CMD_EXEC_DATA (0x04) --------------------> |
   | <--- BSL_REP_ACK (0x80) -------------------------- |
   |                                                    v
   |                                             [Branch to 0x00006200]
   |                                             (FDL1 runs & initializes DDR)
   |                                                    |
   | --- BSL_CMD_CONNECT (0x00) ----------------------> |
   | <--- BSL_REP_ACK (0x80) -------------------------- |
   |                                                    |
   | --- BSL_CMD_START_DATA (0x80100000, len) --------> |
   | <--- BSL_REP_ACK (0x80) -------------------------- |
   |     ... (streams FDL2 into DRAM) ...               |
   | --- BSL_CMD_EXEC_DATA (0x04) --------------------> |
   |                                                    v
   |                                             [Branch to 0x80100000]
   |                                             (FDL2 runs & loads MOCOR)
```

---

## 6. Using BSL in N2660Emu

N2660Emu provides both direct binary staging and full BSL protocol simulation.

### 1. Direct FDL1 Staging into SRAM
Execute `fdl1.bin` loaded at `0x00006200`:
```bash
./bin/n2660emu --fdl1 extracted_fw/fdl1.bin
```

### 2. Full BSL Protocol Simulation
Simulate the BootROM UART0 handshake and packet streaming:
```bash
./bin/n2660emu --fdl1 extracted_fw/fdl1.bin --bsl
```

### 3. Dual-Stage FDL1 and FDL2 Loading
Stage `fdl1.bin` into SRAM at `0x00006200` and `fdl2.bin` into DRAM at `0x80100000`:
```bash
./bin/n2660emu --fdl1 extracted_fw/fdl1.bin --fdl2 extracted_fw/fdl2.bin --bsl
```

### 4. Custom Load Addresses
Override default memory entry points if using custom SPL or test kernels:
```bash
./bin/n2660emu --fdl1 payload.bin --fdl1-addr 0x00001000 --bsl
```

### 5. Rust API (`n2660_core::bsl`) & CLI (`bin/bsl_engine`)
```rust
use n2660_core::bsl::{build_bsl_packet, parse_bsl_packet, crc16_ccitt, hdlc_stuff, hdlc_unstuff, BslPacket};

// Encode a BSL command packet with HDLC framing and CRC-16
let pkt = build_bsl_packet(0x00, &payload); // 0x00 = BSL_CMD_CONNECT

// CLI Tool:
// ./bin/bsl_engine --help
```
