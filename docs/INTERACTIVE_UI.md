# N2660Emu: Interactive Real-Time Terminal GUI & Keypad Runner

This document describes the interactive real-time terminal user interface (`tools/gui_runner.py` and `./bin/n2660emu -i`), its display rendering architecture, keypad matrix mapping, and the baremetal interactive OS demonstration firmware.

---

## 1. Overview

The interactive runner provides a zero-dependency, real-time terminal dashboard emulating the complete physical Nokia 2660 Flip (Model TA-1480):
- **Live Dual-Display Rendering**: 240×320 QVGA main TFT LCD (GalaxyCore GC9307) and 120×160 QQVGA outer sub-display (Sitronix ST7735) rendered using ANSI 24-bit Truecolor half-block characters (`▀`).
- **Nokia T9 Keypad Matrix**: Full visual keypad widget with live press highlighting, mapping PC keyboard strokes to Unisoc SPRD Keypad controller registers (`0x50060000`) and triggering INTC IRQ 5.
- **Hardware Telemetry Dashboard**: Real-time display of ARM Cortex-A7 CPU registers (`PC`, `SP`, `LR`, `CPSR`, `R0-R3`), execution mode, MIPS execution rate, INTC status, System Timer, GPIO flip state, and Audio buzzer tone playback.
- **Live Serial Console**: Streaming output from firmware via SPRD UART0 (`0x50000000`).

---

## 2. Launching the Interactive Runner

### Quick Start
```bash
# Build firmware and launch interactive shell:
make interactive

# Or launch directly via CLI:
./bin/n2660emu -i

# Direct invocation with custom options:
./tools/gui_runner.py tests/baremetal/interactive_demo.bin --view dual --fps 30
```

### Command-Line Arguments (`tools/gui_runner.py`)
| Argument | Default | Description |
|---|---|---|
| `binary` | `interactive_demo.bin` | Path to raw firmware binary |
| `--entry` | `0x80000000` | Entry address for binary |
| `--flip` | `open` | Initial flip clamshell state (`open` or `closed`) |
| `--view` | `main` | Display view mode (`main`, `sub`, or `dual`) |
| `--fps` | `30` | Target UI refresh rate in frames per second |
| `--slice` | `25000` | Instructions executed per UI frame tick |
| `--headless` | `False` | Run non-interactively for testing or CI |
| `--timeout` | `2.0` | Execution timeout in headless mode (seconds) |

---

## 3. Keyboard Controls and Nokia 2660 Mapping

Keystrokes in the interactive terminal map directly to physical Nokia 2660 Flip hardware inputs:

| Keyboard Input | Nokia 2660 Physical Key | Hardware Register / Channel | Action in Demo OS |
|---|---|---|---|
| `0` – `9` | Numeric Keys 0–9 | `SPRD_KPD_STATUS0` bits 0–9 | Types digit into dialing buffer |
| `*` / `#` | Star (`*`) / Hash (`#`) | `SPRD_KPD_STATUS0` bits 10–11 | Types `*` or `#` |
| `[` | Left SoftKey (LSK) | `SPRD_KPD_STATUS1` bit 1 | Opens Main Menu |
| `]` | Right SoftKey (RSK) | `SPRD_KPD_STATUS1` bit 2 | Back / Clear dialed digit |
| `c` / `C` | Call / Send button | `SPRD_KPD_STATUS1` bit 3 | Initiates phone call |
| `x` / `X` / `e` | End / Power button | `SPRD_KPD_STATUS1` bit 4 | Hangs up call / Clears to Home |
| `Enter` / `Space` | Center / OK button | `SPRD_KPD_STATUS1` bit 0 | Selects menu option |
| `Up` / `Down` | D-Pad Navigation | `SPRD_KPD_STATUS0` bits 12–13 | Scrolls menu items |
| `Left` / `Right`| D-Pad Navigation | `SPRD_KPD_STATUS0` bits 14–15 | Horizontal navigation |
| `!` | Rear Emergency SOS key | `SPRD_KPD_STATUS1` bit 5 | Triggers emergency dialer |
| `+` / `-` | Volume Up / Down keys | `SPRD_KPD_STATUS1` bits 6–7 | Adjusts volume |
| `f` / `F` | Hall Effect Flip Sensor | GPIO Bank 0 Pin 0 (bit 0) | Toggles clamshell Open/Closed |
| `v` / `V` | Display Mode Toggle | Host UI state | Cycles Main → Sub → Dual display |
| `p` / `P` | Pause / Resume | Host VM control | Pauses / resumes execution |
| `t` / `T` | Step Single Instruction | Host VM control | Single-steps CPU (when paused) |
| `s` / `S` | Step 1,000 Instructions| Host VM control | Steps 1,000 instructions |
| `r` / `R` | Reset VM | Host VM control | Resets CPU registers to entry point |
| `q` / `Ctrl+C` | Quit | Host UI state | Restores terminal and exits |

---

## 4. Half-Block ANSI Truecolor Display Architecture

Terminal character cells typically have a ~1:2 width-to-height aspect ratio. To achieve high vertical pixel density without external GUI dependencies, `gui_runner.py` utilizes the Unicode upper half block character:
$$\text{Character Cell} = \texttt{\\033[38;2;R1;G1;B1;48;2;R2;G2;B2m▀}$$

- **Foreground color** (`\033[38;2;...`): RGB888 color of the top pixel.
- **Background color** (`\033[48;2;...`): RGB888 color of the bottom pixel.
- **Result**: Each character cell displays **two independent truecolor vertical pixels**, doubling vertical resolution and maintaining the 3:4 portrait aspect ratio of the phone screen.

---

## 5. Interactive Demo Firmware (`tests/baremetal/interactive_demo.c`)

The interactive demo firmware showcases real-time interaction between hardware components:
1. **Boot Initialization**: Configures CP15 VBAR, initializes UART0 at 115200 baud, initializes GC9307 and ST7735 LCDs, audio tone buzzer, SPRD INTC, and enables Keypad IRQ 5 (`kpd_enable_interrupt()`).
2. **Interrupt-Driven Keypad**: Pressing any key triggers IRQ 5; `kpd_isr_handler` reads `SPRD_KPD_STATUS0/1`, clears the interrupt, and signals `key_event_count`.
3. **Audio Feedback**: Every keypress triggers a 1200 Hz click beep via the SPRD Audio DAC (`0x500C0000`). Call button plays 440 Hz dial tone; End button plays 350 Hz disconnect tone.
4. **Clamshell Flip Reaction**: Toggling the flip closes the main LCD and activates the outer ST7735 screen with digital clock, date, and status indicators.
