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

## 4. Display Controller & Panel (GalaxyCore GC9307)

The Nokia 2660 Flip primary screen is a 2.8-inch QVGA (240x320) TFT LCD driven by a GalaxyCore GC9307 controller over a 4-line SPI or 8-bit MCU 8080 parallel bus.

- **SPI0 Base Address**: `0x50080000`
- **Resolution**: 240 x 320 pixels
- **Color Depth**: RGB565 (16 bits per pixel)
- **Outer Secondary Display**: 1.77-inch QQVGA (120x160 pixels)
