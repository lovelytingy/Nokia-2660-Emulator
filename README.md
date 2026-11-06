# Nokia 2660 Flip Emulator (N2660Emu)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Target: Unisoc T107](https://img.shields.io/badge/SoC-Unisoc%20T107%20%2F%20UMS9117-orange.svg)](#target-hardware-specifications)
[![Arch: ARM Cortex-A7](https://img.shields.io/badge/CPU-ARM%20Cortex--A7%20(ARMv7--A)-red.svg)](#target-hardware-specifications)
[![Engine: Rust Native](https://img.shields.io/badge/Engine-Rust%20(Native)-brightgreen.svg)](docs/RUST_ENGINE.md)

<a href="https://buymeacoffee.com/lovelytingy"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="32"></a>

An open-source hardware emulation and reverse-engineering environment for the **Nokia 2660 Flip** (Model **TA-1480**, India variant), powered by the **Unisoc T107 / UMS9117** platform.

The goal of this project is to understand and emulate the phone's hardware environment sufficiently to execute and debug firmware components, investigate the original **MOCOR** RTOS, and support the development of **OpenFeatureOS**.

> [!IMPORTANT]
> **Legal Notice & Non-Affiliation**: This project is an independent open-source research effort and is **not affiliated with, sponsored by, or endorsed by Nokia Corporation, HMD Global Oy, or Unisoc Technologies Co., Ltd.** All trademarks belong to their respective owners. **No proprietary firmware binaries are distributed in this repository.** See [DISCLAIMER.md](DISCLAIMER.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Target Hardware Specifications

| Component | Specification |
| :--- | :--- |
| **Device & Model** | Nokia 2660 Flip (Model TA-1480, India Variant) |
| **SoC / Platform** | Unisoc T107 / UMS9117-L (Single-core ARM Cortex-A7 @ 1.0 GHz) |
| **Memory** | 64 KB On-Chip SRAM (`0x00000000`), 48 MB SIP LPDDR2 DRAM (`0x80000000`) |
| **Storage** | 128 MB SPI NAND Flash (Winbond W25N01GV) with direct XIP (`0x60000000`) |
| **Displays** | Primary 2.8" QVGA (240×320) GC9307 TFT LCD + Sub 1.77" QQVGA (120×160) ST7735 |
| **Input / Audio** | SPRD 3×4 Keypad Matrix, GPIO Pin 0 Hall Clamshell Flip Sensor, Audio DAC & Tone Buzzer |
| **Target OS** | MOCOR RTOS |

---

## Features

- **High-Performance Rust Core**: Pure Rust execution engine (`crates/n2660-core`) delivering **16+ MIPS** via zero-cost FFI MMIO hooks, bit-exact peripheral simulation, and ARM IRQ exception vector dispatch.
- **Authentic Desktop Host GUI**: Native Cocoa/minifb window (`make gui`) featuring a hand-crafted physical Nokia 2660 Flip skin, interactive 3D-embossed keypad with mouse hit-testing, dual LCD pipelines, and clamshell flip simulation (`f`).
- **ANSI Truecolor Terminal TUI**: Real-time terminal interface (`make interactive`) with half-block downsampled dual displays, live T9 keypad matrix, and hardware telemetry.
- **Reverse-Engineering Toolchain**: Native Rust tools for Spreadtrum BSL protocol framing, BOOT0 image packaging, `.PAC` firmware container unpacking/packing, ATAGS staging, and annotated ARM disassembly.
- **Zero Proprietary Blobs**: Standalone clean-room ARMv7-A BootROM and baremetal verification firmware.

---

## Quick Start

### Prerequisites
- **Host OS**: macOS (Apple Silicon or Intel) or Linux.
- **Toolchains**: Rust (`rustc` & `cargo` 1.75+) and ARM GNU toolchain (`arm-none-eabi-gcc`).

```bash
# macOS (Homebrew)
brew install qemu arm-none-eabi-gcc rust
```

### Essential Commands

```bash
# Build firmware, clean-room BootROM, and native Rust engine
make

# Run baremetal hardware verification payload
make run

# Launch native Desktop GUI with Nokia 2660 physical skin
make gui

# Launch interactive real-time terminal TUI
make interactive

# Run full integration test suite (100% Rust workspace tests)
make test
```

---

## CLI & Tooling Reference

The main emulator CLI [`bin/n2660emu`](bin/n2660emu) provides full control over hardware simulation:

```bash
# Launch Desktop GUI with custom casing theme ("Pop Red", "Deep Blue", "Classic Black")
./bin/n2660emu --gui --theme "Pop Red"

# Launch directly with clamshell closed (outer sub-display active)
./bin/n2660emu --gui --flip closed

# Execute vendor .PAC container with automatic BSL handshake & FDL staging
./bin/n2660emu firmware.pac --bsl

# Boot ARM Linux / MOCOR OS kernel with ATAGS handoff
./bin/n2660emu zImage --entry 0x80008000 --atags --cmdline "console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M"

# Cold boot with BOOT0 image in SPI NAND Block 0
./bin/n2660emu --boot0 boot0.bin
```

### Standalone Utilities ([`tools/`](tools/))

| Utility | Description | Example Usage |
| :--- | :--- | :--- |
| [`tools/pac_extractor`](tools/pac_extractor) | Unpack, inspect, and build Spreadtrum `.PAC` packages | `./tools/pac_extractor extract fw.pac -o extracted_fw/` |
| [`tools/mksprd_img`](tools/mksprd_img) | Create & verify SPRD BOOT0 headers | `./tools/mksprd_img create -i fw.bin -o boot0.bin` |
| [`tools/atags`](tools/atags) | Build & inspect binary ARM Boot Tags lists | `./tools/atags build -o atags.bin --cmdline "..."` |
| [`tools/disasm`](tools/disasm) | Disassemble ARMv7-A/Thumb with T107 symbols | `./tools/disasm test_firmware.bin --base 0x80000000` |
| [`tools/bsl_engine`](tools/bsl_engine) | Spreadtrum BSL HDLC framing & CRC16 calculator | `./tools/bsl_engine --crc16 "123456789"` |

---

## Documentation Index

Comprehensive technical documentation is maintained in the [`docs/`](docs/) directory:

| Document | Description |
| :--- | :--- |
| [**Architecture Guide**](docs/ARCHITECTURE.md) | High-level emulator design, SoC architecture, and data pipelines |
| [**Rust Engine & Benchmarks**](docs/RUST_ENGINE.md) | Native Rust engine details, zero-cost FFI, and performance benchmarks |
| [**Desktop Host GUI**](docs/DESKTOP_GUI.md) | Nokia 2660 Flip physical skin, hit-testing, dual LCDs, and controls |
| [**Interactive Terminal TUI**](docs/INTERACTIVE_UI.md) | 24-bit ANSI Truecolor dual-display rendering and T9 keypad matrix |
| [**Memory Map & Registers**](docs/MEMORY_MAP.md) | Verified memory addresses, MMIO peripherals, and register definitions |
| [**Boot Flow Specification**](docs/BOOT_FLOW.md) | Reset vector, BootROM flow, BOOT0 headers, and staging mechanics |
| [**BSL Protocol Specification**](docs/BSL_PROTOCOL.md) | Spreadtrum BSL HDLC framing (`0x7E`), command opcodes, and CRC-16 |
| [**Firmware Packaging (.PAC)**](docs/FIRMWARE_PACKAGING.md) | Unisoc `.PAC` container layout, partition records, and pack/unpack flow |
| [**ATAGS & Kernel Handoff**](docs/ATAGS_AND_KERNEL_BOOT.md) | ARM Boot Tags synthesis, DRAM placement, and register state (`r0=0, r1=0x9117, r2=0x80000100`) |
| [**Hardware Peripherals**](docs/PERIPHERALS.md) | Specifications for SPRD UART, Timers, GPIO, INTC, SFC, Audio, GC9307, ST7735 |

---

## Project Structure

```
Nokia-2660-Emulator/
├── bin/n2660emu                 # Unified CLI launcher
├── Cargo.toml                   # Root Cargo workspace manifest
├── crates/
│   ├── n2660-core/              # Hardware simulation library (T107 VM, MMIO, BSL, PAC, ATAGS)
│   ├── n2660-cli/               # Native CLI binary, Desktop GUI (minifb) & Terminal TUI (crossterm)
│   └── n2660-tools/             # Native tooling & firmware packaging crate
├── docs/                        # Deep-dive architecture & hardware specifications
├── hw/                          # QEMU C peripheral models (UART, Timer, GPIO, INTC, SFC, Displays)
├── roms/                        # Standalone clean-room ARMv7-A BootROM assembly & binary
├── tests/baremetal/             # Baremetal verification firmware & interactive Nokia demo
├── tools/                       # Standalone tool binaries (pac_extractor, mksprd_img, atags, disasm)
└── Makefile                     # Root build, test, and run orchestrator
```

---

## Roadmap

- [x] **Phase 1: CPU & Memory Emulation** (ARM Cortex-A7, SRAM, DRAM, SPRD UART)
- [x] **Phase 2: T107 Core SoC** (Timers, GPIO, MMIO access logging, down-counter verification)
- [x] **Phase 3: Nokia Peripherals** (GC9307 QVGA, ST7735 outer LCD, Keypad matrix, Hall flip sensor, SFC 128 MB SPI NAND, Audio DAC & Buzzer)
- [x] **Phase 4: BootROM, BSL, PAC Tools, ATAGS & INTC** (Reset vector, BSL framing & CRC16, PAC tools, ARM ATAGS handoff, 32-line INTC & IRQ vectors)
- [x] **Phase 5: High-Performance Rust Emulation Core & Native Host GUI** (100% Rust workspace, zero-cost FFI, 16+ MIPS, native Desktop GUI with phone skin)
- [ ] **Phase 6: MOCOR OS Bringup & Vendor Firmware Execution** (FDL1 DDR init, FDL2 flash ops, BOOT1/LK, MOCOR OS)
- [ ] **Phase 7: OpenFeatureOS Development** (Custom open-source kernel bringup and verification)

---

## ❤️ Support My Work

I am an independent developer building this open-source emulator. If you find this project valuable, voluntary contributions help support hardware research, testing equipment, and ongoing development:

<a href="https://buymeacoffee.com/lovelytingy"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50"></a>

---

## License

This project is licensed under the [MIT License](LICENSE).
