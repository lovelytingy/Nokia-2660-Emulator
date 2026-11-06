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
4. **Firmware Analysis Tools (`tools/`)**:
   - `pac_extractor.py`: Parses and extracts partitions from official Spreadtrum `.pac` firmware files.
   - `disasm.py`: Capstone-powered disassembler with Unisoc register symbol annotations.
   - `unicorn_engine.py`: Fast Python/Unicorn runner for isolated binary execution, memory inspection, and unit testing.

---

## 4. Phase Roadmap

- [x] **Phase 1: CPU and Memory Emulation**
  - CPU architecture defined (Cortex-A7, ARMv7-A).
  - Minimal custom machine model created.
  - Memory regions (IRAM at `0x00000000`, DRAM at `0x80000000`) mapped.
  - Baremetal ARM test suite created and verified.
  - Virtual UART character output verified.
- [ ] **Phase 2: T107 SoC Peripherals & MMIO**
  - SPRD interrupt controller (VIC / GIC).
  - SPRD system timers and clock control.
  - GPIO and pinmux registers.
  - Unknown MMIO logging mechanism.
- [ ] **Phase 3: Nokia Peripherals**
  - SPI controller & GC9307 display driver.
  - Keypad controller & flip sensor.
  - USB device controller (BSL protocol).
  - SPI NAND flash emulation.
- [ ] **Phase 4: Firmware Execution**
  - FDL1 execution and memory init verification.
  - FDL2 protocol handling.
  - BOOT0/BOOT1 loading.
  - MOCOR OS startup tracing.
- [ ] **Phase 5: OpenFeatureOS Development**
  - Custom kernel booting.
  - Driver development and hardware validation.
