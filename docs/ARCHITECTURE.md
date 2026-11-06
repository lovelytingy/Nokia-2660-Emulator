# Nokia 2660 Flip (TA-1480) Emulator Architecture (N2660Emu)

## 1. System Overview

The **N2660Emu** project is an open-source hardware emulation and reverse-engineering environment for the **Nokia 2660 Flip (Model TA-1480, India Variant)**, powered by the **Unisoc T107 (UMS9117 / UMS9117-L)** SoC.

### Purpose
- Understand and emulate the target phone's hardware environment with high fidelity.
- Execute and debug low-level firmware components (BootROM, FDL1, FDL2, BOOT0, BOOT1, MOCOR OS).
- Provide a clean foundation for the bringup and testing of **OpenFeatureOS**.
- Adhere strictly to verified hardware behavior, avoiding invented register definitions.

---

## 2. Target Hardware Specifications

| Component | Specification | Notes / Verification Status |
| :--- | :--- | :--- |
| **SoC** | Unisoc T107 / UMS9117 | Single-die Cat.1 4G Feature Phone SoC |
| **Application CPU** | ARM Cortex-A7 MPCore (Single Core) | ARMv7-A ISA, VFPv4-D16/D32, NEON, 1.0 GHz |
| **Caches** | 32 KB L1 Instruction, 32 KB L1 Data, 128 KB L2 | Verified via Unisoc SoC specs |
| **Co-Processor** | ARM Cortex-M4 | Always-On (AON) subsystem & PMU controller |
| **Internal SRAM** | ~64 KB (Base `0x00000000`) | Mapped to AP on-chip RAM; holds FDL1 (`0x6200`) |
| **DRAM** | 48 MB (Base `0x80000000`) | SIP LPDDR2 (`0x80000000` - `0x82FFFFFF`) |
| **Flash Storage** | 128 MB SPI NAND / SLC NAND | Partitioned: Boot0, Boot1, Kernel, System, Userdata |
| **Display** | 2.8" QVGA (240x320) LCD (GalaxyCore GC9307) | SPI/MIPI interface; secondary 1.77" outer display |
| **Keypad** | Matrix Keypad + Flip Detection Hall Sensor | Scanned via dedicated GPIO / KPD controller |
| **UART Interfaces**| Unisoc Serial Controller (SPRD UART) | Debug console, AT commands, FDL flashing transport |

---

## 3. High-Level Emulation Design

```
+-----------------------------------------------------------------------+
|                             N2660Emu Host                             |
|  +-----------------------------------------------------------------+  |
|  |                 CLI Orchestrator (n2660emu)                     |  |
|  +-----------------------------------------------------------------+  |
|         |                                              |              |
|         v                                              v              |
|  +-----------------------------+              +--------------------+  |
|  |     QEMU System Emulation   |              |   Unicorn Rapid    |  |
|  |   (t107_machine / nokia2660)|              |   Test Engine      |  |
|  +-----------------------------+              +--------------------+  |
|     |  ARM Cortex-A7 (v7-A)                      |  ARMv7-A Core     |
|     |  Internal SRAM (0x00000000, 64KB)          |  SRAM (64KB)      |
|     |  DRAM (0x80000000, 48MB)                   |  DRAM (48MB)      |
|     |  SPRD UART (0x50000000)                    |  SPRD UART (MMIO) |
|     |  MMIO Bus Router & Access Logger           |  MMIO Hook Logger |
|     +--------------------------------------------+-------------------+|
+-----------------------------------------------------------------------+
```

### Components:
1. **Machine Model (`hw/arm/nokia_2660.c`)**:
   - Implements the QEMU `MachineClass` (`TYPE_NOKIA_2660_MACHINE`).
   - Wires up the T107 SoC, allocates system RAM (48MB), maps internal SRAM (64KB), and attaches serial devices.
2. **SoC Model (`hw/arm/t107_soc.c`)**:
   - Manages the Cortex-A7 CPU, memory controller, bus interconnects, and peripheral mappings.
   - Contains fallback unmapped MMIO loggers that record any unimplemented register reads/writes.
3. **UART Peripheral (`hw/char/sprd_uart.c`)**:
   - Emulates the Unisoc/Spreadtrum UART IP block as documented in the Linux kernel (`sprd_serial.c`).
   - Implements FIFO status (`SPRD_STS1`), TX register (`SPRD_TXD`), RX register (`SPRD_RXD`), and clock divider registers.
4. **Firmware Analysis Tools & Rust Engine (`crates/` & `bin/`)**:
   - `bin/pac_extractor`: Parses, lists, extracts, and creates Spreadtrum `.pac` firmware files in native Rust.
   - `bin/disasm`: Capstone-powered Cortex-A7 disassembler with Unisoc register symbol annotations in native Rust.
   - `bin/n2660emu`: High-performance ARM Cortex-A7 virtual machine and native host GUI with live serial console, hardware telemetry deck, and interactive keypad.

---

## 4. Phase Roadmap

- [x] **Phase 1: CPU and Memory Emulation**
  - CPU architecture defined (Cortex-A7, ARMv7-A).
  - High-performance ARM Unicorn VM engine in Rust.
  - Memory regions (IRAM at `0x00000000`, DRAM at `0x80000000`) mapped.
  - Baremetal ARM test suite created and verified.
  - Virtual UART character output verified.
- [x] **Phase 2: T107 SoC Peripherals & MMIO**
  - SPRD interrupt controller (INTC 32 channels).
  - SPRD system timers and clock control.
  - GPIO and flip hall effect sensor.
  - Unknown MMIO logging mechanism with address trap throttling.
- [x] **Phase 3: Nokia Peripherals**
  - Dual SPI LCD controllers (GC9307 240x320 main and ST7735 120x160 outer).
  - Keypad controller (KPD matrix scanner + interrupt for 21 physical buttons).
  - BSL protocol engine (HDLC 0x7E framing + CRC16-CCITT).
  - SPI NAND flash emulation (Winbond W25N01GV 128 MB).
  - Audio DAC & PCM tone buzzer generator.
- [x] **Phase 4: Firmware Execution**
  - Automated .PAC vendor firmware container ingestion.
  - FDL1 SRAM staging and memory init verification.
  - FDL2 DRAM staging and SPI NAND flash mounting.
  - BOOT0/SPL loader with 32-byte SPRD/BIMG header validation.
  - ARM Boot Tags (ATAGS) generation & kernel handoff.
- [ ] **Phase 5: OpenFeatureOS Development**
  - Custom kernel booting.
  - Driver development and hardware validation.
