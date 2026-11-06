# Unisoc T107 (UMS9117) Memory Map Specification

## 1. Memory Regions Overview

The Unisoc T107 (UMS9117) SoC incorporates an ARM Cortex-A7 application core alongside an ARM Cortex-M4 Always-On (AON) subsystem.

The following table summarizes the verified physical memory addresses and device regions:

| Address Range | Size | Region Name | Description | Verification Status |
| :--- | :--- | :--- | :--- | :--- |
| `0x00000000 - 0x0000FFFF` | 64 KB | AP Internal SRAM (IRAM) | On-chip fast SRAM. Holds `FDL1` (loaded at `0x00006200`). Stack starts at top of SRAM. | Verified (Spreadtrum BSL & FDL specs) |
| `0x00010000 - 0x00012FFF` | 12 KB | Shared SRAM / SBUF | Audio, AON, and inter-processor communication buffers. | Documented |
| `0x40000000 - 0x4FFFFFFF` | 256 MB | AON & Core Peripherals | Clock controller, power management, watchdog, reset controller. | Documented in Unisoc datasheets |
| `0x50000000 - 0x50000FFF` | 4 KB | SPRD UART0 (Debug Serial) | Main serial console and FDL UART channel. | Verified (Linux `sprd_serial.c`) |
| `0x50010000 - 0x50010FFF` | 4 KB | SPRD UART1 | Secondary serial interface / AT command port. | Documented |
| `0x50020000 - 0x50020FFF` | 4 KB | SPRD UART2 | Bluetooth / Modem communication port. | Documented |
| `0x50030000 - 0x50030FFF` | 4 KB | SPRD Timers | High-resolution system tick and watchdog timers. | Documented |
| `0x50040000 - 0x50040FFF` | 4 KB | SPRD GPIO / Keypad | Keypad matrix scanner and GPIO lines. | Documented |
| `0x50080000 - 0x50080FFF` | 4 KB | SPRD SPI0 (LCD Controller) | High-speed SPI to GC9307 QVGA Display panel. | In progress |
| `0x70000000 - 0x7FFFFFFF` | 256 MB | System Bus Matrix / Crypto | Security engine, DMA controller, fast bus arbitration. | Documented |
| `0x80000000 - 0x82FFFFFF` | 48 MB | System DRAM (LPDDR2) | Main physical RAM (Nokia 2660 configuration). `FDL2` loaded at `0x80100000`. Kernel loaded at `0x80000000+`. | Verified (`spd_dump` dumps & vendor specs) |
| `0xFFFF0000 - 0xFFFFFFFF` | 64 KB | High Vector BootROM | CPU boot exception vector before remap to SRAM. | Standard ARMv7-A architecture |

---

## 2. Bootloader & Firmware Load Addresses

| Binary Component | Target Address | Execution Mode | Purpose |
| :--- | :--- | :--- | :--- |
| **BootROM** | `0xFFFF0000` / `0x00000000` | ARM / Supervisor | Initial chip ROM. Checks boot key (e.g. `*` key), handles USB/UART download mode handshake. |
| **FDL1 (First Downloader)** | `0x00006200` | ARM32 (SRAM) | Initializes clocks, PLLs, PMU, and DDR memory controller. |
| **FDL2 (Second Downloader)**| `0x80100000` | ARM32 (DRAM) | Flash driver payload; handles read/write of flash partitions over USB/UART. |
| **BOOT0 / SPL** | `0x00006200` or `0x80000000` | ARM32 | Secondary Program Loader on cold boot. |
| **BOOT1 / LK (Little Kernel)**| `0x80020000` | ARM32 | Hardware abstraction layer, LCD init splash screen, kernel loader. |
| **MOCOR OS / RTOS** | `0x80040000` | ARM32 / Thumb-2 | Main feature phone operating system image. |

---

## 3. SPRD UART Register Definitions

Unisoc/Spreadtrum UART IP block uses standard 32-bit aligned registers at base address `0x50000000`:

| Offset | Register Name | Description | Access | Reset Value |
| :--- | :--- | :--- | :--- | :--- |
| `0x0000` | `SPRD_TXD` | Transmit Data Register (write 8-bit char) | W | `0x00000000` |
| `0x0004` | `SPRD_RXD` | Receive Data Register (read 8-bit char) | R | `0x00000000` |
| `0x0008` | `SPRD_STS0` | Status Register 0 | R | `0x00000000` |
| `0x000C` | `SPRD_STS1` | Status Register 1 (FIFO count & ready flags) | R | `0x00000000` (bit 8 = TX FIFO empty) |
| `0x0010` | `SPRD_IEN` | Interrupt Enable Register | R/W | `0x00000000` |
| `0x0014` | `SPRD_ICLR` | Interrupt Clear Register | W | `0x00000000` |
| `0x0018` | `SPRD_CTL0` | Control Register 0 | R/W | `0x00000000` |
| `0x001C` | `SPRD_CTL1` | Control Register 1 | R/W | `0x00000000` |
| `0x0020` | `SPRD_CTL2` | Control Register 2 | R/W | `0x00000000` |
| `0x0024` | `SPRD_CLKD0`| Clock Divider Low | R/W | `0x00000000` |
| `0x0028` | `SPRD_CLKD1`| Clock Divider High | R/W | `0x00000000` |

### TX FIFO Status Emulation Note
In guest software polling mode, guest code checks `SPRD_STS1` to verify there is space in the transmit FIFO. When emulating in QEMU or Unicorn:
- Bit 8 (or TX count < FIFO size) must indicate ready / space available, otherwise loops polling TX ready will hang indefinitely.

---

## 4. Unknown MMIO Handling Principle

In alignment with project philosophy:
- Any access outside mapped regions (`0x00000000 - 0x0000FFFF`, `0x50000000 - 0x50000FFF`, `0x80000000 - 0x82FFFFFF`) triggers an MMIO logger event.
- Reads from unknown MMIO return `0x00000000` by default while printing:
  `[MMIO TRAP] Unknown read at 0x%08X (size %u) from PC 0x%08X`
- Writes to unknown MMIO print:
  `[MMIO TRAP] Unknown write at 0x%08X (size %u) val 0x%08X from PC 0x%08X`
- This ensures hardware dependencies are tracked and recorded without guessing behavior.
