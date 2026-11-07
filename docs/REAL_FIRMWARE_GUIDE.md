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
* `FDL2`: Second DownLoader (staged into High DRAM at `0x80100000` / `0x80020000`; handles flash writing).
* `BOOT0` / `SPL`: Primary bootloader with DHTB/SPRD header (staged into SPI NAND Block 0 & SRAM at `0x00006200`, entry `0x00006400`).
* `BOOT1` / `LK`: Secondary bootloader (staged into High DRAM at `0x80100000`, entry `0x80100200`). Initializes FTL and verifies kernel.
* `KernelImg` / `BOOT`: MOCOR RTOS operating system kernel (staged into High DRAM at `0x80a05e00`, entry `0x80a06000`).
* `UserImg` / `SYSTEM`: Userland application framework (`MS_MMI_Main`) staged into User DRAM at `0x88000000` (entry `0x88479F6F`).
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

## 4. Multi-Stage Booting with N2660Emu

N2660Emu natively ingests downloaded `.PAC` files directly, auto-detects `v23_update_Nokia_2660.pac` in the workspace when no file is specified, and supports multi-stage boot selection via the `--stage` flag:

```bash
# 1. Native Desktop GUI (auto-detects PAC, boots in order, renders live on LCD)
cargo run -p n2660-cli -- --gui

# 2. Cold boot through all stages in order (BOOT0 -> BOOT1 -> Kernel -> MMI)
cargo run -p n2660-cli -- --stage boot0 --timeout 10

# 3. Boot directly into Stage 2 Bootloader (BOOT1 at 0x80100200)
cargo run -p n2660-cli -- --stage boot1 --timeout 10

# 4. Boot directly into MOCOR OS Kernel (0x80a06000)
cargo run -p n2660-cli -- --stage kernel --timeout 10

# 5. Boot directly into Userland Application Framework (MS_MMI_Main at 0x88479F6F)
cargo run -p n2660-cli -- --stage user --timeout 10
```

### Live LCD Display Progression Across Stages:
* **Stage 1 (`BOOT0` SPL @ `0x00006400`)**: The 240×320 LCD screen illuminates in authentic Nokia Deep Navy Blue (`0x016C`), showing the bold white **NOKIA** wordmark, hardware initialization status, and 25% boot progress.
* **Stage 2 (`BOOT1` LK @ `0x80100200`)**: The LCD displays Flash Translation Layer (`SCI_FTL`) mount status, DHTB signature verification, and 50% boot progress. The outer 120×160 cover screen shows `Stage 2: Loading OS...`.
* **Stage 3 (`KernelImg` @ `0x80a06000`)**: The LCD displays MOCOR 20A RTOS bringup (CP15 SCTLR, mode stacks, `__scatterload`, ThreadX scheduler active) and 75% boot progress.
* **Stage 4 (`UserImg` @ `0x88479F6F`)**: The LCD transitions to the full authentic Nokia 2660 Flip desktop UI with 4G LTE signal bars, battery meter, large digital clock, date, wallpaper, interactive T9 dialpad, and softkeys. Closing the flip dynamically switches the outer 120×160 sub-display to the closed cover clock and battery widget!

### Automated Ingestion Pipeline
When a `.PAC` archive is loaded:
1. **Header Validation**: Verifies the Spreadtrum `0xFFFAFFFA` magic and decodes product metadata.
2. **SRAM Staging**: `FDL1` is placed into Internal SRAM at `0x00006200`.
3. **BOOT0 Staging**: `BOOT0` is placed into SRAM at `0x00006200` and SPI NAND Block 0 (entry `0x00006400`).
4. **BOOT1 Staging**: `BOOT1` is placed into DRAM at `0x80100000` (entry `0x80100200`).
5. **Kernel Staging**: `KernelImg` is placed into DRAM at `0x80a05e00` (reset vector `0x80a06000` -> `0x80a061f0`).
6. **Userland Staging**: `UserImg` is placed into DRAM at `0x88000000` (entry `0x88479F6F`).
7. **ThreadX Heap**: Pre-maps `0x7E000000`..`0x80000000` for dynamic RTOS thread stacks.
8. **Entry Point Selection**: Configured based on the `--stage` parameter or explicit `--entry` override.

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

---

## 7. Executing Bare-Metal RAM Payloads (`keypad_test.bin`)

You can execute bare-metal DRAM payloads (from the companion [`Nokia-2660-RE`](https://github.com/lovelytingy/Nokia-2660-RE) repository) directly on N2660Emu. These payloads run at `0x80100000` (entry `0x80100200`) and communicate with the host via the Unisoc USB OTG controller and `libc_server` protocol.

### Running Keypad Test Payload
```bash
# Headless run with live serial output (timeout 3 seconds)
cargo run -p n2660-cli -- ../Nokia-2660-RE/tools/ram_payload/bin/keypad_test.bin --timeout 3

# Inject a specific key (e.g., Key 5) to test matrix scanning and EIC interrupts
cargo run -p n2660-cli -- ../Nokia-2660-RE/tools/ram_payload/bin/keypad_test.bin --key 5 --timeout 3

# Capture high-resolution 240x320 main display framebuffer to PPM image
cargo run -p n2660-cli -- ../Nokia-2660-RE/tools/ram_payload/bin/keypad_test.bin --key 5 --screen-dump screen.ppm --timeout 2

# Launch in Native Desktop GUI mode with interactive keypad buttons and live display
cargo run -p n2660-cli -- ../Nokia-2660-RE/tools/ram_payload/bin/keypad_test.bin --gui
```

### Features Verified in `keypad_test.bin`:
* **Heap Allocation**: 64 MB DRAM heap dynamically allocated (`malloc heap: 66017280 bytes`).
* **USB Libc Protocol**: Loads authentic `pinmap.bin` (2416 B) and `keymap.bin` (128 B) over USB DMA.
* **GC9307 Identification**: SPI1 controller returns GC9307 ID byte stream (`0x009307`).
* **Hardware Matrix Scanner**: 5x5 keypad matrix generates real-time keypress events (`DOWN [PRESSED]` / `UP [RELEASED]`).
* **SC2720 PMIC EIC Channels**: Power/End key (EIC Pin 1) and Key 8 (EIC Pin 9) dedicated interrupt pathways.

