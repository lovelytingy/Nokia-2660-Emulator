# Nokia 2660 Flip Native Desktop GUI & Developer Deck

Copyright (c) 2026 N2660Emu Developers  
SPDX-License-Identifier: MIT

---

## 1. Overview

The **N2660Emu Desktop GUI** is a native host window application built entirely in **Rust** (leveraging `minifb` with Cocoa on macOS and X11/Wayland on Linux). It delivers an authentic physical hardware skin of the **Nokia 2660 Flip** (Model TA-1480, Unisoc T107 / UMS9117 platform) alongside an integrated real-time **Hardware Telemetry & Serial Console Deck** in a unified 990×860 window.

```
+------------------------------------+-------------------------------------------+
|      NOKIA 2660 FLIP (0..490)      |        DEVELOPER DECK (500..990)          |
|                                    |                                           |
|       [ Upper Clamshell ]          | [FLIP: OPEN]  [PAUSE/RESUME]  [STEP]      |
|   +----------------------------+   +-------------------------------------------+
|   |    NOKIA Chrome Badge      |   | HARDWARE TELEMETRY (UNISOC T107 CORTEX-A7)|
|   |  +----------------------+  |   |   PC   : 0x80000000     SP   : 0x8003FF00 |
|   |  |                      |  |   |   LR   : 0x80000000     CPSR : 0x600000D3 |
|   |  |   240x320 Main LCD   |  |   |   Speed: 4.85 MIPS      Insns: 1,420,000  |
|   |  |   (GC9307 SPI0)      |  |   |   Hall Sensor : Pin 0 = 1 (OPEN)          |
|   |  |                      |  |   |   INTC Lines  : IRQ 0..31 Ready           |
|   |  +----------------------+  |   |   Audio DAC   : 1000 Hz Tone Buzzer Ready |
|   +----------------------------+   +-------------------------------------------+
|       [ Dual Metallic Hinge ]      | LIVE SERIAL CONSOLE (UART0 @ 115200 8N1)  |
|   +----------------------------+   |   [BOOT] ARM Cortex-A7 initialized        |
|   |  [Soft1]   [^]   [Soft2]   |   |   [TEST] Internal SRAM (IRAM) verified    |
|   |  [ < ]    [OK]    [ > ]    |   |   [PASS] GC9307 240x320 display verified  |
|   |  [CALL]   [ v ]   [END]    |   |   [TEST] SPRD Keypad Matrix & Hall Sensor |
|   |  [1 .@]   [2 ABC] [3 DEF]  |   |   [PASS] Keypad & flip sensor verified    |
|   |  [4 GHI]  [5 JKL] [6 MNO]  |   |   [TEST] SFC Winbond W25N01GV SPI NAND    |
|   |  [7PQRS]  [8 TUV] [9WXYZ]  |   |   [RESULT] *** ALL CHECKS PASSED ***      |
|   |  [* +]    [0 _]   [# ^]    |   |   (Auto-scrolls latest 44 lines at 60 FPS)|
|   +----------------------------+   |                                           |
+------------------------------------+-------------------------------------------+
```

---

## 2. Architecture & Design

### 2.1 Single Authentic Theme: Nokia Deep Navy Blue
The emulator standardizes on the signature physical finish of the real Nokia 2660 Flip:
- **Body Casing**: Deep Navy Blue (`#14223A`) with beveled outer boundaries (`#0E192B`).
- **Metallic Hinge**: Multi-segment brushed aluminum hinges (`#7A889B` / `#566270`) connecting upper and lower clamshell assemblies.
- **Keypad Tactile Buttons**: Embossed slate-blue keys (`#1E2E46`) with high-contrast T9 legends, green Call accent (`#1B6328`), red End accent (`#781E22`), and chrome D-Pad ring (`#32425E`).

### 2.2 Dual Display Pipeline
1. **Main Display (GC9307)**: 240×320 QVGA, 16-bit RGB565 over SPI0 (`0x50080000`).
2. **Outer Sub-Display (ST7735)**: 120×160 QQVGA, 16-bit RGB565 over SPI1 (`0x50090000`).

Both framebuffers convert directly to 32-bit XRGB host window pixels via fast bitwise channel scaling:
```rust
fn rgb565_to_xrgb(c: u16) -> u32 {
    let r = (((c >> 11) & 0x1F) * 255 / 31) as u32;
    let g = (((c >> 5) & 0x3F) * 255 / 63) as u32;
    let b = ((c & 0x1F) * 255 / 31) as u32;
    (r << 16) | (g << 8) | b
}
```

### 2.3 Interactive Keypad Matrix & Hit-Testing
Clicking any keypad button triggers geometric hit-testing against the phone keypad matrix:
- **Visual Feedback**: The clicked key is dynamically highlighted with active accent colors.
- **Hardware Injection**: Button events directly set the corresponding row/column bits in the Unisoc Keypad controller registers (`STATUS0` / `STATUS1` at `0x50060000` / `0x50060004`) and trigger INTC IRQ 5.

### 2.4 Magnetic Clamshell Hall Effect Flip Sensor
The Nokia 2660 Flip uses an internal magnetic Hall effect sensor connected to **SPRD GPIO Bank 0 Pin 0** (`0x50040004`):
- **Flip Open**: Pin 0 is pulled HIGH (`0x1`). Renders the main 240×320 GC9307 display and enables full keypad interaction.
- **Flip Closed**: Pin 0 is pulled LOW (`0x0`). Transitions to closed clamshell casing, displaying the camera module pill, optical lens reflection, warm LED flash, outer Nokia logo, and active 120×160 sub-display clock/notifications.

Toggle the flip state anytime via the toolbar **`[FLIP]`** button or the `f` / `F` key.

### 2.5 Live Serial Console & Hardware Telemetry
The companion developer deck (X: 500..990) features:
- **Zero-Dependency 5×7 Bitmap Font**: Embedded 96-glyph ASCII font rendering crisp text directly into the frame buffer without requiring system font libraries.
- **Live Auto-scrolling UART0 Stream**: Continuously pulls TX buffer data from the virtual UART0 FIFO at 60 FPS, color-coding log entries (`[PASS]` in emerald green, `[TEST]` in cyan, `[MMIO TRAP]` in amber, and general telemetry in cool gray).
- **Live CPU & SoC Telemetry**: Real-time readouts of PC, SP, LR, CPSR, instruction counts, dynamic execution speed (MIPS), GPIO Hall sensor, INTC lines, and Audio DAC status.

---

## 3. Controls Reference Table

| Host Key | Mouse Action | Nokia 2660 Function | Hardware Matrix Mapping |
| :--- | :--- | :--- | :--- |
| `0` – `9` | Click Button `0`–`9` | Numeric T9 Input | `STATUS0` bits 0–9 |
| `*` | Click Button `*` | Symbol / Star | `STATUS0` bit 10 |
| `#` | Click Button `#` | Shift / Space / Hash | `STATUS0` bit 11 |
| `c` / `C` | Click `CALL` | Call / Answer / Dial | `STATUS1` bit 0 |
| `x` / `e` / `Esc` | Click `END` | End Call / Power / Clear | `STATUS1` bit 1 |
| `[` | Click Left SoftKey | Left SoftKey (Menu) | `STATUS1` bit 2 |
| `]` | Click Right SoftKey | Right SoftKey (Back) | `STATUS1` bit 3 |
| `Up` Arrow | Click `^` | D-Pad Navigate Up | `STATUS1` bit 4 |
| `Down` Arrow | Click `v` | D-Pad Navigate Down | `STATUS1` bit 5 |
| `Left` Arrow | Click `<` | D-Pad Navigate Left | `STATUS1` bit 6 |
| `Right` Arrow | Click `>` | D-Pad Navigate Right | `STATUS1` bit 7 |
| `Return` / `Space` | Click `OK` | D-Pad Center OK | `STATUS1` bit 8 |
| `f` / `F` | Click `[FLIP]` | Clamshell Open $\leftrightarrow$ Closed | GPIO Bank 0 Pin 0 |
| `p` / `P` | Click `[PAUSE]` | Pause / Resume CPU Execution | Host VM Step Clock |
| `s` / `S` | Click `[STEP]` | Single Step Instruction Slice | Step 1,000 Instructions |

---

## 4. Usage

### Launch via Cargo:
```bash
cargo run --release -p n2660-cli -- --gui tests/baremetal/test_firmware.bin
```

### Launch via Launcher Binary:
```bash
# Launch native desktop GUI
./bin/n2660emu tests/baremetal/test_firmware.bin --gui

# Launch with clamshell initially closed
./bin/n2660emu tests/baremetal/test_firmware.bin --gui --flip closed
```
