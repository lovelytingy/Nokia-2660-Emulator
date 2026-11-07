# Unisoc T107 (UMS9117) Memory Map Specification

## 1. Memory Regions Overview

The Unisoc T107 (UMS9117) SoC incorporates an ARM Cortex-A7 application core alongside an ARM Cortex-M4 Always-On (AON) subsystem.

The following table summarizes the verified physical memory addresses and device regions:

| Address Range | Size | Region Name | Description | Verification Status |
| :--- | :--- | :--- | :--- | :--- |
| `0x00000000 - 0x0003FFFF` | 256 KB | AP Internal SRAM (IRAM) | On-chip fast SRAM. Holds `BOOT0` (`0x00006200`, entry `0x00006400`) and `FDL1` (`0x00006200`). | Verified (Hardware & Dumps) |
| `0x12000000 - 0x1200FFFF` | 64 KB | ARM Cortex-A7 GIC-400 | Generic Interrupt Controller (Distributor `0x12001000`, CPU Interface `0x12002000`). | Verified (Kernel execution) |
| `0x20800000 - 0x20801FFF` | 8 KB | SPRD LCDC (Display Controller) | QVGA 240x320 Display Controller with RGB565 framebuffer pointer (`y_base` << 2 at `0x20800024`). | Verified (Baremetal & Dumps) |
| `0x20E00000 - 0x20E03FFF` | 16 KB | SPRD AHB Clock & Power | Bus clocks, peripheral power domains, and core clock gating. | Verified (Dumps & RE) |
| `0x21A00000 - 0x21A0FFFF` | 64 KB | SPRD HW SFC (Serial Flash) | Hardware Serial Flash Controller interface to SPI NAND flash (JEDEC ID `0xBAEF`). | Verified (Hardware & Dumps) |
| `0x30000000 - 0x3000FFFF` | 64 KB | SPRD DMC / DDR Delay PHY | Dynamic Memory Controller, BIST registers, and DLL delay lines. | Verified (Dumps & RE) |
| `0x40230000 - 0x40230FFF` | 4 KB | SPRD AON Systimer | Free-running monotonic system microsecond timer. | Verified (Hardware & Dumps) |
| `0x40250000 - 0x40250FFF` | 4 KB | SPRD Keypad Matrix | APB Keypad matrix scanner (STATUS0 & STATUS1). | Verified (Hardware & Dumps) |
| `0x402A0000 - 0x402A1FFF` | 8 KB | SPRD AON Pinmux & Boot Straps | Hardware boot key pinmux (Key 7 UART mode, Key 3 JTAG mode). | Verified (Dumps & RE) |
| `0x402E0000 - 0x402E3FFF` | 16 KB | SPRD Secure Boot & Sys Ctrl | UMS9117 Chip ID (`0x91170000`) and security gate status. | Verified (Dumps & RE) |
| `0x40600000 - 0x40601FFF` | 8 KB | SPRD PMIC ADI Base | SC2720 PMIC ADI bus controller. | Verified (Hardware & Dumps) |
| `0x40608000 - 0x4060BFFF` | 16 KB | SPRD PMIC EIC & APB | PMIC power key (`0x40608280`), Torch LED (`0x40608df4`), Vibrator (`0x40608e00`). | Verified (Hardware & Dumps) |
| `0x50000000 - 0x50000FFF` | 4 KB | SPRD UART0 (Debug Serial) | Main serial console and FDL UART channel. | Verified (Linux `sprd_serial.c`) |
| `0x50010000 - 0x50010FFF` | 4 KB | SPRD UART1 | Secondary serial interface / AT command port. | Documented |
| `0x50020000 - 0x50020FFF` | 4 KB | SPRD UART2 | Bluetooth / Modem communication port. | Documented |
| `0x50030000 - 0x50030FFF` | 4 KB | SPRD Timers | High-resolution system tick and watchdog timers. | Documented |
| `0x50040000 - 0x50040FFF` | 4 KB | SPRD GPIO Controller | General-purpose I/O & Hall effect flip sensor (Bank 0 Pin 0). | Verified |
| `0x50050000 - 0x50050FFF` | 4 KB | SPRD INTC (Interrupt Controller) | 32-line interrupt controller (IRQ/FIQ routing, software triggers). | Verified |
| `0x50060000 - 0x50060FFF` | 4 KB | SPRD Keypad Controller | 8x8 Hardware keypad matrix scanner (STATUS0 & STATUS1). | Verified |
| `0x50080000 - 0x50080FFF` | 4 KB | SPRD SPI0 (GC9307 LCD) | High-speed SPI to primary 2.8" QVGA (240x320) display panel. | Verified |
| `0x50090000 - 0x50090FFF` | 4 KB | SPRD SPI1 (ST7735 LCD) | SPI interface to secondary 1.77" QQVGA (120x160) outer screen. | Verified |
| `0x500A0000 - 0x500A0FFF` | 4 KB | SPRD SFC (Flash Controller)| Serial Flash Controller interface to 128 MB SPI NAND flash. | Verified |
| `0x500C0000 - 0x500C0FFF` | 4 KB | SPRD Audio Subsystem | Audio DAC, PCM FIFO streaming, speaker/headphone routing & buzzer tone generator. | Verified |
| `0x60000000 - 0x67FFFFFF` | 128 MB | Direct Flash Window | High-speed direct memory read window (XIP) into SPI NAND. | Verified |
| `0x70100000 - 0x70100FFF` | 4 KB | SPRD Boot Debug UART | Dedicated bootloader debug UART transmit FIFO. | Verified (Dumps & RE) |
| `0x71300000 - 0x71300FFF` | 4 KB | SPRD APB Clock Gates | APB bus clock gating and enable flags. | Verified (Dumps & RE) |
| `0x72000000 - 0x7FFFFFFF` | 224 MB | ThreadX Heap & Task Stacks | Monolithic RTOS dynamic task stack allocation and heap memory. | Verified (Kernel execution) |
| `0x80000000 - 0x83FFFFFF` | 64 MB | System DRAM (LPDDR2) | Main physical RAM (Nokia 2660 configuration). `BOOT1` (`0x80100000`), `KernelImg` (`0x80a05e00`). | Verified (Hardware dumps) |
| `0x88000000 - 0x89FFFFFF` | 32 MB | Userland Application DRAM | `UserImg` MMI task and application framework (`MS_MMI_Main` at `0x88479F6F`). | Verified (Hardware dumps) |
| `0xA1600000 - 0xA1601FFF` | 8 KB | SPRD ADI Controller | Synchronous serial ADI interface for SC2720 PMIC communication. | Verified (Dumps & RE) |
| `0xA1610000 - 0xA1611FFF` | 8 KB | SPRD Analog Die Window | Analog die configuration and status registers. | Verified (Dumps & RE) |
| `0xFFFF0000 - 0xFFFFFFFF` | 64 KB | High Vector BootROM | CPU boot exception vector before remap to SRAM. | Verified (Clean-Room BootROM) |

---

## 2. Bootloader & Firmware Load Addresses

| Binary Component | Target Load Address | Entry Point | Execution Mode | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| **BootROM** | `0xFFFF0000` / `0x00000000` | `0x00000000` | ARM / Supervisor | Initial chip ROM. Checks boot keys (`*`), handles USB/UART BSL download handshake. |
| **FDL1 (First Downloader)** | `0x00006200` | `0x00006400` | ARM / Thumb | Initializes clocks, PLLs, PMU, and DDR memory controller over USB. |
| **FDL2 (Second Downloader)**| `0x80020000` / `0x80100000` | `0x80100220` | ARM / Thumb | Flash driver payload; handles read/write of flash partitions over USB. |
| **BOOT0 / SPL** | `0x00006200` | `0x00006400` | ARM -> Thumb | First on-NAND stage. Trains LPDDR2 controller, loads `BOOT1`. |
| **BOOT1 / Secondary Loader**| `0x80100000` | `0x80100200` (stub -> `0x80100220`) | ARM -> Thumb | Initializes FTL, verifies `KernelImg`, hands off to kernel in ARM mode. |
| **KernelImg (MOCOR RTOS)** | `0x80a05e00` | `0x80a06000` (stub -> `0x80a061f0`) | ARM -> Thumb-2 | Core monolithic OS: ThreadX scheduler, device drivers, cellular stacks. |
| **UserImg (Application Layer)**| `0x88000000` | `0x88479f6f` (`MS_MMI_Main`) | Thumb-2 | MMK Window Manager, UI theme engine, phonebook, native applications. |

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
