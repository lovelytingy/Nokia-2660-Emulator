# Nokia 2660 Flip Desktop Host GUI (`desktop_gui.py`)

Copyright (c) 2026 N2660Emu Developers  
SPDX-License-Identifier: MIT

---

## 1. Overview

The **N2660Emu Desktop GUI** is a native host window application designed for macOS and Linux that provides an authentic physical skin of the **Nokia 2660 Flip** (Model TA-1480, Unisoc T107 / UMS9117 platform). It features real-time dual display rendering, an interactive 3D-embossed keypad, clamshell flip simulation, hardware telemetry, and an integrated bidirectional serial console.

```
+------------------------------------+-------------------------------------------+
|                                    |  Nokia 2660 Flip Emulator                 |
|       [ Upper Clamshell ]          |  Unisoc T107 | ARM Cortex-A7 @ 1.0 GHz    |
|   +----------------------------+   +-------------------------------------------+
|   |   NOKIA Chrome Badge       |   | [Toolbar: Pause, Step, Reset, Flip, Snap] |
|   |  +----------------------+  |   +-------------------------------------------+
|   |  |                      |  |   | Hardware Telemetry:                       |
|   |  |  240x320 Main LCD    |  |   |   PC:   0x80000000   INTC Raw:  0x00000000|
|   |  |  (GC9307 SPI0)       |  |   |   SP:   0x8003FF00   INTC Mask: 0x00000000|
|   |  |                      |  |   |   LR:   0x80000000   Timer Cnt: 0x0000FFFA|
|   |  +----------------------+  |   |   CPSR: 0x600000D3   Audio Tone: 1000 Hz  |
|   +----------------------------+   |   Insns: 1,420,000   Speed:     4.50 MIPS |
|       [ Dual Metallic Hinge ]      +-------------------------------------------+
|   +----------------------------+   | Live UART0 Serial Console:                |
|   |  (LSK)   [DPAD/OK]   (RSK) |   |   [BOOT] ARM Cortex-A7 initialized        |
|   |  (CALL)              (END) |   |   [LCD] GC9307 240x320 QVGA online        |
|   |  [ 1 ]    [ 2 ]    [ 3 ]   |   |   [DEMO] Ready for user dialing...        |
|   |  [ 4 ]    [ 5 ]    [ 6 ]   |   |   -------------------------------------   |
|   |  [ 7 ]    [ 8 ]    [ 9 ]   |   |   [TX Entry: Type and press Enter -> ]    |
|   |  [ * ]    [ 0 ]    [ # ]   |   |                                           |
|   +----------------------------+   +-------------------------------------------+
+------------------------------------+-------------------------------------------+
```

---

## 2. Architecture & Design

### 2.1 Vector Casing & Hardware Skin (`Nokia2660PhoneSkin`)
The phone casing is rendered via **Cairo 2D vector graphics**, ensuring crisp anti-aliased geometry at any display scale:
- **Upper Clamshell**: Features the earpiece speaker slit, metallic chrome `NOKIA` badge, 240×320 screen glass bezel with a linear glass reflection gradient, and subtle casing curvature.
- **Dual Cylindrical Metallic Hinge**: Rendered with multi-stop linear metallic highlights replicating the real physical folding hinge.
- **Lower Clamshell**: Houses the 3D-embossed keypad with soft keys, illuminated green Call and red End buttons, directional D-Pad navigation ring, central OK button, standard 12-key T9 layout (with primary numbers and secondary alpha characters), and lower microphone pinhole.
- **Closed Clamshell Cover**: Features the rear camera module pill, optical lens reflection, warm LED flash, outer `NOKIA` badge, and outer 120×160 sub-display bezel.

### 2.2 Dual Display Pipeline & Ultra-Fast Framebuffer Conversion
The emulator supports two independent SPI display controllers:
1. **Main Display (GC9307)**: 240×320 QVGA, 16-bit RGB565 over SPI0 (`0x50080000`).
2. **Outer Sub-Display (ST7735)**: 120×160 QQVGA, 16-bit RGB565 over SPI1 (`0x50090000`).

To achieve smooth 60 FPS rendering without CPU overhead, pixel conversion uses a **precomputed 65,536-entry lookup table (`LUT_RGB565_TO_RGB24`)**:
```python
LUT_RGB565_TO_RGB24 = [
    struct.pack(
        '<BBBB',
        (c & 0x1F) * 255 // 31,           # Blue
        ((c >> 5) & 0x3F) * 255 // 63,    # Green
        ((c >> 11) & 0x1F) * 255 // 31,   # Red
        0xFF                              # Alpha / X
    )
    for c in range(65536)
]
```
This converts 76,800 pixels into a native Cairo `ImageSurface` buffer in **~2.0 ms**, easily maintaining 60 Hz display refresh.

### 2.3 Interactive Keypad Matrix & Mouse Hit-Testing
Clicking on the vector phone skin triggers geometric hit-testing (`hit_test(x, y)`) against the keypad button layout:
- **Visual Depression**: Pressing a button translates its label by +1 px and applies dynamic shading gradients (`key_press` color).
- **Hardware Matrix Injection**: Button events directly set the corresponding row/column bits in the Unisoc Keypad controller MMIO registers (`STATUS0` / `STATUS1` at `0x50060000` / `0x50060004`) and trigger INTC IRQ 5.
- **Tactile Sound Effects**: Native low-latency audio click playback via host OS audio (`afplay /System/Library/Sounds/Tink.aiff` on macOS).

### 2.4 Clamshell Hall Effect Flip Sensor
The Nokia 2660 Flip uses an internal magnetic Hall effect sensor wired to **SPRD GPIO Bank 0 Pin 0** (`0x50040004`):
- **Flip Open**: Pin 0 is pulled HIGH (`0x1`). The main 240×320 display is rendered and the keypad is active.
- **Flip Closed**: Pin 0 is pulled LOW (`0x0`). The GUI transitions to the closed clamshell view, showing the camera pill, LED flash, and outer 120×160 clock/notification sub-display.

Toggling can be performed via the toolbar **Flip** button or the `f` / `F` keyboard shortcut.

### 2.5 Hardware Telemetry Deck
The right-hand panel provides live real-time introspection into the Unisoc T107 SoC:
- **CPU Registers**: PC, SP, LR, CPSR, and current ARM execution mode (`USR`, `IRQ`, `SVC`, `ABT`, `UND`, `SYS`).
- **Performance**: Instruction counter and dynamic execution speed in **MIPS** (Million Instructions Per Second).
- **Peripherals**: INTC raw interrupt status, unmasked pending IRQs, SPRD System Timer downcounter (`0x50030004`), Audio tone status (frequency, duration), and UART0 TX byte count.

### 2.6 Live Bidirectional UART0 Serial Console
- **RX Log Output**: Continuously streams characters output by the firmware via SPRD UART0 TX FIFO (`0x50000000`).
- **TX Input Injection**: Allows typing messages or commands into the host text entry and injecting them directly into the virtual UART0 RX FIFO with carriage return for interactive shell debugging.

---

## 3. Casing Color Themes

The GUI includes three selectable factory casing finishes via the toolbar dropdown:
- **Deep Blue**: Signature Nokia 2660 deep navy blue with slate-blue keypad buttons.
- **Classic Black**: Understated matte dark charcoal with refined monochrome accents.
- **Pop Red**: Vibrant high-visibility crimson red with high-contrast keys.

---

## 4. Controls Reference Table

| Host Key | Mouse Action | Nokia 2660 Function | Hardware Matrix Mapping |
| :--- | :--- | :--- | :--- |
| `0` – `9` | Click Button `0`–`9` | Numeric T9 Input | `STATUS0` bits 0–9 |
| `*` | Click Button `*` | Symbol / Star | `STATUS0` bit 10 |
| `#` | Click Button `#` | Shift / Space / Hash | `STATUS0` bit 11 |
| `c` / `C` | Click `CALL` | Call / Answer / Dial | `STATUS1` bit 0 |
| `x` / `e` / `Esc` | Click `END` | End Call / Power / Clear | `STATUS1` bit 1 |
| `[` | Click Left SoftKey | Left SoftKey (Menu / Options) | `STATUS1` bit 2 |
| `]` | Click Right SoftKey | Right SoftKey (Names / Back) | `STATUS1` bit 3 |
| `Up` Arrow | Click `▲` | D-Pad Navigate Up | `STATUS1` bit 4 |
| `Down` Arrow | Click `▼` | D-Pad Navigate Down | `STATUS1` bit 5 |
| `Left` Arrow | Click `◀` | D-Pad Navigate Left | `STATUS1` bit 6 |
| `Right` Arrow | Click `▶` | D-Pad Navigate Right | `STATUS1` bit 7 |
| `Return` / `Space` | Click `OK` | D-Pad Center OK / Select | `STATUS1` bit 8 |
| `f` / `F` | Click `📱 Flip` | Clamshell Open $\leftrightarrow$ Closed | GPIO Bank 0 Pin 0 |
| `p` / `P` | Click `⏸ Pause` | Pause / Resume CPU Execution | Host VM Step Clock |
| `+` / `-` | — | Volume Up / Volume Down | `STATUS1` bits 9, 10 |

---

## 5. Usage

### Launch via Makefile:
```bash
make gui
```

### Launch via CLI Launcher:
```bash
# Default interactive demo in Deep Blue
./bin/n2660emu --gui

# Custom firmware binary with Pop Red skin
./bin/n2660emu tests/baremetal/test_firmware.bin --gui --theme "Pop Red"

# Start with clamshell initially closed
./bin/n2660emu --gui --flip closed
```

### Direct Python Script:
```bash
./tools/desktop_gui.py tests/baremetal/interactive_demo.bin --theme "Classic Black"
```
