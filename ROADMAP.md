# Nokia 2660 Flip Emulator (N2660Emu) — Roadmap & Boot Milestones

This document defines the technical roadmap and honest boot milestones for N2660Emu. Each milestone represents a verified phase of hardware emulation and firmware execution on the **Unisoc T107 / UMS9117** platform (ARM Cortex-A7).

> [!IMPORTANT]
> **Strict Engineering Rule**: A milestone is marked completed **only** when original vendor firmware code or verified bare-metal payload code genuinely executes the initialization steps without mocked function redirection or hardcoded graphical replacement.

---

## Boot Milestones

```mermaid
flowchart TD
    M1["Milestone 1: Firmware Ingestion (PAC / XML)"] --> M2["Milestone 2: CPU Execution (ARMv7-A / Thumb)"]
    M2 --> M3["Milestone 3: Early Hardware Init (SRAM / Clock / UART)"]
    M3 --> M4["Milestone 4: Bootloader Execution (BOOT0 / BOOT1)"]
    M4 --> M5["Milestone 5: Memory & Storage (DDR / SPI NAND / FTL)"]
    M5 --> M6["Milestone 6: MOCOR RTOS Kernel (ThreadX Scheduler)"]
    M6 --> M7["Milestone 7: Device Interface (GC9307 / ST7735 / KPD)"]
    M7 --> M8["Milestone 8: Interactive Device (MS_MMI_Main GUI)"]

    style M1 fill:#1b5e20,stroke:#4caf50,color:#fff
    style M2 fill:#1b5e20,stroke:#4caf50,color:#fff
    style M3 fill:#1b5e20,stroke:#4caf50,color:#fff
    style M4 fill:#2e7d32,stroke:#81c784,color:#fff
    style M5 fill:#e65100,stroke:#ff9800,color:#fff
    style M6 fill:#e65100,stroke:#ff9800,color:#fff
    style M7 fill:#b71c1c,stroke:#f44336,color:#fff
    style M8 fill:#b71c1c,stroke:#f44336,color:#fff
```

---

### Milestone 1 — Firmware Ingestion
- [x] Load and parse official Unisoc `.pac` firmware packages (e.g., `v23_update_Nokia_2660.pac`).
- [x] Parse header metadata: Product Name (`UMS9117`), Version (`MOCOR_20A_MP`), Flash Type (`1 = NAND`).
- [x] Identify and extract all 29 partition entries: `FDL1`, `FDL2`, `BOOT0`, `BOOT1`, `KernelImg`, `UserImg`, `MMIRes`, `GSMDSPCODE`, `LTEDSPCODE`, `NV`.
- [x] Parse embedded partition geometry manifest (`UMS9117.xml`).
- [x] Validate image sizes, offsets, and partition boundaries (distinguish physical payloads from virtual partition placeholders).

### Milestone 2 — CPU Execution
- [x] Initialize ARM Cortex-A7 (ARMv7-A / AArch32) execution core using Unicorn Engine 2.1.5.
- [x] Support seamless ARM $\leftrightarrow$ Thumb / Thumb-2 mode switching via CPSR `T` bit (bit 5).
- [x] Configure CP15 Coprocessor Access Control (`CPACR` CP10/CP11 full access) and `FPEXC` (bit 30 = 1) for VFPv4/NEON instructions.
- [x] Verify CPU register read/write integrity across `R0`..`R15`, `CPSR`, and `SPSR`.
- [x] Support execution speeds exceeding 16+ MIPS on Apple Silicon (ARM64 macOS) and Linux.

### Milestone 3 — Early Hardware Initialization
- [x] Clean-room BootROM reset vector at `0x00000000` with SPRD header validation.
- [x] Internal SRAM (64 KB at `0x00000000`) access with cold-boot stack pointer at `0x0000FF00`.
- [x] Clock and reset controller (`0x402E0000` / `0x402E1000`) responding to AHB/APB power-on and soft-reset sequences.
- [x] UART0 console controller (`0x50000000` / `0x70000000`) outputting vendor boot diagnostics.
- [x] Unmapped MMIO access logging: track address, size, value, PC, and access type without crashing Apple Silicon JIT.

### Milestone 4 — Bootloader Execution
- [x] Execute authentic `BOOT0` (SPL) machine code from `boot0.bin`.
- [x] Observe genuine BOOT0 UART outputs:
  - `DDR init start` $\rightarrow$ `sdram_clk_init begin` $\rightarrow$ `dmc_dll_init end` $\rightarrow$ `lpddr_powerup_init end` $\rightarrow$ `dmc_init_post_setting end` $\rightarrow$ `qos init end` $\rightarrow$ `DDR init OK`.
  - `boot0 started!: 44bd`, `boot0 UART/JTAG mode = 2`, `boot0 nand flash ID 0xbaef!`, `load boot1 suc`.
- [x] Execute authentic `BOOT1` (LK) machine code from `boot1.bin` at `0x80100200`:
  - `boot1 started!: 1f734`, `rst_monitor_register is 0`, `boot1 nandflash init!`.
  - `boot1 boot1_PartitionSwap enter` $\rightarrow$ `order by ofst` $\rightarrow$ `replace SCI_FTL_REMAIN_AREA by number` $\rightarrow$ `end`.
- [ ] Eliminate simulated FTL / `load_kernel()` bypass hooks: execute BOOT1 kernel verification and loading directly from virtual NAND flash.

### Milestone 5 — Memory and Flash
- [x] High DRAM (48 MB at `0x80000000`) + Extended DRAM (16 MB at `0x83000000`) mapped to complete 64 MB physical DRAM.
- [x] ThreadX stack and dynamic heap window (`0x78000000..0x80000000`, 128 MB) supporting preemptive thread allocations without page-fault loops.
- [x] Userland application window (`0x88000000..0x8A000000`, 32 MB).
- [ ] Populate Winbond W25N01GV SPI NAND flash with genuine partition tables and partition data from `UMS9117.xml`.
- [ ] Emulate full FTL (Flash Translation Layer) sector read/write operations so `SCI_FTL_FormatBoot1` and `rs_ua_execute` succeed naturally.

### Milestone 6 — MOCOR Kernel
- [x] Stage `KernelImg` (`EXEC_KERNEL_IMAGE.bin`) at `0x80A05E00`, entry `0x80A06000`.
- [x] Observe real kernel execution handoff: `run kernel:20eed`.
- [x] Execute 40+ million instructions of genuine MOCOR RTOS kernel code:
  - Task creation, ThreadX stack pointer setup (`SP = 0x7DD73020`).
  - Preemptive scheduler startup and IPC queue allocation.
- [ ] Connect timer interrupt tick (`0x40230000` / SPRD System Timer) to drive ThreadX preemptive scheduler time slicing.

### Milestone 7 — Device Interface
- [x] Emulate GalaxyCore GC9307 (SPI / LCM) primary 2.8" QVGA (240x320) display controller with ID stream `0x009307`.
- [x] Emulate Sitronix ST7735 secondary 1.77" QQVGA (120x160) outer clamshell display controller.
- [x] Emulate 5x5 hardware keypad matrix with real-time down/up event detection.
- [x] Emulate SC2720 PMIC EIC Pin 1 (Power/End key) and Pin 9 (Key 8).
- [x] Emulate GPIO Pin 0 Hall effect clamshell flip sensor.
- [x] Connect genuine firmware framebuffer DMA (`LCDC` `0x20800000` `y_base_addr` and SPI1 `0x70B00000`) to GUI display canvas (removed synthetic 150k instruction mock dialer fallback).
- [x] Display honest multi-stage boot sequence on physical phone LCD (Stage 1 BOOT0 SPL $\rightarrow$ Stage 2 BOOT1 LK $\rightarrow$ Stage 3 MOCOR RTOS Kernel $\rightarrow$ Stage 4 Userland) with live instruction telemetry.

### Milestone 8 — Interactive Device
- [ ] Original MOCOR operating system boots cleanly to userland application framework (`MS_MMI_Main` at `0x88000000`).
- [ ] Desktop Cocoa GUI displays authentic vendor graphics and UI menus generated by real firmware.
- [ ] Physical phone keypad clicks and keyboard inputs navigate MOCOR OS menus.
- [ ] Persistent storage preserves user settings, contacts, and SMS in emulated SPI NAND flash.
