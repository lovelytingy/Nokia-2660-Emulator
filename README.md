# Nokia 2660 Emulator (N2660Emu)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Target: Unisoc T107](https://img.shields.io/badge/SoC-Unisoc%20T107%20%2F%20UMS9117-orange.svg)](#target-hardware)
[![Arch: ARM Cortex-A7](https://img.shields.io/badge/CPU-ARM%20Cortex--A7%20(ARMv7--A)-red.svg)](#architecture)

<a href="https://buymeacoffee.com/lovelytingy"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="32"></a>

An open-source hardware emulation and reverse-engineering environment for the **Nokia 2660 Flip** (Model **TA-1480**, India variant), powered by the **Unisoc T107 / UMS9117** platform.

The goal of this project is to understand and emulate the phone's hardware environment sufficiently to execute and debug firmware components, investigate the original **MOCOR** operating system, and eventually support the development and testing of **OpenFeatureOS**.

> [!IMPORTANT]
> **Legal Notice & Non-Affiliation**: This project is an independent open-source research effort and is **not affiliated with, sponsored by, or endorsed by Nokia Corporation, HMD Global Oy, or Unisoc Technologies Co., Ltd.** All trademarks belong to their respective owners and are used strictly under nominative fair use to describe hardware compatibility. **No proprietary firmware binaries are distributed in this repository.** For full details, see [DISCLAIMER.md](DISCLAIMER.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Target Hardware Specifications

| Component | Specification |
| :--- | :--- |
| **Device** | Nokia 2660 Flip |
| **Model** | TA-1480 (India Variant) |
| **SoC** | Unisoc T107 / UMS9117-L |
| **CPU Core** | ARM Cortex-A7 MPCore (Single Core, 1.0 GHz, ARMv7-A) |
| **Co-Processor** | ARM Cortex-M4 (Always-On Subsystem) |
| **Internal SRAM** | 64 KB (Base `0x00000000`, AP on-chip fast memory) |
| **System DRAM** | 48 MB (Base `0x80000000`, SIP LPDDR2) |
| **Flash Capacity** | 128 MB SPI NAND / SLC NAND |
| **Display** | 2.8" QVGA (240x320) TFT LCD (GalaxyCore GC9307) + 1.77" Outer screen |
| **OS** | MOCOR RTOS |

---

## Project Structure

```
Nokia-2660-Emulator/
├── bin/
│   └── n2660emu                 # Symlink to main CLI orchestrator
├── docs/
│   ├── ARCHITECTURE.md          # Complete emulator architecture & design
│   ├── MEMORY_MAP.md            # Verified memory map and peripheral registers
│   ├── BOOT_FLOW.md             # BootROM reset vector, BOOT0 header & multi-stage boot flow
│   ├── BOOT_PROCESS.md          # Multi-stage boot sequence (BootROM -> FDL1 -> FDL2 -> OS)
│   ├── BSL_PROTOCOL.md          # Spreadtrum BSL framing, opcodes, CRC16 & FDL staging
│   └── PERIPHERALS.md           # SPRD UART, Timers, Keypad, SPI / Display specs
├── hw/
│   ├── arm/
│   │   ├── nokia_2660.c         # QEMU Machine model (TYPE_NOKIA_2660_MACHINE)
│   │   └── t107_soc.c           # QEMU T107 SoC model (CPU, SRAM, DRAM, MMIO trap)
│   ├── audio/
│   │   └── sprd_audio.c         # Unisoc SPRD Audio Subsystem, DAC & Tone Buzzer model
│   ├── block/
│   │   └── sprd_sfc.c           # Unisoc SPRD Serial Flash Controller & 128 MB SPI NAND
│   ├── char/
│   │   └── sprd_uart.c          # QEMU Unisoc SPRD UART model
│   ├── display/
│   │   ├── gc9307.c             # GalaxyCore GC9307 240x320 TFT LCD controller (Main screen)
│   │   └── st7735.c             # Sitronix ST7735 120x160 TFT LCD controller (Outer sub-screen)
│   ├── gpio/
│   │   └── sprd_gpio.c          # Unisoc SPRD GPIO controller
│   ├── input/
│   │   └── sprd_kpd.c           # Unisoc SPRD Keypad matrix controller
│   ├── ssi/
│   │   └── sprd_spi.c           # Unisoc SPRD SPI host controller (SPI0 & SPI1)
│   └── timer/
│       └── sprd_timer.c         # Unisoc SPRD System Timer
├── include/
│   └── hw/
│       ├── arm/t107.h           # SoC and machine state headers
│       ├── audio/sprd_audio.h   # SPRD Audio & Buzzer Tone Generator register defines
│       ├── block/sprd_sfc.h     # SFC registers and SPI NAND geometry
│       ├── char/sprd_uart.h     # SPRD UART register defines and state
│       ├── display/
│       │   ├── gc9307.h         # GC9307 command set and LCD state
│       │   └── st7735.h         # ST7735 outer sub-screen state
│       ├── gpio/sprd_gpio.h     # SPRD GPIO registers and bank definitions
│       ├── input/sprd_kpd.h     # Keypad matrix and Nokia keycode definitions
│       ├── ssi/sprd_spi.h       # SPRD SPI host interface
│       └── timer/sprd_timer.h   # SPRD System Timer registers
├── roms/
│   ├── bootrom.s                # Clean-room ARMv7-A BootROM assembly (Reset vector at 0x0)
│   ├── bootrom.ld               # BootROM linker script targeting 0x00000000
│   ├── bootrom.bin              # Compiled 864-byte standalone BootROM binary
│   └── Makefile                 # Built with arm-none-eabi-gcc
├── tests/
│   ├── baremetal/               # ARM Cortex-A7 baremetal verification firmware
│   │   ├── main.c               # CPU, SRAM, DRAM, Timer, GPIO, Displays, KPD, Flash, BSL, Audio, BootROM
│   │   ├── startup.s            # Vector table, stack setup, VFP/NEON enable
│   │   ├── bootrom.c            # Baremetal BootROM reset vector & BOOT0 loader driver
│   │   ├── bootrom.h            # SPRD BOOT0 header definition & loader API
│   │   ├── sprd_uart.c          # Baremetal UART driver
│   │   ├── gc9307.c             # Baremetal GC9307 LCD driver & test pattern
│   │   ├── st7735.c             # Baremetal ST7735 outer sub-display driver
│   │   ├── sfc.c                # Baremetal SPI NAND / SFC flash driver
│   │   ├── bsl.c                # Baremetal BSL HDLC & CRC16 protocol driver
│   │   ├── audio.c              # Baremetal Audio DAC, PCM streaming & tone buzzer driver
│   │   ├── kpd.c                # Baremetal Keypad and Hall flip sensor driver
│   │   ├── linker.ld            # Linker script targeting T107 memory map
│   │   └── Makefile             # Built with arm-none-eabi-gcc
│   └── unit/
│       ├── test_bootrom.py      # BootROM reset vector, BOOT0 packaging & boot flow tests
│       ├── test_memory_map.py   # Automated memory map & peripheral tests
│       └── test_bsl_protocol.py # BSL protocol framing, CRC16, and FDL loader tests
├── tools/
│   ├── n2660emu.py              # Unified CLI emulator launcher
│   ├── unicorn_runner.py        # Unicorn VM runner with exact T107 MMIO
│   ├── mksprd_img.py            # Unisoc SPRD BOOT0 image packaging & NAND flash utility
│   ├── bsl_engine.py            # Spreadtrum BSL protocol engine (Client & Server)
│   ├── pac_extractor.py         # Spreadtrum .PAC vendor firmware extractor
│   └── disasm.py                # Capstone disassembler with T107 annotations
├── Makefile                     # Root build & test Makefile
└── README.md                    # Project documentation
```

---

## Getting Started

### Prerequisites

- **Host OS**: macOS (Apple Silicon or Intel) or Linux.
- **ARM Toolchain**: `arm-none-eabi-gcc`, `arm-none-eabi-ld`, `arm-none-eabi-objcopy`.
- **Python 3**: with `capstone` and `unicorn`.
- **QEMU**: `qemu-system-arm`.

On macOS with Homebrew:
```bash
brew install qemu arm-none-eabi-gcc
pip3 install capstone unicorn
```

---

## Building and Running the Test Firmware

### 1. Build the Baremetal Payload
Compile the ARMv7-A baremetal test firmware:
```bash
make firmware
```
This produces `tests/baremetal/test_firmware.elf` and flat binary `test_firmware.bin`.

### 2. Run in N2660Emu (Unicorn Engine)
Execute the payload with full Unisoc T107 memory mapping and SPRD UART virtual output:
```bash
make run
```
Output:
```
========================================================
   Nokia 2660 Flip Emulator (N2660Emu) - Test Firmware  
   Target: Model TA-1480 | Unisoc T107 / UMS9117        
========================================================

[CPU] MIDR: 0x412FC0F1 | CPSR: 0x600000D3
[CPU] Implementer: ARM Ltd (0x41)
[CPU] Architecture: ARMv7-A
[CPU] Core Model Part: 0x00000C0F
[TEST] Testing Internal SRAM (IRAM) at 0x00001000 (1024 bytes)...
[PASS] Internal SRAM (IRAM) verified successfully.
[TEST] Testing High DRAM (LPDDR2) at 0x80020000 (1024 bytes)...
[PASS] High DRAM (LPDDR2) verified successfully.
[TEST] Testing SPRD System Timer at 0x50030000...
[TIMER] Read 1: 0x0000FFF8 -> Read 2: 0x0000FB24
[PASS] SPRD System Timer counter down-counting verified.
[TEST] Testing SPRD GPIO Controller at 0x50040000...
[GPIO] Written: 0x00000020 -> Readback: 0x00000021
[PASS] SPRD GPIO pin direction and output state verified.
[TEST] Initializing GalaxyCore GC9307 LCD at SPI0 0x50080000...
[LCD] Drawing 240x320 QVGA test pattern & color bars...
[PASS] GC9307 display rendering test completed.
[TEST] Testing SPRD Keypad Matrix & Hall Flip Sensor at 0x50060000...
[FLIP] Hall effect sensor state: OPEN (Pin 0 = 1)
[KPD] Key status STATUS0: 0x00000000 | STATUS1: 0x00000000
[PASS] Keypad matrix and flip sensor interface verified.
[TEST] Initializing Outer Sub-Display (1.77" 120x160 QQVGA) at SPI1 0x50090000...
[SUB-LCD] Drawing 120x160 outer screen clock & status frame...
[PASS] Outer sub-display rendering test completed.
[TEST] Testing Unisoc SFC & SPI NAND Flash at 0x500A0000...
[NAND] JEDEC Identification: 0x00EFAA21 (Winbond W25N01GV)
[XIP] Direct Flash Window read at 0x60040000: 0xAA000000
[PASS] SPI NAND JEDEC ID, page programming, cache readback & XIP verified.
[TEST] Testing Spreadtrum BSL Protocol Engine & FDL Staging...
[BSL] CRC16 test vector: 0x0000EBB5
[PASS] Spreadtrum BSL protocol framing, CRC16 & FDL1 loading verified.
[TEST] Testing SPRD Audio Subsystem & Buzzer Tone Generator at 0x500C0000...
[AUDIO] Status STS: 0x00001004 (TX FIFO Samples: 16)
[N2660Emu Audio] Beep Tone: 1000 Hz, 50 ms, volume 85%
[BUZZER] Triggered 1000 Hz key click tone (50 ms, vol 85%)
[PASS] SPRD Audio DAC, PCM FIFO streaming & tone buzzer verified.
[TEST] Testing BootROM Reset Vector & Multi-Stage Boot Flow...
[BOOTROM] SPRD magic: 0x53505244 | Checksum: 0x10B050E0 | Staged at: 0x00006200
[PASS] BootROM reset vector, SPRD header validation & BOOT0 staging verified.
[TEST] Triggering MMIO read at unmapped address 0x40001000...

[N2660Emu MMIO TRAP] Unknown peripheral READ at 0x40001000 (size 4) from PC 0x8000097C
[INFO] Unmapped MMIO read returned: 0x00000000

--------------------------------------------------------
[RESULT] *** ALL PHASE 1, 2, 3 & 4 (DISPLAY, INPUT, STORAGE, AUDIO & BOOTROM) CHECKS PASSED ***
[RESULT] CPU, SRAM, DRAM, UART, Timer, GPIO, SPI0, GC9307, KPD, Flip, Sub-LCD, SFC, BSL, Audio & BootROM OK!
--------------------------------------------------------
```

### 3. Execute Hardware BootROM & Multi-Stage Boot (BOOT0 vs BSL)
Execute the clean-room ARMv7-A BootROM starting at the reset vector (`0x00000000`):
```bash
# 1. Cold boot with BOOT0 image in SPI NAND Block 0:
./bin/n2660emu --boot0 boot0.bin

# 2. Force BSL UART download mode using Nokia '*' download key:
./bin/n2660emu --bootrom --boot-key star --fdl1 extracted_fw/fdl1.bin

# 3. Simulate blank SPI NAND fallback to BSL recovery mode:
./bin/n2660emu --bootrom
```

### 4. Package Raw Binaries into Unisoc SPRD BOOT0 Containers
Use `tools/mksprd_img.py` to wrap any baremetal payload with standard 32-byte SPRD headers:
```bash
python3 tools/mksprd_img.py create -i payload.bin -o boot0.bin --load-addr 0x00006200 --entry 0x00006200
python3 tools/mksprd_img.py verify boot0.bin
```

### 5. Simulate Keypad Input and Flip Opening / Closing
Simulate hardware key events and flip states dynamically via CLI arguments:
```bash
# Simulate pressing '5' and 'OK' with clamshell closed:
./bin/n2660emu tests/baremetal/test_firmware.bin --flip closed --key 5 --key ok
```

### 6. Mount Custom SPI NAND Flash Images
Mount external raw flash images, sparse partition dumps, or partitions extracted from vendor `.PAC` packages:
```bash
./bin/n2660emu tests/baremetal/test_firmware.bin --flash flash_dump.bin
```

### 7. Execute FDL1 & Simulate BootROM BSL Download Handshake
Load first-stage bootloader `fdl1.bin` into SRAM (`0x00006200`) and optionally simulate the Spreadtrum BSL HDLC protocol handshake over UART0:
```bash
# Direct FDL1 staging into SRAM and execution:
./bin/n2660emu --fdl1 extracted_fw/fdl1.bin

# Simulate BootROM BSL protocol handshake (0x7E framing, CRC-16, ACK sequence):
./bin/n2660emu --fdl1 extracted_fw/fdl1.bin --bsl

# Stage both FDL1 (SRAM 0x00006200) and FDL2 (DRAM 0x80100000):
./bin/n2660emu --fdl1 extracted_fw/fdl1.bin --fdl2 extracted_fw/fdl2.bin --bsl
```

### 8. Capture Audio and Buzzer Tones to WAV
Capture PCM audio playback and hardware buzzer tone generation to a standard 16-bit 44.1 kHz stereo WAV file:
```bash
./bin/n2660emu tests/baremetal/test_firmware.bin --audio-dump audio_capture.wav
```

### 9. Run with Instruction Tracing
Trace CPU instructions as they execute in real time:
```bash
make trace
```

### 10. Render Primary and Secondary Sub-Displays
Render test graphics and dump high-resolution framebuffer PPM images:
```bash
# Render and dump 240x320 main display:
make screen

# Render and dump 120x160 outer sub-screen:
make sub-screen
```

### 11. Run Automated Unit Tests
```bash
make test
```

### 12. Disassemble Binary with Unisoc T107 Annotations
```bash
make disasm
```

---

## Reverse Engineering Tools

### Unpacking Official Nokia `.PAC` Firmware
Extract raw partition images (`fdl1.bin`, `fdl2.bin`, `boot0.bin`, `boot1.bin`, `kernel.bin`) from manufacturer `.pac` containers:
```bash
./tools/pac_extractor.py <firmware.pac> -o extracted_fw/
```

To list partition layouts and load addresses without extracting:
```bash
./tools/pac_extractor.py <firmware.pac> -l
```

---

## Engineering Principles

1. **Understand Before Emulating**: Rely on verified hardware observations and firmware disassembly rather than guesswork.
2. **Never Invent Registers**: Unmapped peripheral accesses trigger explicit `[N2660Emu MMIO TRAP]` logs rather than silently returning dummy values.
3. **Modular Architecture**: Clean separation between CPU, memory map, machine definitions, and peripheral IP blocks.
4. **Preserve Original Dumps**: Original firmware dumps are treated as immutable references.

---

## Roadmap

- [x] **Phase 1: CPU and Memory Emulation** (ARM Cortex-A7, SRAM, DRAM, SPRD UART, Baremetal test passed)
- [x] **Phase 2: T107 Memory Map & Core SoC** (Timers, GPIO, MMIO access logging, down-counter verification)
- [x] **Phase 3: Nokia Peripherals (Displays, Input, Storage & Audio)** (SPRD SPI0 & SPI1, GC9307 main LCD, ST7735 sub-screen, SPRD KPD matrix, Hall flip sensor, 128 MB SPI NAND & SFC, SPRD Audio DAC & Tone Buzzer)
- [x] **Phase 4: BootROM, BSL Engine & Multi-Stage Boot Flow** (Reset vector at 0x00000000, download key detection, SPRD BOOT0 header parser, NAND Block 0 staging, BSL HDLC framing & CRC16, FDL1/FDL2 staging)
- [ ] **Phase 4 (Cont.): MOCOR OS Bringup & Vendor Firmware Execution** (FDL1 DDR init, FDL2 flash ops, BOOT1/LK, MOCOR OS)
- [ ] **Phase 5: OpenFeatureOS Development** (Custom kernel bringup and verification)

---

## ❤️ Support My Work

I am an independent developer building and maintaining this open-source emulator. If you find this project valuable, voluntary contributions help support continued development, hardware research, and ongoing improvements.

<a href="https://buymeacoffee.com/lovelytingy"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50"></a>

---

## License

This project is licensed under the [MIT License](LICENSE).
