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
| `BOOT1`   | 161,364     | `0x80000002`     | `0x8010_0000` (DRAM)     | `0x8010_0200` | ARM -> Thumb |
| `Kernel`  | 12,976,468  | `0x80000003`     | `0x80a0_5e00` (DRAM)     | `0x80a0_6000` | ARM -> Thumb-2 |
| `UserImg` | 11,834,228  | `0x80000004`     | `0x8800_0000` (DRAM)     | `0x8847_9F6F` | Thumb-2 |
| `FDL1`    | 32,596      | `0x40000000`     | `0x0000_6200` (IRAM)     | `0x0000_6400` | Flashing only |
| `FDL2`    | 63,924      | `0x00000000`     | `0x8002_0000` (DRAM)     | `0x8010_0220` | Flashing only |

> **Critical Fix in Kernel Staging Address:** In earlier sessions, `KernelImg` was staged at `0x8000_0000`. Because `KernelImg` is 12.97 MB, this directly overwrote `0x8010_0000`..`0x8012_7640` where `BOOT1` resides. Reverse engineering `boot1.bin` confirmed that `KernelImg` is compiled with a DHTB header base at **`0x80a0_5e00`**, with its ARM reset vector at **`0x80a0_6000`** (`+0x200`) jumping to the hardware bringup stub at **`0x80a0_61f0`**.

---

## 5. BOOT1 -> MOCOR Kernel Transition Architecture

### 5.1 FTL Superblock & Recovery Mode Bypass
In `boot1.bin`:
1. `0x801005C4` and `0x80100700` invoke `SCI_FTL_Load(0)` at `0x801136E0`.
2. When physical flash superblocks are unpopulated, `SCI_FTL_Load` returns error code `3` (`FTL_NOT_FORMATTED`), falling into recovery mode (`rsupdate`), which attempts to call an unpopulated flash routine pointer.
3. In `n2660-core`, we intercept `SCI_FTL_Load` (`0x801136E0`):
   - Sets device state `*(0x80571af0) = 2` (LOADED / OPEN).
   - Writes valid geometry attributes (sector size 2048, erase unit 64, total sectors 1024).
   - Returns `r0 = 0` (SUCCESS).
4. `SCI_FTL_GetDevInfo` at `0x80113C30` verifies `*(0x80571af0) == 2` and succeeds.

### 5.2 Kernel Handoff Sequence
At `0x8010070E`:
1. `bl #0x80100c24` calls `load_kernel()`. Since `KernelImg` is already staged in DRAM at `0x80a05e00`, `load_kernel()` returns `r0 = 1` (SUCCESS).
2. `boot1` logs `"load kernel suc"`.
3. Calls `is_secure_boot_enabled()` (`0x80102048`), which returns 0.
4. `cbz r0, #0x80100766` jumps straight to handoff:
   - Prints `"run kernel:%x"`.
   - Loads `r4 = 0x80a06000` (Kernel reset entry).
   - Executes `blx #0x80100274`.
5. At `0x80100274`: Executes clean ARM mode CP15 cache purge, disables MMU in SCTLR, and executes `mov pc, r0`, transferring execution directly to `0x80a06000`.

---

## 6. MOCOR OS Kernel Bring-Up

Upon entry at `0x80a06000`:
1. `0x80a06000: ldr pc, [pc]` loads `0x80a061f0` into PC in ARM mode.
2. `0x80a061f0`: System Control Register (`SCTLR`): clears bit 13 (`0x2000`) to enforce low exception vectors (`0x00000000`).
3. Coprocessor Access (`CPACR` & `FPEXC`): Enables CP10 and CP11 coprocessors (`0x00F0_0000`) and executes `vmsr fpexc, r0` (`0x4000_0000`) to activate hardware VFP/NEON floating-point engine.
4. Banked Core Mode Stacks (`0x80a060b4`): Sets up separate stack pointers across all CPU execution modes (FIQ `0xd1`, IRQ `0xd2`, ABT `0xdb`, UND `0xd7`, SYS `0xdf`, SVC `0xd3`).
5. Scatterloader: Branches to ARM C runtime `__scatterload` (`0x814c029c`), zeroing `.bss` and copying data sections into DRAM, then hands off to `__rt_entry` (`0x814eb0d4`) and `main()` (`0x80a140e0`).
6. Multitasking RTOS Scheduler: Enters ThreadX / SCI scheduler loop (`0x80a0e128` -> `0x80a0e144`) with CPSR `0x13`, managing static task tables (`T_CM`, `T_DIAG`, `T_KPDSVR`, `T_AUDIO`, `T_P_APP`).

---

## 7. Peripheral MMIO & Display Integration

- **LCD Controller (`0x2080_0000`):**
  - Control register (`0x2080_0000`) bit 3 = Start Refresh.
  - Framebuffer pointer register (`0x2080_0024`): physical address = `y_base << 2`.
  - When refresh is triggered, frame buffer data is blitted into the emulator's virtual GC9307 display and bit 0 of `0x2080_011C` (`refresh done`) is set.
- **Bare-Metal Payloads Verified:**
  - `baremetal_lcd.bin`: 13.2M instructions at 6.59 MIPS, live 240x320 framebuffer refresh confirmed.
  - `baremetal_asm.bin`: 18M instructions at 9.02 MIPS, verified all 5 mailbox markers at `0x80700100` (`0x14802660`, `0xCAFEBABE`, `0xDEADBEEF`, loop counter `1048575`, status `1`).

---

## 8. Userland Handoff & MMU Page Table Architecture

### Kernel to Userland Transition
The MOCOR RTOS kernel completes its device initialization sequence and invokes the userland application framework (`UserImg`) via an interworking branch:
1. **Kernel Call (`0x80B2EA8C`)**: Thumb-2 instruction `blx 0x80fefd8c`.
2. **ARM Veneer Trampoline (`0x80FEFD8C`)**: `ldr pc, [pc, #-4]` loading `0x884A838B` from the literal pool at `0x80FEFD90`.
3. **Userland Entry (`0x884A838A`)**: `MS_MMI_Main` entry in `EXEC_USER_IMAGE.bin`.

### Resolving CP15 MMU Page Table Translation Faults
In full firmware execution, the kernel configures CP15 `SCTLR` (`0x00C5107F`) with the MMU enabled (`SCTLR.M = 1`).
- The Translation Table Base Register (`TTBR0`) points to the first-level page table at `0x81640000`.
- During kernel bringup, entries for `0x88000000`..`0x88F00000` (Userland DRAM) were initialized with coarse page table descriptors pointing to L2 tables (`0x81644400`..`0x81648000`). Because the physical flash partitions had not yet populated these second-level entries, all L2 PTEs were zero, triggering an instruction prefetch abort (`EXCP_PREFETCH_ABORT`).
- **Solution:** The emulator maps the 32 MB userland DRAM window (`0x88000000`..`0x8A000000`) directly into the L1 page table at `0x81640000` using standard 1 MB section descriptors (`va | 0x1DE2`, Full Read/Write/Execute access) and performs an architectural TLB flush via CP15 c8 (`TLBIALL`).
- With the page table entries established, the CPU hardware MMU seamlessly executes userland code at 5+ MIPS across 75+ million instructions without faults.

### Key Hardware Intercepts Solved
- **PMIC eFuse Polling Loop (`0x80D61EFA..0x80D61F1A`)**: Intercepted reads of `0x40608314` (SC2720 PMIC eFuse status) to return `0xFFFF` (`ready`), preventing 300,000 tick delay loops.
- **LCD Backlight Assertion (`lcd_backlight.c:411`)**: Clamped brightness requests at `0x80CC901C` and `0x80CC9054` to `<= 100`.
- **AON APB Chip ID (`0x402E00FC`)**: Pre-initialized to `0x98180000` (Unisoc T107 / UMS9117-L), satisfying `init.c:290`.
- **BML Flash Partition Emulation (`0x811D702C..0x811D7856`)**: Emulated Block Management Layer partition handles and 128 KB block geometry, satisfying `init.c:275`.
