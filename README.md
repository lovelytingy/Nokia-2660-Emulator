# Nokia 2660 Emulator (N2660Emu)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Target: Unisoc T107](https://img.shields.io/badge/SoC-Unisoc%20T107%20%2F%20UMS9117-orange.svg)](#target-hardware)
[![Arch: ARM Cortex-A7](https://img.shields.io/badge/CPU-ARM%20Cortex--A7%20(ARMv7--A)-red.svg)](#architecture)
[![Engine: Rust Native](https://img.shields.io/badge/Engine-Rust%20(Native)-brightgreen.svg)](#high-performance-rust-emulation-engine)

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
├── Cargo.toml                   # Root Cargo workspace configuration
├── bin/
│   └── n2660emu                 # Symlink to main CLI orchestrator
├── crates/
│   ├── n2660-core/              # Core hardware emulation engine (T107 VM, MMIO, BSL, PAC, ATAGS)
│   ├── n2660-cli/               # High-performance native CLI binary (target/release/n2660emu-rs)
│   └── n2660-tools/             # Native tooling & firmware packaging crate
├── docs/
│   ├── ARCHITECTURE.md          # Complete emulator architecture & design
│   ├── RUST_ENGINE.md           # High-performance native Rust core architecture & benchmarks
│   ├── DESKTOP_GUI.md           # Native desktop host GUI & vector Nokia 2660 skin guide
│   ├── INTERACTIVE_UI.md        # Real-time terminal GUI & T9 keypad matrix guide
│   ├── ATAGS_AND_KERNEL_BOOT.md # ARM Boot Tags (ATAGS) & kernel handoff specification
│   ├── MEMORY_MAP.md            # Verified memory map and peripheral registers
│   ├── BOOT_FLOW.md             # BootROM reset vector, BOOT0 header & multi-stage boot flow
│   ├── BOOT_PROCESS.md          # Multi-stage boot sequence (BootROM -> FDL1 -> FDL2 -> OS)
│   ├── BSL_PROTOCOL.md          # Spreadtrum BSL framing, opcodes, CRC16 & FDL staging
│   ├── FIRMWARE_PACKAGING.md    # Unisoc .PAC container format, partition table & builder
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
│   ├── intc/
│   │   └── sprd_intc.c          # Unisoc SPRD Interrupt Controller (INTC 32 lines)
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
│       ├── intc/sprd_intc.h     # SPRD Interrupt Controller registers & IRQ map
│       ├── ssi/sprd_spi.h       # SPRD SPI host interface
│       └── timer/sprd_timer.h   # SPRD System Timer registers
├── roms/
│   ├── bootrom.s                # Clean-room ARMv7-A BootROM assembly (Reset vector at 0x0)
│   ├── bootrom.ld               # BootROM linker script targeting 0x00000000
│   ├── bootrom.bin              # Compiled 864-byte standalone BootROM binary
│   └── Makefile                 # Built with arm-none-eabi-gcc
├── tests/
│   ├── baremetal/               # ARM Cortex-A7 baremetal verification firmware
│   │   ├── main.c               # CPU, SRAM, DRAM, Timer, GPIO, INTC, Displays, KPD, Flash, Audio, BootROM & ATAGS
│   │   ├── interactive_demo.c   # Interactive Nokia OS demo (Dialing, Calling, Menu, Flip & Audio tones)
│   │   ├── startup.s            # Vector table, stack setup, boot register handoff, IRQ handler, VFP/NEON enable
│   │   ├── atags.c              # Baremetal ARM Boot Tags (ATAGS) driver & validator
│   │   ├── atags.h              # ATAGS structures & ARM handoff declarations
│   │   ├── intc.c               # Baremetal SPRD INTC driver, ISR dispatcher & register controls
│   │   ├── intc.h               # INTC registers, IRQ channel numbers & CPU interrupt APIs
│   │   ├── font5x7.c            # 5x7 ASCII bitmap font implementation & character renderer
│   │   ├── font5x7.h            # Font definitions & display string drawing routines
│   │   ├── bootrom.c            # Baremetal BootROM reset vector & BOOT0 loader driver
│   │   ├── bootrom.h            # SPRD BOOT0 header definition & loader API
│   │   ├── sprd_uart.c          # Baremetal UART driver
│   │   ├── gc9307.c             # Baremetal GC9307 LCD driver & test pattern
│   │   ├── st7735.c             # Baremetal ST7735 outer sub-display driver
│   │   ├── sfc.c                # Baremetal SPI NAND / SFC flash driver
│   │   ├── bsl.c                # Baremetal BSL HDLC & CRC16 protocol driver
│   │   ├── audio.c              # Baremetal Audio DAC, PCM streaming & tone buzzer driver
│   │   ├── kpd.c                # Baremetal Keypad and Hall flip sensor driver
│   │   ├── linker.ld            # Linker script targeting T107 memory map (with 4 KB IRQ stack)
│   │   └── Makefile             # Built with arm-none-eabi-gcc
├── tools/
│   ├── atags                    # ARM Boot Tags generator & inspector (Rust)
│   ├── bsl_engine               # Spreadtrum BSL protocol engine (Rust)
│   ├── disasm                   # Capstone disassembler with T107 annotations (Rust)
│   ├── mksprd_img               # Unisoc SPRD BOOT0 image packager (Rust)
│   └── pac_extractor            # Spreadtrum .PAC firmware container extractor (Rust)
├── Makefile                     # Root build & test Makefile
└── README.md                    # Project documentation
```

---

## Getting Started

### Prerequisites

- **Host OS**: macOS (Apple Silicon or Intel) or Linux.
- **Rust Toolchain**: `rustc` and `cargo` (1.75+ recommended).
- **ARM Toolchain**: `arm-none-eabi-gcc`, `arm-none-eabi-ld`, `arm-none-eabi-objcopy`.
- **QEMU**: `qemu-system-arm`.

On macOS with Homebrew:
```bash
brew install qemu arm-none-eabi-gcc rust
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
[TEST] Testing ARM Boot Tags (ATAGS) & Kernel Handoff...
[ATAGS] RAM Base: 0x80000000 | Size: 48 MB | Board ID: 0x9117
[PASS] ARM Boot Tags (ATAG_CORE, MEM, CMDLINE, SERIAL) & kernel handoff verified.
[TEST] Testing SPRD Interrupt Controller (INTC) at 0x50050000...
[INTC] Software IRQ 15 successfully executed (Counter: 1)
[INTC] Keypad IRQ 5 successfully executed (Counter: 1)
[INTC] Final Masked Status: 0x00000000
[PASS] SPRD Interrupt Controller, software IRQs & hardware IRQ vectors verified.
[TEST] Triggering MMIO read at unmapped address 0x40001000...

[N2660Emu MMIO TRAP] Unknown peripheral READ at 0x40001000 (size 4) from PC 0x80000D80
[INFO] Unmapped MMIO read returned: 0x00000000

--------------------------------------------------------
[RESULT] *** ALL PHASE 1, 2, 3 & 4 (DISPLAY, INPUT, STORAGE, AUDIO, BOOTROM, ATAGS & INTC) CHECKS PASSED ***
[RESULT] CPU, SRAM, DRAM, UART, Timer, GPIO, INTC, SPI0, GC9307, KPD, Flip, Sub-LCD, SFC, BSL, Audio, BootROM & ATAGS OK!
--------------------------------------------------------
```

### 3. Native Desktop Host GUI (`make gui` or `n2660emu --gui`)
Launch the native desktop emulator window with an authentic vector-rendered **Nokia 2660 Flip physical skin**, interactive 3D-embossed keypad, clamshell flip simulation, hardware telemetry deck, and live serial console:
```bash
make gui
# or via CLI launcher:
./bin/n2660emu --gui

# Custom casing color themes ("Deep Blue", "Classic Black", "Pop Red"):
./bin/n2660emu --gui --theme "Pop Red"

# Launch directly with clamshell closed:
./bin/n2660emu --gui --flip closed
```

**Features & Controls in Desktop GUI Mode**:
- **Interactive Vector Skin**: Hand-crafted Cairo vector model of the Nokia 2660 Flip with earpiece speaker, metallic chrome Nokia badge, glass screen reflections, dual metallic hinge, camera module, and LED flash.
- **Dual Display Pipelines**: 240×320 QVGA main display (GalaxyCore GC9307) and 120×160 QQVGA outer sub-display (Sitronix ST7735) with 64K lookup-table acceleration.
- **Tactile Keypad Interaction**: Click any physical button (SoftKeys, Call, End, D-Pad, OK, 0–9, *, #) with visual button depression, hardware MMIO matrix routing, and host click sound effects.
- **Clamshell Flip Toggle (`f` or Toolbar Button)**: Switches between Open (main screen & keypad) and Closed (camera module & outer sub-display) while updating the hardware Hall effect sensor on GPIO Pin 0.
- **Hardware Telemetry Deck**: Live CPU registers (PC, SP, LR, CPSR with mode decoding), instruction counter, dynamic MIPS execution speed, INTC raw/mask status, and hardware audio tone monitor.
- **Bidirectional Serial Console**: Live scrollable output of firmware UART0 prints and an input entry box to inject commands into the virtual UART0 RX FIFO.
- See [docs/DESKTOP_GUI.md](docs/DESKTOP_GUI.md) for full architecture and keyboard shortcuts.

### 4. Interactive Real-Time Terminal GUI & Keypad (`make interactive` or `n2660emu -i`)
Launch the real-time terminal user interface featuring 24-bit ANSI Truecolor dual-display rendering, visual T9 keypad matrix, hardware telemetry, and live serial console:
```bash
make interactive
# or via CLI launcher:
./bin/n2660emu -i
```

**Controls in Interactive Mode**:
- **Numeric Keys `0`–`9`, `*`, `#`**: Types digits into the dialing buffer with authentic key click audio beeps (`1200 Hz`).
- **Call Key `c`**: Dials the entered number, connecting an active call with dial tone (`440 Hz`).
- **End / Power Key `x`**: Terminates call with disconnect tone (`350 Hz`) and clears to Home screen.
- **Left SoftKey `[`**: Opens the Main Menu (Call Log, Contacts, Messages, Settings, Snake).
- **Right SoftKey `]`**: Back / Clears single dialed digit.
- **D-Pad Arrows `Up`/`Down`**: Navigates through menu options with live selection highlight.
- **Flip Key `f`**: Toggles Hall effect clamshell sensor (Open $\leftrightarrow$ Closed); activates the outer ST7735 screen clock when closed.
- **Display View `v`**: Toggles between Main Screen, Sub Screen, and Dual Screen view.
- **Execution `p`**: Pause / Resume execution.
- **Step `t` / `s`**: Single-step 1 instruction / 1,000 instructions (when paused).
- **Quit `q`**: Restores terminal and exits cleanly.

### 5. Execute Hardware BootROM & Multi-Stage Boot (BOOT0 vs BSL)
Execute the clean-room ARMv7-A BootROM starting at the reset vector (`0x00000000`):
```bash
# 1. Cold boot with BOOT0 image in SPI NAND Block 0:
./bin/n2660emu --boot0 boot0.bin

# 2. Force BSL UART download mode using Nokia '*' download key:
./bin/n2660emu --bootrom --boot-key star --fdl1 extracted_fw/fdl1.bin

# 3. Simulate blank SPI NAND fallback to BSL recovery mode:
./bin/n2660emu --bootrom
```

### 6. Package Raw Binaries into Unisoc SPRD BOOT0 Containers
Use `tools/mksprd_img` to wrap any baremetal payload with standard 32-byte SPRD headers:
```bash
./tools/mksprd_img create -i payload.bin -o boot0.bin --load-addr 0x00006200 --entry 0x00006200
./tools/mksprd_img verify boot0.bin
```

### 7. Simulate Keypad Input and Flip Opening / Closing
Simulate hardware key events and flip states dynamically via CLI arguments:
```bash
# Simulate pressing '5' and 'OK' with clamshell closed:
./bin/n2660emu tests/baremetal/test_firmware.bin --flip closed --key 5 --key ok
```

### 8. Mount Custom SPI NAND Flash Images
Mount external raw flash images, sparse partition dumps, or partitions extracted from vendor `.PAC` packages:
```bash
./bin/n2660emu tests/baremetal/test_firmware.bin --flash flash_dump.bin
```

### 9. Execute FDL1 & Simulate BootROM BSL Download Handshake
Load first-stage bootloader `fdl1.bin` into SRAM (`0x00006200`) and optionally simulate the Spreadtrum BSL HDLC protocol handshake over UART0:
```bash
# Direct FDL1 staging into SRAM and execution:
./bin/n2660emu --fdl1 extracted_fw/fdl1.bin

# Simulate BootROM BSL protocol handshake (0x7E framing, CRC-16, ACK sequence):
./bin/n2660emu --fdl1 extracted_fw/fdl1.bin --bsl

# Stage both FDL1 (SRAM 0x00006200) and FDL2 (DRAM 0x80100000):
./bin/n2660emu --fdl1 extracted_fw/fdl1.bin --fdl2 extracted_fw/fdl2.bin --bsl
```

### 10. Capture Audio and Buzzer Tones to WAV
Capture PCM audio playback and hardware buzzer tone generation to a standard 16-bit 44.1 kHz stereo WAV file:
```bash
./bin/n2660emu tests/baremetal/test_firmware.bin --audio-dump audio_capture.wav
```

### 11. Run with Instruction Tracing
Trace CPU instructions as they execute in real time:
```bash
make trace
```

### 12. Render Primary and Secondary Sub-Displays
Render test graphics and dump high-resolution framebuffer PPM images:
```bash
# Render and dump 240x320 main display:
make screen

# Render and dump 120x160 outer sub-screen:
make sub-screen
```

### 13. Run Automated Unit Tests
```bash
make test
```

### 14. Disassemble Binary with Unisoc T107 Annotations
```bash
make disasm
```

### 15. High-Performance Rust Emulation Core (`make rust`)
N2660Emu features a full-fidelity native Rust emulation core ([`crates/n2660-core`](crates/n2660-core), [`crates/n2660-cli`](crates/n2660-cli)) that replaces Python execution bottlenecks with zero-cost FFI, bit-exact MMIO intercept, and ARM exception vector dispatch:

```bash
# Build native release binary:
make rust

# Run firmware directly through native engine:
./target/release/n2660emu-rs tests/baremetal/test_firmware.bin
```

| Metric | Python Engine (`tools/unicorn_runner.py`) | Rust Engine (`target/release/n2660emu-rs`) | Speedup |
| :--- | :--- | :--- | :--- |
| **Baremetal Test Run (724K insns)** | 1.85 s | **0.081 s** | **~23× Faster** |
| **Interactive OS Shell Execution** | 2.1 MIPS | **16.7 MIPS** | **~8× Faster** |
| **MMIO Intercept Latency** | ~400–600 ns / op | **< 15 ns / op** | **~30× Faster** |
| **Startup Overhead** | ~280 ms | **< 4 ms** | **~70× Faster** |

For complete architecture, memory-safety considerations, and benchmark methodology, see [docs/RUST_ENGINE.md](docs/RUST_ENGINE.md).

---

## Reverse Engineering Tools

### Unpacking & Building Official Nokia `.PAC` Firmware
N2660Emu provides [`tools/pac_extractor`](tools/pac_extractor) to parse, unpack, inspect, and package Spreadtrum `.PAC` firmware archives:

```bash
# 1. List partition records, load addresses, and sizes:
./tools/pac_extractor list <firmware.pac>

# 2. Extract all partition images (FDL1, FDL2, BOOT0, BOOT1, KERNEL, SYSTEM):
./tools/pac_extractor extract <firmware.pac> -o extracted_fw/

# 3. Extract a single partition by ID:
./tools/pac_extractor extract <firmware.pac> --id FDL1 -o extracted_fw/

# 4. Dump flash geometry and partition layout XML:
./tools/pac_extractor dump-xml <firmware.pac>

# 5. Build a custom .PAC firmware archive:
./tools/pac_extractor pack -o custom.pac --name "Nokia 2660 Flip" \
    -f FDL1:build/fdl1.bin:0x00006200 \
    -f FDL2:build/fdl2.bin:0x80100000 \
    -f BOOT0:build/boot0.bin:0x00006200
```

### Direct .PAC Execution in N2660Emu
You can pass vendor `.PAC` containers directly to the emulator launcher:
```bash
./bin/n2660emu path/to/firmware.pac --bsl
```
The launcher will automatically unpack the container, stage `FDL1` into SRAM (`0x00006200`), stage `FDL2` into DRAM (`0x80100000`), perform the Spreadtrum BSL handshake over UART0, and boot the virtual machine.

### ARM Boot Tags (ATAGS) & Kernel Handoff
N2660Emu includes full support for the standard ARM Linux / MOCOR OS boot protocol with [`tools/atags`](tools/atags):

```bash
# 1. Synthesize binary ATAGS list:
./tools/atags build -o atags.bin --cmdline "console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M"

# 2. Inspect an existing ATAGS binary:
./tools/atags inspect atags.bin

# 3. Boot an OS kernel image with automatic ATAGS staging & register setup (r0=0, r1=0x9117, r2=0x80000100):
./bin/n2660emu zImage --entry 0x80008000 --atags --cmdline "console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M"
```
For detailed architecture, tag structure layouts, and register specifications, see [docs/ATAGS_AND_KERNEL_BOOT.md](docs/ATAGS_AND_KERNEL_BOOT.md).

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
- [x] **Phase 4: BootROM, BSL Engine, PAC Tools, ATAGS & Interrupt Controller (INTC)** (Reset vector at 0x00000000, download key detection, SPRD BOOT0 header parser, NAND Block 0 staging, BSL HDLC framing & CRC16, FDL1/FDL2 staging, PAC unpacker & builder, ARMv7-A ATAGS synthesis/parser & r0=0, r1=0x9117, r2=0x80000100 handoff, 32-line INTC at 0x50050000, CPU IRQ exception vectors & software IRQ dispatch)
- [x] **Phase 5: High-Performance Rust Emulation Core** (Zero-cost FFI Rust rewrite of T107 ARM Cortex-A7 VM, zero GC pauses, native MMIO callbacks, BSL HDLC/CRC16, ATAGS, SPRD BOOT0, PAC parser/packer, 16.7+ MIPS throughput, 23× faster than Python runner)
- [ ] **Phase 6: MOCOR OS Bringup & Vendor Firmware Execution** (FDL1 DDR init, FDL2 flash ops, BOOT1/LK, MOCOR OS)
- [ ] **Phase 7: OpenFeatureOS Development** (Custom kernel bringup and verification)

---

## ❤️ Support My Work

I am an independent developer building and maintaining this open-source emulator. If you find this project valuable, voluntary contributions help support continued development, hardware research, and ongoing improvements.

<a href="https://buymeacoffee.com/lovelytingy"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50"></a>

---

## License

This project is licensed under the [MIT License](LICENSE).
