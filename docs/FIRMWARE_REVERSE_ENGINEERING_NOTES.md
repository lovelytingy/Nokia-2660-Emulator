# Nokia 2660 Flip (Unisoc T107 / UMS9117) Firmware Reverse-Engineering Notes
<!-- SPDX-License-Identifier: MIT -->

**Session Date:** November 6, 2026  
**Target Device:** Nokia 2660 Flip (Model TA-1480, SKU3 `13ARA80IN00`)  
**Platform SoC:** Unisoc T107 / UMS9117 (ARM Cortex-A7 @ 1.0 GHz, ARMv7-A)  
**Firmware Container:** `v23_update_Nokia_2660.pac` (39.5 MB, MOCOR 20A MP W22.04.6 P5)

---

## 1. Executive Summary & Major Milestones

We have achieved complete native machine code execution of the vendor's primary Stage 1 bootloader (`BOOT0` / SPL) directly from the official Nokia PAC container:

```text
DDR init start
sdram_clk_init begin
dmc_dll_init end
lpddr_powerup_init end
dmc_init_post_setting end
qos init end
DDR init OK
boot0 started!: 44bd
boot0 UART/JTAG  mode = 2
boot0 nand flash ID 0xbaef!
load boot1 suc
run boot1:7f87
```

All vendor hardware initialization algorithms have been reverse-engineered, modeled, and verified in Rust (`crates/n2660-core`).

---

## 2. Reverse-Engineered Hardware Peripherals & MMIO Maps

### 2.1 Unisoc T107 Dynamic Memory Controller (DMC) & DDR Delay PHY
* **Base Address:** `0x3000_0000` (`SPRD_DMC_BASE`)
* **Key Registers & Semantics:**
  - `0x3000_0008` (`DMC_DLL_STATUS`): Read in calibration loop at `0x924A`. Bit 17 indicates DLL phase comparator feedback. The algorithm sweeps delay line taps [0..31]; bit 17 must transition from 0 to 1 when tap count passes the lock threshold (modeled at `>= 16`).
  - `0x3000_0180` (`DMC_BIST_CMD`): Built-in self test status. Reads must return bits `1 << 2` (`BIST_DONE`) and `1 << 3` (`BIST_PASS`) (`val |= 0x0C`).
  - `0x3000_0304`, `0x3000_0404`, `0x3000_0504` (`DMC_SLICE_CAL`): Byte-slice delay line calibration status. Must return bit `1 << 28` asserting calibration lock.

### 2.2 Always-On System Timer (AON Systimer)
* **Base Address:** `0x4023_0000` (`SPRD_SYSTIMER_BASE`)
* **Key Registers & Semantics:**
  - Offsets `0x00`, `0x04`, `0x0C`: Free-running microsecond monotonic counter.
  - Used by `boot0` delay loops at `0x7416` and `0x767C`. Must dynamically advance (modeled as `(instruction_count >> 4) as u32`) so spinlocks do not hang.

### 2.3 Hardware Serial Flash Controller (SFC / SPI NAND)
* **Base Address:** `0x21A0_0000` (`SPRD_HW_SFC_BASE`)
* **Key Registers & Semantics:**
  - Offset `0x10` (`SFC_STATUS`): Command execution status. Bit 24 asserts `CMD_DONE` (`1 << 24`). Bit 27 must remain 0 (`CMD_ERR = 0`).
  - Offset `0x19C` (`SFC_RDATA`): SPI NAND flash JEDEC ID. Must return `0x0000_BAEF` (Winbond W25N01GV, entry #58 in vendor flash ID table at `0x0000C5B8`).

### 2.4 Boot Debug UART
* **Base Address:** `0x7010_0000` (`SPRD_DEBUG_UART_BASE`)
* **Key Registers & Semantics:**
  - Offset `0x00` (`UART_TXD`): Transmit FIFO register. Written by vendor `putc`/`printf` routines.
  - Offset `0x0C` (`UART_STS1`): FIFO status register. Bit 8 must be clear (FIFO not full) to prevent TX polling hangs.

---

## 3. Disassembly & BOOT0 -> BOOT1 Transition Architecture

Disassembly of `boot0.bin` at offset `0x6BE8` reveals the exact handoff mechanism:

```arm
00006be8:  bl     0x73bc                ; get timestamp
00006bec:  mov    r1, r0
00006bee:  adr    r0, 0x6d74            ; "run boot1:%x\n"
00006bf0:  bl     0x6eb8                ; printf("run boot1:%x\n", 0x7F87)
00006bf4:  ldr    r0, [pc, #396]        ; Load r0 = 0x80100200 (BOOT1 entry point)
00006bf6:  blx    0x6468                ; Branch with link & exchange: Thumb -> ARM mode!
00006bfa:  b.n    0x6bfa                ; Infinite halt loop (fallback)
```

At target address `0x6468` (executing in **ARM mode**):

```arm
00006468:  bl     0xc1d4                ; Call CP15 cache & MMU flush routine
0000646c:  mov    pc, r0                ; Branch directly to BOOT1 entry (0x80100200) in ARM mode!
```

Inside the cache cleaning routine at `0xC1D4`:
- Disables interrupts (`msr CPSR_c, r6`).
- Cleans and invalidates Data and Instruction Caches via CP15 coprocessor `p15`:
  `mcr p15, 0, fp, c7, c14, 2` (Clean and invalidate D-Cache line by set/way).
- Reads Cache Size ID Register (`mrc p15, 1, r1, c0, c0, 0`).
- Issues memory barriers `dsb sy`, `dmb sy`.
- Restores CPSR and returns via `pop {r0, r1, r2, r3, r6, r7, r8, pc}`.

---

## 4. Boot Execution Addresses & Memory Staging

| Partition | Size (bytes) | PAC Load Address | Emulator Staging Address | Entry Point | State |
|-----------|-------------|------------------|--------------------------|-------------|-------|
| `BOOT0`   | 31,620      | `0x80000001`     | `0x0000_6200` (IRAM)     | `0x0000_6400` | ARM -> Thumb |
| `BOOT1`   | 161,364     | `0x80000002`     | `0x8010_0000` (DRAM)     | `0x8010_0200` | ARM mode |
| `Kernel`  | 12,976,468  | `0x80000003`     | `0x8000_0000` (DRAM)     | `0x8000_0000` | ARM mode |
| `FDL1`    | 32,596      | `0x40000000`     | `0x0000_6200` (IRAM)     | `0x0000_6400` | Flashing only |
| `FDL2`    | 63,924      | `0x00000000`     | `0x8002_0000` (DRAM)     | `0x8002_0000` | Flashing only |

> **Note on DHTB Headers:** Both `boot0.bin` and `boot1.bin` are packaged with a 512-byte (`0x200`) Unisoc DHTB security header. Code execution begins at `staged_base + 0x200`.

---

## 5. Next Steps for Tomorrow

1. **CP15 CCSIDR Fast-Forward / Set-Way Loop Tuning:**
   - In `0xC524`, `boot0.bin` reads CCSIDR (`mrc p15, 1, r1, c0, c0, 0`).
   - The default value causes an inner loop iterating `1024 * 32768` (~33.5M) cycles.
   - Hook or optimize CP15 / hook PC `0x00006468` to complete cache flush instantly or hook `0xC1D4` return.
2. **Execute BOOT1 (`0x80100200`):**
   - Verify that `0x80100200` executes in ARM mode.
   - Monitor `0x70100000` UART logs as `boot1` performs partition swapping, checks security keys, and locates `KernelImg`.
3. **Kernel Stage Handoff:**
   - Follow `boot1`'s handoff of execution to `0x80000000` (`KernelImg` - MOCOR OS).
4. **Display Framebuffer Hook:**
   - Direct display engine output into the desktop GUI window.
