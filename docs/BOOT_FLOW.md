# Unisoc T107 (UMS9117) BootROM & Multi-Stage Boot Architecture

## 1. Overview

The Nokia 2660 Flip (model TA-1480), powered by the Unisoc T107 (UMS9117) SoC, implements a multi-stage boot sequence anchored in an on-chip **BootROM**.

The BootROM is the first software executed by the ARM Cortex-A7 core following release from power-on reset. Its primary duties are:
1. Initialize the core CPU state (Supervisor mode, stack pointer in on-chip SRAM).
2. Configure basic UART0 communication for early debug status.
3. Sample hardware boot straps (Keypad matrix `*` key and GPIO boot strap pins).
4. Dispatch between **Normal SPI NAND Flash Boot (`BOOT0`)** and **Spreadtrum BSL Download Mode (`FDL1`)**.

```
                           +------------------------+
                           |  CPU Power-On Reset    |
                           |  Vector at 0x00000000  |
                           +-----------+------------+
                                       |
                                       v
                           +------------------------+
                           |    Unisoc BootROM      |
                           | (IRAM Stack 0x0000FFF0)|
                           +-----------+------------+
                                       |
                      +----------------+----------------+
                      |                                 |
         Download Key '*' pressed?             No download key
         or GPIO Pin 1 asserted?               (Normal boot)
                      |                                 |
                      v                                 v
         +--------------------------+     +--------------------------+
         |    BSL Download Mode     |     |   Probe SPI NAND Block 0 |
         |   UART0 HDLC Protocol    |     |   (Direct Window 0x6000) |
         +------------+-------------+     +-------------+------------+
                      |                                 |
                      | Host sends FDL1                 | Valid SPRD Header?
                      v                                 +---------------+
         +--------------------------+                   |               |
         |    Stage FDL1 in SRAM    |                 Yes               No
         |       at 0x00006200      |                   |               |
         +------------+-------------+                   v               v
                      |                    +------------------+  +-------------+
                      |                    | Copy BOOT0 to    |  |  Fallback   |
                      |                    | SRAM 0x00006200  |  |  BSL Mode   |
                      |                    +--------+---------+  +------+------+
                      |                             |                   |
                      v                             v                   |
         +--------------------------+     +-------------------+         |
         |  Hand off CPU execution  |     |  Branch to BOOT0  |         |
         |      to 0x00006200       |     |   at 0x00006200   | <-------+
         +--------------------------+     +-------------------+
```

---

## 2. Hardware Exception Vector Table

The Cortex-A7 core begins instruction fetch at `0x00000000` (or `0xFFFF0000` if High Vectors `SCTLR.V` are enabled). In N2660Emu, the clean-room BootROM occupies the base of Internal SRAM (`0x00000000` - `0x00001FFF`):

| Offset | Vector Name | Target Handler | Description |
| :--- | :--- | :--- | :--- |
| `0x00000000` | Reset | `reset_handler` | Release from reset; initializes CPU mode and stack. |
| `0x00000004` | Undefined Instruction | `hang` | Traps illegal instructions. |
| `0x00000008` | Software Interrupt (SVC) | `hang` | Supervisor call entry. |
| `0x0000000C` | Prefetch Abort | `hang` | Instruction fetch memory fault. |
| `0x00000010` | Data Abort | `hang` | Data load/store memory fault. |
| `0x00000014` | Reserved | `.word 0` | Reserved by ARM architecture. |
| `0x00000018` | IRQ Interrupt | `hang` | Standard hardware interrupt. |
| `0x0000001C` | FIQ Fast Interrupt | `hang` | Fast hardware interrupt. |

---

## 3. Hardware Boot Strapping & Key Detection

At power-on, the BootROM samples two primary hardware sources:

### A. Keypad Matrix Controller (`0x50060000`)
- **Register**: `KPD_STATUS0` (`0x5006001C`)
- **Key**: Nokia `*` (asterisk / star) key
- **Bit Mapping**: Bit 10 (`1 << 10 = 0x0400`)
- When the user holds the `*` key while powering on or connecting USB, the BootROM detects bit 10 and bypasses flash storage to enter BSL download mode.

### B. GPIO Controller (`0x50040000`)
- **Register**: `GPIO_DATA` (`0x50040000`)
- **Pin**: Bank 0 Pin 1 (`0x00000002`)
- Serves as the hardware strap line and USB VBUS detect signal.

---

## 4. SPRD BOOT0 Header Specification

Unisoc BOOT0 / SPL images are prefixed with a 32-byte header placed at the beginning of Block 0 (Page 0) of the SPI NAND flash:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Magic ("SPRD" = 0x53505244)             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Header Version (1)                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Target SRAM Load Address (0x00006200)       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Payload Image Size (bytes)                  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Execution Entry Point (0x00006200)          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   32-Bit Checksum (Additive Sum)              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Execution Flags (0 = Normal)                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Reserved / Padding (0)                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                   Executable ARMv7-A Payload                  |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### C Header Definition (`tests/baremetal/bootrom.h`):
```c
typedef struct __attribute__((packed)) {
    uint32_t magic;         /* 0x53505244 ("SPRD") or 0x42494D47 ("BIMG") */
    uint32_t version;       /* 1 */
    uint32_t load_addr;     /* Default 0x00006200 */
    uint32_t image_size;    /* Size of payload in bytes */
    uint32_t entry_point;   /* Default 0x00006200 */
    uint32_t checksum;      /* 32-bit addition sum of payload bytes */
    uint32_t flags;         /* Boot options */
    uint32_t reserved;      /* Padding */
} sprd_boot0_hdr_t;
```

---

## 5. Tooling: `bin/mksprd_img`

The emulator provides `bin/mksprd_img` to create, inspect, and inject BOOT0 images:

### Packaging a Payload into BOOT0:
```bash
./bin/mksprd_img create -i payload.bin -o boot0.bin --load-addr 0x00006200 --entry 0x00006200
```

### Inspecting an Existing Image:
```bash
./bin/mksprd_img verify boot0.bin
```

### Injecting into an SPI NAND Image:
```bash
./bin/mksprd_img inject -b boot0.bin -f nand_flash.bin --block 0
```

---

## 6. Running with N2660Emu

### Cold Boot from SPI NAND Flash Block 0:
```bash
./bin/n2660emu --boot0 boot0.bin --gui
```

### Forcing BSL Download Mode with FDL1:
```bash
./bin/n2660emu --bootrom --fdl1 fdl1.bin --bsl
```

### Simulating Blank / Corrupted Flash Fallback:
```bash
./bin/n2660emu --bootrom
```
In this mode, the BootROM probes Block 0, observes blank NAND (`0xFFFFFFFF`), prints a warning, and falls back to BSL recovery mode.
