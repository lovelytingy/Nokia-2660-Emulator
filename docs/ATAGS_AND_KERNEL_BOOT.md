# ARM Boot Tags (ATAGS) & Kernel Handoff Architecture

## 1. Overview

The Nokia 2660 Flip (Model TA-1480), powered by the Unisoc T107 (UMS9117) SoC, runs a Cortex-A7 single-core CPU that executes multi-stage bootloaders (BOOT0, FDL1, FDL2) before handing off execution to the operating system kernel (MOCOR RTOS or ARM Linux).

Under the standard ARM Linux boot protocol adopted by Spreadtrum / Unisoc application processors, the bootloader communicates essential hardware parameters, memory geometries, board identifiers, and kernel command lines to the target OS kernel through **ARM Boot Tags (ATAGS)** placed in low physical DRAM.

```
+-------------------------------------------------------------------------+
|                              Unisoc T107                                |
|                           Physical DRAM Map                             |
+-------------------------------------------------------------------------+
0x80000000 +-------------------------------------------------------------+
           | Page 0 / ARM Exception Vectors (Reserved)                   |
0x80000100 +-------------------------------------------------------------+
           | ARM Boot Tags (ATAGS List)                                  |
           |   - ATAG_CORE     (5 words: pagesize 4096, flags, rootdev)  |
           |   - ATAG_MEM      (4 words: 48 MB @ 0x80000000)             |
           |   - ATAG_CMDLINE  (variable: console, rootfs, mem options)  |
           |   - ATAG_REVISION (3 words: board HW revision 0x01)         |
           |   - ATAG_SERIAL   (4 words: 64-bit device serial number)    |
           |   - ATAG_NONE     (0 words: list terminator 0x00000000)     |
0x80008000 +-------------------------------------------------------------+
           | Target Kernel Image (zImage / MOCOR OS Binary)              |
           | Code & Read-Only Data                                       |
           | BSS & System Stacks                                         |
0x83000000 +-------------------------------------------------------------+ (End of 48 MB DRAM)
```

---

## 2. ARM CPU Register State at Kernel Entry

Prior to branching to the operating system entry point, the bootloader or emulator must establish a well-defined architectural state:

| Register | Architectural Value | Description |
|:---|:---|:---|
| **`r0`** | `0x00000000` | Mandatory zero requirement per ARM boot protocol |
| **`r1`** | `0x00009117` | Machine / Architecture ID for Unisoc UMS9117 / T107 |
| **`r2`** | `0x80000100` | Physical DRAM pointer to the contiguous ATAGS tag chain |
| **`CPSR`**| `0x000000D3` | Supervisor (SVC) mode, IRQ disabled (I=1), FIQ disabled (F=1), ARM state |
| **MMU**  | Disabled | CP15 `SCTLR.M = 0` (Flat physical memory addressing) |
| **D-Cache** | Disabled | CP15 `SCTLR.C = 0` (Data cache disabled / flushed) |
| **I-Cache** | Optional | CP15 `SCTLR.I` enabled or disabled; clean state |

---

## 3. Tag Data Structures & Header Format

An ATAG list consists of a contiguous array of 32-bit word-aligned structures. Every tag begins with a 2-word header followed by tag-specific payload:

```c
struct tag_header {
    uint32_t size; /* Length of the tag in 32-bit words, including this header */
    uint32_t tag;  /* Unique 32-bit tag identifier */
};
```

### Supported Tag Types

| Tag Identifier | Hex Magic | Size (Words) | Payload Definition |
|:---|:---|:---:|:---|
| **`ATAG_NONE`** | `0x00000000` | 0 | **Terminator**: Signals end of the ATAG list (`size = 0, tag = 0`). |
| **`ATAG_CORE`** | `0x54410001` | 5 | **Header**: `flags` (32-bit), `pagesize` (usually 4096), `rootdev` (32-bit). Mandatory first tag. |
| **`ATAG_MEM`** | `0x54410002` | 4 | **Memory Window**: `size` (bytes, e.g. `0x03000000` for 48 MB), `start` (base address `0x80000000`). |
| **`ATAG_CMDLINE`** | `0x54410009` | 2 + N | **Boot Arguments**: Null-terminated ASCII command line string, padded with zeroes to 4-byte word boundary. |
| **`ATAG_REVISION`** | `0x54410007` | 3 | **Board Revision**: 32-bit hardware board revision number (`0x00000001`). |
| **`ATAG_SERIAL`** | `0x54410006` | 4 | **Serial Number**: 64-bit unique board serial (`low` 32-bit word, `high` 32-bit word). |
| **`ATAG_INITRD2`** | `0x54420005` | 4 | **Initial Ramdisk**: `start` (physical address in DRAM), `size` (bytes). |

---

## 4. Emulation & Tooling Support

### Synthesizer & Inspector: `tools/atags.py`

`tools/atags.py` provides standalone synthesis and structural inspection of ARM Boot Tags:

#### Generating an ATAGS Binary
```bash
python3 tools/atags.py build -o build/atags.bin \
    --dram-base 0x80000000 \
    --dram-size 0x03000000 \
    --cmdline "console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M earlycon"
```

#### Inspecting an ATAGS Binary
```bash
python3 tools/atags.py inspect build/atags.bin --base 0x80000100
```

Sample output:
```
======================================================================
 ARM Boot Tags (ATAGS) Inspection
======================================================================
[0x80000100] ATAG_CORE    : pagesize=4096, flags=0x00000000
[0x80000114] ATAG_MEM     : start=0x80000000, size=50,331,648 bytes (48 MB)
[0x80000124] ATAG_CMDLINE : "console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M earlycon"
[0x80000168] ATAG_REVISION: 0x00000001
[0x80000174] ATAG_SERIAL  : 0x2660148000000001
[0x80000184] ATAG_NONE    : (Terminator)
======================================================================
```

---

## 5. Emulator CLI Options

The emulator CLI (`n2660emu` and `tools/unicorn_runner.py`) supports full ATAGS injection and kernel register setup:

| Option | Argument | Description | Default |
|:---|:---|:---|:---|
| `--atags` | None | Stage standard ARM ATAGS in DRAM and setup `r0=0, r1=0x9117, r2=atags_addr` | Disabled |
| `--atags-addr` | `<hex>` | Physical DRAM address to stage ATAGS | `0x80000100` |
| `--board-id` | `<hex>` | ARM machine/board architecture ID passed in register `r1` | `0x9117` |
| `--cmdline` | `"<str>"` | Kernel command line arguments embedded in `ATAG_CMDLINE` | `console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M earlycon` |

### Example Invocations

#### Booting a Linux zImage with ATAGS
```bash
./bin/n2660emu zImage --entry 0x80008000 --atags \
    --cmdline "console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M"
```

#### Running Baremetal Verification with ATAGS Validation
```bash
./bin/n2660emu tests/baremetal/test_firmware.bin --engine unicorn \
    --atags --atags-addr 0x80030000
```

---

## 6. Baremetal Driver & Verification

The baremetal suite incorporates a driver in `tests/baremetal/atags.c` and `tests/baremetal/atags.h` exposing the following API:

- `int atags_validate(const struct tag *tags)`: Verifies list starts with `ATAG_CORE` and cleanly ends with `ATAG_NONE`.
- `const struct tag *atags_find(const struct tag *tags, uint32_t tag_id)`: Searches the list for a specific tag ID.
- `int atags_get_mem(const struct tag *tags, uint32_t *start, uint32_t *size)`: Extracts physical DRAM start and size.
- `const char *atags_get_cmdline(const struct tag *tags)`: Returns a pointer to the null-terminated kernel command line string.

Test 13 in `tests/baremetal/main.c` validates:
1. Emulator boot handoff registers (`r0=0, r1=0x9117, r2=atags_addr`).
2. Structural integrity and tag decoding of the active tag list.
3. Accurate parsing of physical RAM boundaries (48 MB @ `0x80000000`).
4. Proper retrieval of kernel command line options and board serial number.
