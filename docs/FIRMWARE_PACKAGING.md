# Unisoc / Spreadtrum .PAC Firmware Container Specification & Tooling

## 1. Overview

Official stock firmware for the Nokia 2660 Flip (TA-1480, Unisoc T107 / UMS9117) and other Spreadtrum-based feature phones is distributed as `.PAC` archive packages.

A `.PAC` file is a structured container holding:
- Machine and firmware metadata (product name, firmware build version).
- Hardware configuration parameters (flash type, target bus strategy).
- Partition table entries with load and execution addresses.
- Compressed or raw partition binaries (`FDL1`, `FDL2`, `BOOT0`, `BOOT1`, `BOOT`, `SYSTEM`, `USERDATA`).
- Partition configuration descriptor XML (`partition.xml`).

N2660Emu provides clean-room tools to inspect, unpack, and build `.PAC` packages via [`tools/pac_extractor.py`](file:///Users/lovelytingy/Documents/Personal-Projects/Nokia-2660-Emulator/tools/pac_extractor.py) and boot them directly in the emulator via [`bin/n2660emu`](file:///Users/lovelytingy/Documents/Personal-Projects/Nokia-2660-Emulator/bin/n2660emu).

---

## 2. Binary Layout

A Spreadtrum `.PAC` file consists of three contiguous sections:

```
+-------------------------------------------------------------+
|                      1. PAC Header                          |
|                       (2124 bytes)                          |
+-------------------------------------------------------------+
|                 2. File Table (File Records)                |
|               (nFileCount * 2124 bytes each)                |
+-------------------------------------------------------------+
|                     3. Partition Payloads                   |
|               (Contiguous raw binary payloads)              |
+-------------------------------------------------------------+
```

### A. Main Header (`PAC_HEADER`)

| Offset (Hex) | Offset (Dec) | Field Name | Type | Description |
| :--- | :--- | :--- | :--- | :--- |
| `0x0000` | `0` | `szVersion` | `wchar_t[48]` (96 B) | PAC format version (e.g., `BP_R1.0.0`). |
| `0x0060` | `96` | `dwSize` | `uint32` (4 B) | Total byte size of the `.PAC` archive. |
| `0x0064` | `100` | `szPrdName` | `wchar_t[256]` (512 B) | Target product string (e.g., `Nokia 2660 Flip`). |
| `0x0264` | `612` | `szPrdVersion` | `wchar_t[256]` (512 B) | Release version string (e.g., `TA-1480_v20.00`). |
| `0x0464` | `1124` | `nFileCount` | `uint32` (4 B) | Number of partition records in file table. |
| `0x0468` | `1128` | `dwFileOffset` | `uint32` (4 B) | Byte offset where the file record array starts (usually `2124`). |
| `0x046C` | `1132` | `dwMode` | `uint32` (4 B) | Flash mode. |
| `0x0470` | `1136` | `dwFlashType` | `uint32` (4 B) | Flash technology (0=NAND, 1=NOR, 2=eMMC, 3=SPI NAND). |
| `0x0474` | `1140` | `dwNandStrategy` | `uint32` (4 B) | Bad block management strategy. |
| `0x0478` | `1144` | `dwIsCheckBcc` | `uint32` (4 B) | BCC checksum flag. |
| `0x047C` | `1148` | `dwIsDownloadUser` | `uint32` (4 B) | Download user data flag. |
| `0x0480` | `1152` | `dwReserved` | `uint8[800]` (800 B) | Reserved padding. |
| `0x07A0` | `1952` | `dwMagic` | `uint32` (4 B) | PAC magic constant (`0xFFFAFFFA`). |

---

### B. File Entry Record (`PAC_FILE_ENTRY`)

Each partition record is 2124 bytes in size:

| Offset | Field Name | Type | Description |
| :--- | :--- | :--- | :--- |
| `0x000` | `dwSize` | `uint32` (4 B) | Total size of this record entry (`2124`). |
| `0x004` | `szFileID` | `wchar_t[256]` (512 B) | Partition ID (e.g. `FDL1`, `FDL2`, `BOOT0`, `BOOT`, `SYSTEM`). |
| `0x204` | `szFileName` | `wchar_t[256]` (512 B) | Target filename on disk (e.g. `fdl1.bin`). |
| `0x404` | `szFileVersion` | `wchar_t[256]` (512 B) | Partition version string. |
| `0x604` | `nFileSize` | `uint64` (8 B) | Byte length of the partition binary payload. |
| `0x60C` | `nFileFlag` | `uint32` (4 B) | 1 = Active / Download, 0 = Omitted. |
| `0x610` | `nCheckFlag` | `uint32` (4 B) | Checksum verification flag. |
| `0x614` | `dwDataOffset` | `uint32` (4 B) | Byte offset within `.PAC` where payload begins. |
| `0x618` | `dwCanOmitFlag` | `uint32` (4 B) | Whether flash tool may skip partition if unmodified. |
| `0x61C` | `dwAddrNum` | `uint32` (4 B) | Number of target load addresses (typically `1`). |
| `0x620` | `dwLoadAddr[5]` | `uint32[5]` (20 B) | Target memory/flash load addresses (`dwLoadAddr[0]` = primary target). |

---

## 3. Standard Nokia 2660 Partition Table

When unpacked, an official firmware image contains the following core components:

| Partition ID | Target File | Default Load Addr | Description |
| :--- | :--- | :--- | :--- |
| `FDL1` | `fdl1.bin` | `0x00006200` | First downloader; executes in SRAM; initializes DDR controller. |
| `FDL2` | `fdl2.bin` | `0x80100000` | Second downloader; executes in DRAM; handles SPI NAND flashing. |
| `BOOT0` / `SPL` | `boot0.bin` | `0x00006200` | Initial cold-boot secondary program loader in flash Block 0. |
| `BOOT1` / `LK` | `boot1.bin` | `0x80020000` | Little Kernel bootloader; initializes displays & splash logo. |
| `BOOT` / `KERNEL`| `boot.bin` | `0x80040000` | MOCOR RTOS operating system kernel payload. |
| `PARTITION_XML` | `partition.xml` | `N/A` | XML descriptor of SPI NAND flash geometry and blocks. |
| `PRODNV` | `prodnv.bin` | `N/A` | Factory production non-volatile calibration parameters. |
| `SYSTEM` | `system.img` | `N/A` | Read-only system partition and core application binaries. |
| `USERDATA` | `userdata.img` | `N/A` | User storage partition (contacts, messages, settings). |

---

## 4. CLI Tool Usage

### 1. Listing Partitions
To inspect a PAC file's header and partition records without writing files:
```bash
python3 tools/pac_extractor.py list Nokia_2660_Flip.pac
```

### 2. Extracting All Partitions
To unpack all active partition images into a folder:
```bash
python3 tools/pac_extractor.py extract Nokia_2660_Flip.pac -o extracted_fw/
```

### 3. Extracting a Single Partition
To extract only a specific binary (e.g. `FDL1`):
```bash
python3 tools/pac_extractor.py extract Nokia_2660_Flip.pac --id FDL1 -o extracted_fw/
```

### 4. Dumping Partition Geometry XML
To extract and print the flash partition layout XML:
```bash
python3 tools/pac_extractor.py dump-xml Nokia_2660_Flip.pac
```

### 5. Creating a Custom PAC Package
To package custom bootloaders and images into a compliant `.PAC` archive:
```bash
python3 tools/pac_extractor.py pack -o custom_fw.pac \
    --name "Nokia 2660 Flip" \
    --version "CUSTOM_v1.0" \
    -f FDL1:build/fdl1.bin:0x00006200 \
    -f FDL2:build/fdl2.bin:0x80100000 \
    -f BOOT0:build/boot0.bin:0x00006200
```

---

## 5. Direct Execution in N2660Emu

N2660Emu can accept `.PAC` containers directly as firmware arguments:
```bash
./bin/n2660emu path/to/firmware.pac --bsl
```
When passed a `.PAC` archive, the launcher automatically:
1. Parses the container header and partition table.
2. Caches and extracts `FDL1`, `FDL2`, and `BOOT0` to `build/pac_cache/<firmware_name>/`.
3. Resolves memory load addresses (`0x00006200` for FDL1, `0x80100000` for FDL2).
4. Initiates the BSL protocol handshake over UART0 and boots the emulator.
