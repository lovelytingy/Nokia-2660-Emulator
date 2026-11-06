# Unisoc T107 (UMS9117) Boot Process & Architecture

## 1. Boot Stages Overview

The Nokia 2660 Flip boot hierarchy follows the standard Unisoc / Spreadtrum multi-stage boot protocol:

```
+-------------------------------------------------------------+
|                      1. BootROM (BROM)                      |
|                Internal ROM at 0xFFFF0000                   |
+-------------------------------------------------------------+
                               |
                               | Check download key (* key)
             +-----------------+-----------------+
             |                                   |
             v (Boot key held)                   v (Normal Boot)
+---------------------------+       +-------------------------+
|     BSL Download Mode     |       |       BOOT0 / SPL       |
|    USB / UART protocol    |       |  Internal SRAM (0x6200) |
+---------------------------+       +-------------------------+
             |                                   |
             v Host sends FDL1                   v
+---------------------------+       +-------------------------+
|           FDL1            |       |       BOOT1 / LK        |
|  Initializes DDR & Clocks |       |     DRAM (0x80020000)   |
+---------------------------+       +-------------------------+
             |                                   |
             v Host sends FDL2                   v
+---------------------------+       +-------------------------+
|           FDL2            |       |     MOCOR OS Kernel     |
|   Flash partition ops     |       |     DRAM (0x80040000)   |
+---------------------------+       +-------------------------+
```

---

## 2. Detailed Boot Stages

### Stage 1: BootROM (BROM)
- Hardcoded on-chip ROM executed upon CPU release from reset.
- Checks hardware strapping and key combination:
  - On the Nokia 2660 Flip (TA-1480), holding the `*` (asterisk) key while connecting USB enters **BSL (Bootloader) mode**.
- If download mode is detected, BROM exposes a USB Serial / UART protocol to handshake with flashing tools (`spd_dump`).
- If normal boot, reads the Secondary Program Loader (`BOOT0`) from SPI NAND flash into Internal SRAM (`0x00006200`).

### Stage 2: FDL1 / BOOT0 (First Downloader / SPL)
- Executed from Internal SRAM (`0x00006200`) before DDR initialization.
- Responsibility:
  - Disables watchdog timer.
  - Configures system PLLs and bus clock dividers.
  - Initializes PMU power rails.
  - Initializes the LPDDR2 controller for the 48 MB system DRAM at `0x80000000`.
- In download mode, receives `FDL2` from the host and loads it into DRAM (`0x80100000`).

### Stage 3: FDL2 / BOOT1 (Second Downloader / Little Kernel)
- Executed from high DRAM (`0x80100000` / `0x80020000`).
- Responsibility:
  - Full hardware abstraction layer initialization.
  - Initializes SPI NAND flash controller.
  - Initializes display controller (GC9307) and shows the boot splash screen.
  - In normal boot: loads the MOCOR OS kernel image (`0x80040000`) and passes boot tags.

### Stage 4: MOCOR OS (Proprietary RTOS)
- Main feature phone operating system based on Unisoc's RTOS platform.
- Manages multi-tasking, GSM/LTE cellular modem stack, audio DSP, UI framework, SIM card, and power management.

---

## 3. Emulation Strategy in N2660Emu

1. **Phase 1 (Milestone Complete)**:
   - Emulate ARM Cortex-A7 core.
   - Map Internal SRAM (`0x00000000`, 64 KB) and DRAM (`0x80000000`, 48 MB).
   - Route SPRD UART console (`0x50000000`).
   - Execute baremetal payloads verifying CPU instructions, memory access, and MMIO traps.

2. **Phase 2 & 3 (Peripherals)**:
   - Add timer interrupts, GPIO keypad matrix, and SPI display output.

3. **Phase 4 (Firmware Bringup)**:
   - Execute real extracted `fdl1.bin` and trace DDR initialization routines.
   - Execute `fdl2.bin` and simulate flash partition reads.
   - Boot MOCOR OS kernel image.
