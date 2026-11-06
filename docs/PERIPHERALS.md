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

## 2. SPRD System Timers & Watchdog

- **Base Address**: `0x50030000`
- **Clock Source**: 32.768 kHz RTC clock or 26 MHz system clock
- **Modes**: 32-bit and 64-bit width, free-running or periodic down-counter

### Register Map (`sprd,sc9860-timer`)
| Offset | Register Name | Description | Access | Reset |
| :--- | :--- | :--- | :--- | :--- |
| `+0x0000` | `TIMER_LOAD_LO` | Timer Initial Load Value (Low 32-bits) | R/W | `0x00000000` |
| `+0x0004` | `TIMER_LOAD_HI` | Timer Initial Load Value (High 32-bits) | R/W | `0x00000000` |
| `+0x0008` | `TIMER_VALUE_LO` | Current Counter Value (Low 32-bits) | R | `0x00000000` |
| `+0x000C` | `TIMER_VALUE_HI` | Current Counter Value (High 32-bits) | R | `0x00000000` |
| `+0x0010` | `TIMER_CTL` | Control Register (Bit 0: Periodic, Bit 1: Enable, Bit 16: 64-bit) | R/W | `0x00000000` |
| `+0x0014` | `TIMER_INT` | Interrupt Status & Control (Bit 0: EN, Bit 1: Raw, Bit 2: Masked, Bit 3: CLR) | R/W | `0x00000000` |
| `+0x0018` | `TIMER_SHDW_LO` | Atomic Read Shadow Register (Low 32-bits) | R | `0x00000000` |
| `+0x001C` | `TIMER_SHDW_HI` | Atomic Read Shadow Register (High 32-bits) | R | `0x00000000` |

---

## 3. SPRD GPIO Controller

- **Base Address**: `0x50040000`
- **Bank Organization**: 16 GPIO lines per bank, bank stride `0x0080`
- **Total Lines**: Up to 128/256 GPIO lines across 8/16 banks

### Register Map per Bank (`sprd,sc9860-gpio`)
| Offset | Register Name | Description | Access | Reset |
| :--- | :--- | :--- | :--- | :--- |
| `+0x0000` | `SPRD_GPIO_DATA` | Pin Data (Output write / Input read) | R/W | `0x00000000` |
| `+0x0004` | `SPRD_GPIO_DMSK` | Data Bitmask Register | R/W | `0x00000000` |
| `+0x0008` | `SPRD_GPIO_DIR` | Direction Register (`1` = Output, `0` = Input) | R/W | `0x00000000` |
| `+0x000C` | `SPRD_GPIO_IS` | Interrupt Sense (`1` = Level-sensitive, `0` = Edge-sensitive) | R/W | `0x00000000` |
| `+0x0010` | `SPRD_GPIO_IBE` | Interrupt Both Edges Trigger Enable | R/W | `0x00000000` |
| `+0x0014` | `SPRD_GPIO_IEV` | Interrupt Event (`1` = High level / Rising edge, `0` = Low / Falling) | R/W | `0x00000000` |
| `+0x0018` | `SPRD_GPIO_IE` | Interrupt Enable Mask | R/W | `0x00000000` |
| `+0x001C` | `SPRD_GPIO_RIS` | Raw Interrupt Status | R | `0x00000000` |
| `+0x0020` | `SPRD_GPIO_MIS` | Masked Interrupt Status (`RIS & IE`) | R | `0x00000000` |
| `+0x0024` | `SPRD_GPIO_IC` | Interrupt Clear (Write `1` to clear bit) | W | `0x00000000` |
| `+0x0028` | `SPRD_GPIO_INEN` | Input Buffer Enable | R/W | `0x00000000` |

### Nokia 2660 Pin Assignments
- **GPIO Bank 0, Pin 0**: Hall Effect Sensor (Flip Open / Closed state detection). Default: `1` (Open).
- **GPIO Bank 0, Pin 1-7**: Keypad Matrix Rows.
- **GPIO Bank 1, Pin 0-7**: Keypad Matrix Columns.
- **GPIO Bank 2, Pin 0**: Rear Emergency Assistance Button.

---

## 4. Display Subsystem (Dual Screens)

The Nokia 2660 Flip features a dual-display architecture designed for clamshell operation:
1. **Primary Internal Display**: 2.8-inch QVGA (240x320) LCD for main phone interface when the flip is open.
2. **Secondary Outer Sub-Display**: 1.77-inch QQVGA (120x160) LCD on the exterior lid for displaying the clock, battery gauge, signal status, and caller ID when closed.

### 4.1 Primary Screen (GalaxyCore GC9307)
- **Controller**: GalaxyCore GC9307
- **Bus Interface**: Unisoc SPRD SPI0 (Base `0x50080000`)
- **Resolution**: 240 x 320 pixels (QVGA)
- **Pixel Format**: RGB565 (16 bits per pixel)
- **Command Set**: Standard MIPI-DCS (`0x01` SWRESET, `0x11` SLPOUT, `0x29` DISPON, `0x2A` CASET, `0x2B` RASET, `0x2C` RAMWR, `0x3A` COLMOD).

### 4.2 Secondary Outer Sub-Display (Sitronix ST7735 / GC9106)
- **Controller**: Sitronix ST7735S / GalaxyCore GC9106 compatible
- **Bus Interface**: Unisoc SPRD SPI1 (Base `0x50090000`)
- **Resolution**: 120 x 160 pixels (QQVGA, portrait)
- **Pixel Format**: RGB565 (16 bits per pixel)
- **Active Trigger**: Controlled by MOCOR OS based on the Hall effect flip sensor state (GPIO Bank 0, Pin 0). When the flip is closed (`Pin 0 = 0`), power and rendering are routed to the outer screen.

### SPI Host Controller Register Map (SPI0: `0x50080000`, SPI1: `0x50090000`)
| Offset | Register Name | Description | Access | Reset |
| :--- | :--- | :--- | :--- | :--- |
| `+0x0000` | `SPRD_SPI_TXD` | Transmit Data Register (Write commands or pixel stream) | W | `0x00000000` |
| `+0x0004` | `SPRD_SPI_CLKD` | Clock Divider Register | R/W | `0x00000000` |
| `+0x0008` | `SPRD_SPI_CTL0` | SPI Control Register 0 | R/W | `0x00000000` |
| `+0x000C` | `SPRD_SPI_CTL1` | SPI Control Register 1 | R/W | `0x00000000` |
| `+0x0018` | `SPRD_SPI_CTL4` | Data/Command Control (Bit 0: `0` = Command, `1` = Data) | R/W | `0x00000000` |
| `+0x0030` | `SPRD_SPI_STS1` | Status 1 (Bit 8: TX FIFO Empty, Bit 0: Busy) | R | `0x00000100` |

---

## 5. SPRD Keypad Controller (KPD) & Hall Effect Flip Sensor

The Nokia 2660 Flip incorporates an 8x8 matrix keypad interface and a Hall-effect magnetic lid switch for detecting flip opening and closure.

### Keypad Matrix Controller
- **Base Address**: `0x50060000`
- **Register Map**:
| Offset | Register Name | Description | Access | Reset |
| :--- | :--- | :--- | :--- | :--- |
| `+0x0000` | `SPRD_KPD_CTRL` | Control Register (Bit 0: Controller Enable, Bits 1-7: Debounce) | R/W | `0x00000000` |
| `+0x0004` | `SPRD_KPD_INT_EN` | Interrupt Enable (Bit 0: Key Event Interrupt) | R/W | `0x00000000` |
| `+0x0008` | `SPRD_KPD_INT_RAW_STS` | Raw Interrupt Status | R | `0x00000000` |
| `+0x000C` | `SPRD_KPD_INT_MASK_STS` | Masked Interrupt Status (`RAW & EN`) | R | `0x00000000` |
| `+0x0010` | `SPRD_KPD_INT_CLR` | Interrupt Clear (Write `1` to clear) | W | `0x00000000` |
| `+0x0014` | `SPRD_KPD_POLARITY` | Key Scan Polarity | R/W | `0x00000000` |
| `+0x0018` | `SPRD_KPD_CLK_DIV` | Scan Clock Divider | R/W | `0x00000000` |
| `+0x001C` | `SPRD_KPD_STATUS0` | Key Matrix Status Register 0 (Keys 0–15) | R | `0x00000000` |
| `+0x0020` | `SPRD_KPD_STATUS1` | Key Matrix Status Register 1 (Keys 16–31) | R | `0x00000000` |

### Nokia 2660 Key Map & Bit Allocation
| Key Name | Code | Register | Bit | Description |
| :--- | :--- | :--- | :--- | :--- |
| `0` - `9` | `0` - `9` | `STATUS0` | Bits `0` - `9` | Numeric keypad digits |
| `*` (Star) | `10` | `STATUS0` | Bit `10` | Star key |
| `#` (Hash) | `11` | `STATUS0` | Bit `11` | Hash / Lock key |
| `UP` | `12` | `STATUS0` | Bit `12` | D-pad Up |
| `DOWN` | `13` | `STATUS0` | Bit `13` | D-pad Down |
| `LEFT` | `14` | `STATUS0` | Bit `14` | D-pad Left |
| `RIGHT` | `15` | `STATUS0` | Bit `15` | D-pad Right |
| `OK` | `16` | `STATUS1` | Bit `0` | D-pad Center / Select |
| `LSK` | `17` | `STATUS1` | Bit `1` | Left Softkey |
| `RSK` | `18` | `STATUS1` | Bit `2` | Right Softkey |
| `CALL` | `19` | `STATUS1` | Bit `3` | Green Call key |
| `END` / `POWER` | `20` | `STATUS1` | Bit `4` | Red End / Power key |
| `SOS` | `21` | `STATUS1` | Bit `5` | Rear Emergency Button |
| `VOL_UP` | `22` | `STATUS1` | Bit `6` | Side Volume Up |
| `VOL_DOWN` | `23` | `STATUS1` | Bit `7` | Side Volume Down |

### Hall Effect Flip Sensor
- **Interface**: GPIO Bank 0, Pin 0 (`SPRD_GPIO_BASE + 0x0000`).
- **Signal Logic**:
  - `High (1)`: Flip Open (main screen active, keypad enabled).
  - `Low (0)`: Flip Closed (clamshell shut, outer sub-display active).
- **Interrupts**: Configured via SPRD GPIO Bank 0 interrupt sense and event registers to trigger wake-on-open and sleep-on-close events in MOCOR OS.

