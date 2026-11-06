# Running Official Nokia 2660 Flip Firmware on N2660Emu

## 1. Overview

The primary mission of N2660Emu is to run official, production Nokia 2660 Flip (Model TA-1480 / Unisoc T107 / UMS9117) firmware downloaded from the internet.

This guide details the firmware packaging, ingestion pipeline, hardware emulation readiness, and execution workflows.

---

## 2. Firmware Format & Acquisition

### Supported Model Variants
Official Nokia 2660 Flip firmware is distributed across model variants based on regional SKUs and SIM configurations:
* **TA-1480**: Global / European Single SIM & Dual SIM (Primary target)
* **TA-1469 / TA-1474 / TA-1486**: APAC, LATAM, and North American carrier SKUs

### Distribution Container: `.PAC`
Vendor firmware is packaged in Unisoc/Spreadtrum `.PAC` (Packet Archive) format (magic `0xFFFAFFFA`). Common package names include:
```
Nokia_2660_Flip_TA-1480_v20.00.17.01.pac
TA-1480_10.00.17.01_pac.zip -> contains firmware.pac
```

Inside the `.PAC` archive, the vendor stores all hardware partition images and partition metadata:
* `FDL1`: First DownLoader (staged into SRAM at `0x00006200`; sets up clock trees and DRAM controller).
* `FDL2`: Second DownLoader (staged into High DRAM at `0x80100000`; handles flash writing).
* `BOOT0` / `SPL`: Primary bootloader with 32-byte `SPRD`/`BIMG` header (staged into SPI NAND Block 0).
* `BOOT` / `KERNEL`: MOCOR RTOS operating system kernel (staged into High DRAM at `0x80000000`).
* `SYSTEM` / `ROOTFS`: System assets, icons, fonts, and application binaries (stored in SPI NAND flash).
* `PRODNV` / `MISC`: RF calibration, NVRAM, and IMEI parameters.
* `PARTITION_XML`: Flash partition geometry manifest.

---

## 3. How to Inspect a Downloaded Firmware Archive

Before running, you can inspect the partition map and header metadata using [`bin/pac_extractor`](file:///Users/lovelytingy/Documents/Personal-Projects/Nokia-2660-Emulator/bin/pac_extractor):

```bash
# List all partitions and memory targets
./bin/pac_extractor list Nokia_2660_Flip_TA-1480.pac

# Dump the embedded partition geometry XML
./bin/pac_extractor dump-xml Nokia_2660_Flip_TA-1480.pac

# Unpack all partitions to a folder for reverse engineering
./bin/pac_extractor extract Nokia_2660_Flip_TA-1480.pac -o extracted_fw/
```

---

## 4. One-Command Booting with N2660Emu

N2660Emu natively ingests downloaded `.PAC` files directly without requiring manual extraction:

```bash
# Launch in authentic Nokia 2660 Desktop GUI mode
./bin/n2660emu Nokia_2660_Flip_TA-1480.pac --gui

# Or pass explicitly via --pac flag
./bin/n2660emu --pac Nokia_2660_Flip_TA-1480.pac --gui

# Run in headless mode with instruction timeout
./bin/n2660emu Nokia_2660_Flip_TA-1480.pac --timeout 30
```

### Automated Ingestion Pipeline
When a `.PAC` archive is loaded:
1. **Header Validation**: Verifies the Spreadtrum `0xFFFAFFFA` magic and decodes product metadata.
2. **SRAM Staging**: `FDL1` is placed into 64 KB Internal SRAM at `0x00006200`.
3. **DRAM Staging**: `FDL2` is placed into DRAM at `0x80100000`.
4. **Bootloader & SPL Staging**:
   - `BOOT0` is written to SPI NAND Block 0 (first page).
   - `BOOT0` is mirrored to Direct Flash XIP window (`0x60000000`).
   - If an SPRD 32-byte header is detected, the payload is staged to `hdr.load_addr` and the entry point is configured.
5. **Kernel Staging**: `BOOT` / `KERNEL` is staged into DRAM at `0x80000000`.
6. **Flash Storage Staging**: `SYSTEM` / `ROOTFS` is loaded into the virtual Winbond W25N01GV 128 MB flash storage.
7. **Entry Point Selection**: The CPU register bank (`SP`, `CPSR`, `PC`, `VFP`) is configured for the bootable partition.

---

## 5. Hardware Subsystems Prepared for Real Firmware

Real Unisoc T107 firmware expects specific hardware responses during initialization:

| Hardware Block | Emulation Implementation | Purpose in Stock Firmware |
| :--- | :--- | :--- |
| **CPU (ARM Cortex-A7)** | Unicorn ARMv7-A with VFPv4/NEON, SVC mode, CP15 CPACR/FPEXC. | Executes compiled ARMv7-A machine code at 16+ MIPS. |
| **Internal SRAM** | 64 KB mapped at `0x00000000`. SP starts at `0x0000FF00`. | BootROM and FDL1 execution prior to DRAM initialization. |
| **High DRAM** | 48 MB mapped at `0x80000000` (`0x80000000`..`0x82FFFFFF`). | MOCOR OS heaps, stacks, kernel BSS, and FDL2. |
| **Direct Flash (XIP)** | 128 MB mapped at `0x60000000` (`0x60000000`..`0x67FFFFFF`). | Direct memory-mapped read access to SPI NAND content. |
| **SPI NAND Controller (SFC)** | Winbond W25N01GV (128 MB, JEDEC ID `0x00EFAA21`, 2048-B pages). | MTD partition storage for System, NVRAM, and user data. |
| **Main LCD (SPI0)** | GalaxyCore GC9307 240×320 QVGA controller with D/C mode. | Primary internal screen rendering (2.8" QVGA). |
| **Sub LCD (SPI1)** | Sitronix ST7735 120×160 QQVGA controller with D/C mode. | Outer clamshell screen rendering (1.77" QQVGA). |
| **Keypad (KPD)** | Matrix keyboard scanner at `0x50060000` with IRQ 5 assertion. | Physical 21-key keypad (0..9, *, #, Softkeys, D-Pad). |
| **GPIO & Hall Sensor** | Pin 0 input tracks clamshell open (`1`) vs closed (`0`). | Detects flip open/close state transitions. |
| **Interrupt Controller (INTC)** | 32 hardware interrupt lines with mask, status, IRQ/FIQ routing. | Peripheral interrupt dispatching (KPD, Timer, UART). |
| **System Timer** | Down-counting hardware timer with reload at `0x50030000`. | OS task scheduling, delays, and system clock ticks. |
| **ADI / PMIC** | Battery voltage reporter (3900 mV / ~85%) at `0x500D0000`. | Prevents firmware emergency battery-cutoff shutdown. |
| **Clock Controller** | Returns clock enable status (`0xFFFFFFFF`) at `0x500E0000`. | Passes peripheral clock gating verification checks. |
| **Unmapped MMIO Trap** | Traps accesses across `0x50000000..0x50100000` and `0x40000000`. | Returns default 0/written values with single-address warning logging. |

---

## 6. Real-Time Telemetry & Debugging

When executing vendor firmware in Desktop GUI mode (`--gui`):
* **Live Serial Console**: The right-hand panel streams UART0 output in real time (`115200 8N1`), capturing vendor debug prints, bootloader banners, and OS logs.
* **Hardware Telemetry Deck**: Shows live CPU registers (`PC`, `SP`, `LR`, `CPSR`), execution speed (MIPS), active key input, and interrupt status.
* **Controls**:
  * `FLIP`: Toggles the Hall effect flip sensor between Open and Closed states.
  * `PAUSE` / `RESUME`: Freezes execution to inspect memory and registers.
  * `STEP`: Executes single CPU instructions.
  * `RESET`: Reinitializes CPU registers to the boot entry point.
