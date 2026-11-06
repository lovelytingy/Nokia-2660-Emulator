# Nokia 2660 Emulator (N2660Emu)

[![License: GPL v2+](https://img.shields.io/badge/License-GPL%20v2%2B-blue.svg)](LICENSE)
[![Target: Unisoc T107](https://img.shields.io/badge/SoC-Unisoc%20T107%20%2F%20UMS9117-orange.svg)](#target-hardware)
[![Arch: ARM Cortex-A7](https://img.shields.io/badge/CPU-ARM%20Cortex--A7%20(ARMv7--A)-red.svg)](#architecture)

An open-source hardware emulation and reverse-engineering environment for the **Nokia 2660 Flip** (Model **TA-1480**, India variant), powered by the **Unisoc T107 / UMS9117** platform.

The goal of this project is to understand and emulate the phone's hardware environment sufficiently to execute and debug firmware components, investigate the original **MOCOR** operating system, and eventually support the development and testing of **OpenFeatureOS**.

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
│   ├── BOOT_PROCESS.md          # BootROM, BSL, FDL1, FDL2, and MOCOR flow
│   └── PERIPHERALS.md           # SPRD UART, Timers, Keypad, SPI / Display specs
├── hw/
│   ├── arm/
│   │   ├── nokia_2660.c         # QEMU Machine model (TYPE_NOKIA_2660_MACHINE)
│   │   └── t107_soc.c           # QEMU T107 SoC model (CPU, SRAM, DRAM, MMIO trap)
│   └── char/
│       └── sprd_uart.c          # QEMU Unisoc SPRD UART model
├── include/
│   └── hw/
│       ├── arm/t107.h           # SoC and machine state headers
│       └── char/sprd_uart.h      # SPRD UART register defines and state
├── tests/
│   ├── baremetal/               # ARM Cortex-A7 baremetal verification firmware
│   │   ├── main.c               # CPU identification, SRAM test, DRAM test, MMIO trap
│   │   ├── startup.s            # Vector table, stack setup, VFP/NEON enable
│   │   ├── sprd_uart.c          # Baremetal UART driver
│   │   ├── linker.ld            # Linker script targeting T107 memory map
│   │   └── Makefile             # Built with arm-none-eabi-gcc
│   └── unit/
│       └── test_memory_map.py   # Automated memory map & payload tests
├── tools/
│   ├── n2660emu.py              # Unified CLI emulator launcher
│   ├── unicorn_runner.py        # Unicorn VM runner with exact T107 MMIO
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
[TEST] Triggering MMIO read at unmapped address 0x40001000...

[N2660Emu MMIO TRAP] Unknown peripheral READ at 0x40001000 (size 4) from PC 0x800002CC
[INFO] Unmapped MMIO read returned: 0x00000000

--------------------------------------------------------
[RESULT] *** ALL PHASE 1 MILESTONE CHECKS PASSED ***
[RESULT] CPU, Internal SRAM, DRAM, and Virtual UART OK!
--------------------------------------------------------
```

### 3. Run with Instruction Tracing
Trace CPU instructions as they execute in real time:
```bash
make trace
```

### 4. Run Automated Unit Tests
```bash
make test
```

### 5. Disassemble Binary with Unisoc T107 Annotations
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
- [ ] **Phase 2: T107 Memory Map & Core SoC** (Timers, Interrupt controller, GPIO, MMIO logging)
- [ ] **Phase 3: Nokia Peripherals** (SPI controller, GC9307 LCD, Matrix keypad, USB BSL)
- [ ] **Phase 4: Firmware Execution** (FDL1 DDR init, FDL2 flash ops, BOOT0/BOOT1, MOCOR OS)
- [ ] **Phase 5: OpenFeatureOS Development** (Custom kernel bringup and verification)
