# Unisoc T107 (UMS9117) Peripheral Specifications

## 1. SPRD UART (Serial Communication Interface)

The T107 SoC provides three high-speed UART interfaces.

- **UART0 (Debug / Console)**: Base `0x50000000`
- **UART1 (Modem / AT Commands)**: Base `0x50010000`
- **UART2 (Bluetooth / Auxiliary)**: Base `0x50020000`

### Register Map
- `+0x0000`: `SPRD_TXD` (Write: 8-bit character transmitted)
- `+0x0004`: `SPRD_RXD` (Read: 8-bit character received)
- `+0x0008`: `SPRD_STS0` (Status 0)
- `+0x000C`: `SPRD_STS1` (Status 1: Bit 8 = TX FIFO Empty, Bit 10 = RX FIFO Empty)
- `+0x0010`: `SPRD_IEN` (Interrupt Enable)
- `+0x0014`: `SPRD_ICLR` (Interrupt Clear)
- `+0x0018`: `SPRD_CTL0` (Line Control)
- `+0x001C`: `SPRD_CTL1` (Flow Control)
- `+0x0020`: `SPRD_CTL2` (FIFO Control)
- `+0x0024`: `SPRD_CLKD0` (Baud Rate Divisor Low)
- `+0x0028`: `SPRD_CLKD1` (Baud Rate Divisor High)

---

## 2. Display Controller & Panel (GalaxyCore GC9307)

The Nokia 2660 Flip primary screen is a 2.8-inch QVGA (240x320) TFT LCD driven by a GalaxyCore GC9307 controller over a 4-line SPI or 8-bit MCU 8080 parallel bus.

- **SPI0 Base Address**: `0x50080000`
- **Resolution**: 240 x 320 pixels
- **Color Depth**: RGB565 (16 bits per pixel)
- **Outer Secondary Display**: 1.77-inch QQVGA (120x160 pixels)

---

## 3. Keypad Matrix & Hall Sensor

- **Base Address**: `0x50040000`
- Scans a matrix of rows and columns corresponding to:
  - Numeric keys (`0-9`, `*`, `#`)
  - Navigation D-pad (Up, Down, Left, Right, Center Select)
  - Soft keys (Left Softkey, Right Softkey)
  - Call / End Call keys
  - Emergency Assistance button on rear
  - Hall effect sensor detecting flip open / closed state

---

## 4. Timers and Watchdog

- **Base Address**: `0x50030000`
- Includes high-precision 32-bit hardware timers clocked by 32.768 kHz RTC clock and 26 MHz system oscillator.
- Hardware watchdog timer automatically resets the SoC if not kicked during boot.
