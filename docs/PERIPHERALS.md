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

---

## 6. Serial Flash Controller (SFC) & 128 MB SPI NAND Storage

The Nokia 2660 Flip TA-1480 utilizes a 128 MB (1 Gb) high-speed SPI NAND flash (Winbond W25N01GV or GigaDevice GD5F1GQ4) for all persistent system and user storage partitions.

### Hardware Architecture
- **SFC Base Address**: `0x500A0000`
- **Direct Flash Read Window (XIP)**: `0x60000000` - `0x67FFFFFF` (up to 128 MB mapped space)
- **Flash Geometry**:
  - **Total Capacity**: 128 MB (1,048,576 bytes * 128)
  - **Block Organization**: 1024 Blocks of 128 KB each
  - **Page Organization**: 64 Pages per block (2048 bytes user data + 64 bytes spare / OOB ECC per page)
  - **Internal Page Cache Buffer**: 2112 bytes

### Register Map (`SPRD_SFC_BASE 0x500A0000`)
| Offset | Register Name | Description | Access | Reset |
| :--- | :--- | :--- | :--- | :--- |
| `+0x0000` | `SFC_CMD` | Command Opcode (executes command on write) | R/W | `0x00000000` |
| `+0x0004` | `SFC_ADDR` | Address Register (Page / Block address or Column offset) | R/W | `0x00000000` |
| `+0x0008` | `SFC_DATA` | Data Register / Buffer FIFO (32-bit streaming word) | R/W | `0x00000000` |
| `+0x000C` | `SFC_CTL` | Transfer Control Register | R/W | `0x00000000` |
| `+0x0010` | `SFC_STS` | Status Register (Bit 0: Busy, Bit 1: Done/Ready) | R | `0x00000002` |
| `+0x0014` | `SFC_BUF_PTR` | Internal 2112-byte Cache Buffer Pointer (0–2111) | R/W | `0x00000000` |

### Supported SPI NAND Command Opcodes
| Opcode | Command Name | Description |
| :--- | :--- | :--- |
| `0xFF` | `NAND_CMD_RESET` | Software Reset (resets buffer and clears Write Enable Latch) |
| `0x9F` | `NAND_CMD_READ_ID` | JEDEC ID Read (returns Winbond `0xEF 0xAA 0x21` or GD `0xC8 0xB1 0x48`) |
| `0x13` | `NAND_CMD_PAGE_READ` | Page Data Read (loads 2048+64 bytes from NAND flash page into buffer) |
| `0x03` | `NAND_CMD_READ_CACHE` | Read Buffer (streams bytes out of internal page cache starting at column address) |
| `0x0B` | `NAND_CMD_READ_FAST` | High-speed Read Buffer |
| `0x06` | `NAND_CMD_WRITE_ENABLE` | Sets Write Enable Latch (WEL) in Status Register 3 |
| `0x04` | `NAND_CMD_WRITE_DISABLE`| Clears Write Enable Latch |
| `0x02` | `NAND_CMD_PROGRAM_LOAD` | Loads incoming data stream into internal page buffer |
| `0x10` | `NAND_CMD_PROGRAM_EXEC` | Writes internal buffer contents into specified NAND flash array page |
| `0xD8` | `NAND_CMD_BLOCK_ERASE` | Erases entire 128 KB block (resets all bytes in block to `0xFF`) |
| `0x0F` | `NAND_CMD_GET_FEATURE` | Reads Status Registers (Protection, Configuration, Status) |
| `0x1F` | `NAND_CMD_SET_FEATURE` | Writes Status Registers |

---

## 7. SPRD Audio Subsystem & Tone Generator

- **Base Address**: `0x500C0000`
- **Architecture**: Integrated stereo DAC, 64-word PCM FIFO, hardware volume control, output routing matrix, and hardware tone/buzzer frequency synthesizer.
- **Audio Output Paths**:
  - `1`: Earpiece Receiver
  - `2`: Loudspeaker / Handsfree
  - `3`: 3.5 mm Stereo Headphone Jack

### Register Map
| Offset | Register Name | Description | Access | Reset |
| :--- | :--- | :--- | :--- | :--- |
| `+0x0000` | `AUD_CTL` | Audio Control (Bit 0: Enable, Bit 1: TX Enable, Bit 8: Stereo, Bit 10: 16/24-bit) | R/W | `0x00000000` |
| `+0x0004` | `AUD_STS` | Status (Bit 0: TX Empty, Bit 1: TX Full, Bits 8–15: TX FIFO Count) | R | `0x00000005` |
| `+0x0008` | `AUD_TXD` | Audio TX PCM FIFO Data Register (Write 16/32-bit audio sample) | W | `0x00000000` |
| `+0x000C` | `AUD_RXD` | Audio RX PCM FIFO Data Register (Read microphone sample) | R | `0x00000000` |
| `+0x0010` | `AUD_INT_EN` | Interrupt Enable Register | R/W | `0x00000000` |
| `+0x0014` | `AUD_INT_RAW` | Raw Interrupt Status Register | R | `0x00000000` |
| `+0x0018` | `AUD_INT_CLR` | Interrupt Clear Register | W | `0x00000000` |
| `+0x001C` | `AUD_FIFO_CTL`| FIFO Control & Watermark (Bit 0: Reset TX FIFO, Bits 8–15: Threshold) | R/W | `0x00101000` |
| `+0x0020` | `AUD_DAC_CTL` | DAC Power & Volume Control (Bit 0: DAC Enable, Bits 8–15: Master Volume) | R/W | `0x00008001` |
| `+0x0024` | `AUD_PATH_CTL`| Audio Output Path Selector (1=Earpiece, 2=Speaker, 3=Headphone) | R/W | `0x00000002` |
| `+0x0030` | `TONE_CTL` | Tone Generator Control (Bit 0: Enable, Bit 1: Continuous, Bit 2: Sine/Square) | R/W | `0x00000000` |
| `+0x0034` | `TONE_FREQ` | Tone Frequency in Hz (e.g. 1000 for 1000 Hz key click, 440 for A4) | R/W | `0x000003E8` |
| `+0x0038` | `TONE_DUR` | Tone Duration in milliseconds (e.g. 50 ms for short click) | R/W | `0x00000032` |
| `+0x003C` | `TONE_VOL` | Tone Volume / Amplitude (0–100%) | R/W | `0x00000050` |

---

## 8. SPRD Interrupt Controller (INTC)

- **Base Address**: `0x50050000`
- **Interrupt Lines**: 32 prioritized hardware and software lines
- **Core Connections**: Connected to ARM Cortex-A7 `nIRQ` and `nFIQ` exception inputs
- **Modes**: Programmable line routing between standard IRQ and high-priority FIQ via `INTC_TYPE`

### Register Map
| Offset | Register Name | Description | Access | Reset |
| :--- | :--- | :--- | :--- | :--- |
| `+0x0000` | `INTC_IRQ_RAW_STS` | Raw Interrupt Status (All active peripheral + soft lines) | R | `0x00000000` |
| `+0x0004` | `INTC_IRQ_MASK_STS` | Masked Interrupt Status (`RAW & EN & ~TYPE`) | R | `0x00000000` |
| `+0x0008` | `INTC_IRQ_EN` | Interrupt Enable / Set Register | R/W | `0x00000000` |
| `+0x000C` | `INTC_IRQ_EN_CLR` | Interrupt Enable Clear Register (Write 1 to clear bit) | W | `0x00000000` |
| `+0x0010` | `INTC_IRQ_SOFT` | Software Interrupt Trigger (Write 1 to assert) | R/W | `0x00000000` |
| `+0x0014` | `INTC_IRQ_SOFT_CLR` | Software Interrupt Clear (Write 1 to deassert) | W | `0x00000000` |
| `+0x0018` | `INTC_IRQ_CLR` | Hardware Edge Interrupt Clear (Write 1 to acknowledge) | W | `0x00000000` |
| `+0x001C` | `INTC_FIQ_RAW_STS` | FIQ Raw Interrupt Status | R | `0x00000000` |
| `+0x0020` | `INTC_FIQ_MASK_STS` | FIQ Masked Interrupt Status (`RAW & FIQ_EN & TYPE`) | R | `0x00000000` |
| `+0x0024` | `INTC_FIQ_EN` | FIQ Enable Register | R/W | `0x00000000` |
| `+0x0028` | `INTC_FIQ_EN_CLR` | FIQ Enable Clear Register | W | `0x00000000` |
| `+0x002C` | `INTC_FIQ_SOFT` | FIQ Software Interrupt Trigger | R/W | `0x00000000` |
| `+0x0030` | `INTC_FIQ_SOFT_CLR` | FIQ Software Interrupt Clear | W | `0x00000000` |
| `+0x0034` | `INTC_TYPE` | Routing Selector (`0` = IRQ, `1` = FIQ for bits 0–31) | R/W | `0x00000000` |

### Hardware IRQ Channel Assignments
| IRQ Line | Source Peripheral | Trigger Condition |
| :---: | :--- | :--- |
| **0** | SPRD System Timer | Counter underflow / tick period match |
| **1** | SPRD UART0 | RX FIFO watermark or TX buffer empty (Console / BSL) |
| **2** | SPRD UART1 | Secondary serial interface events |
| **3** | SPRD UART2 | Modem / Bluetooth events |
| **4** | SPRD GPIO | Bank 0 Pin 0 flip sensor or GPIO edge trigger |
| **5** | SPRD Keypad (KPD) | Matrix key press / release event |
| **6** | SPRD SPI0 | Primary GC9307 LCD TX completion |
| **7** | SPRD SPI1 | Secondary ST7735 Sub-LCD TX completion |
| **8** | SPRD SFC | SPI NAND flash command / page transfer done |
| **9** | SPRD Audio | Audio DAC FIFO underrun or tone completion |
| **10** | Watchdog Timer | Watchdog pre-timeout interrupt |
| **11** | DMA Controller | Channel transfer complete |
| **15** | Software Test IRQ | User / OS IPC software interrupt |

---

## 9. Unisoc T107 USB 2.0 OTG Controller & Libc Server Protocol

- **Base Address**: `0x20200000` (Size: `0x00002000` / 8 KB)
- **Architecture**: Unisoc USB 2.0 High-Speed OTG controller with hardware DMA endpoints
- **Hardware Endpoints**:
  - **Endpoint 5 (EP5) TX**: `0x20201C80` (Descriptor pointer at `+0x14`, Control at `+0x04`, Status at `+0x08`)
  - **Endpoint 6 (EP6) RX**: `0x20201E80` (Descriptor pointer at `+0x14`, Control at `+0x04`, Status at `+0x08`, Remaining length at `+0x10`)

### Register Map
| Offset | Register Name | Description | Access |
| :--- | :--- | :--- | :--- |
| `+0x1C80` | `USB_EP5_TX_CFG` | EP5 TX Configuration | R/W |
| `+0x1C84` | `USB_EP5_TX_CTRL` | EP5 DMA Trigger (`bit 0` = 1 triggers transmission) | R/W |
| `+0x1C88` | `USB_EP5_TX_STATUS` | EP5 DMA Status (`bit 18` = 1 signals DMA complete) | R/W |
| `+0x1C94` | `USB_EP5_TX_PTR` | Pointer to 8-byte TX DMA descriptor `[src_addr:4, len:2, ctrl:2]` | R/W |
| `+0x1E80` | `USB_EP6_RX_CFG` | EP6 RX Configuration | R/W |
| `+0x1E84` | `USB_EP6_RX_CTRL` | EP6 DMA Trigger (`bit 0` = 1 arms RX) | R/W |
| `+0x1E88` | `USB_EP6_RX_STATUS` | EP6 DMA Status (`bit 18` = 1 signals packet received) | R/W |
| `+0x1E90` | `USB_EP6_RX_LEN` | RX Remaining Bytes `(12800 - len) << 16` | R/W |
| `+0x1E94` | `USB_EP6_RX_PTR` | Pointer to 8-byte RX DMA descriptor `[dest_addr:4, len:2, ctrl:2]` | R/W |

### Host Libc Server Protocol
Used by bare-metal RAM payloads (`keypad_test.bin`, `baremetal_lcd.bin`, `hello_world.bin`, `fpdoom`) to exchange files, standard I/O streams, and runtime arguments with the host workstation over USB.

1. **Checksum Algorithm (`fastchk16`)**:
   Authentic 16-bit one's complement addition algorithm seeded with `0x5A5A`:
   ```rust
   pub fn fastchk16(mut crc: u32, src: &[u8]) -> u16 {
       let mut i = 0;
       while i + 1 < src.len() {
           crc += ((src[i + 1] as u32) << 8) | (src[i] as u32);
           i += 2;
       }
       if i < src.len() { crc += src[i] as u32; }
       crc = (crc >> 16) + (crc & 0xffff);
       crc += crc >> 16;
       (crc & 0xffff) as u16
   }
   ```
2. **Handshake Sequence**:
   - Guest sends 8-byte FDL ACK frame `[0x7E, 0x00, 0x80, 0x00, 0x00, 0xFF, 0x7F, 0x7E]`.
   - Host replies with `HOST_CONNECT (0x00)`.
   - Host services post-connect drain loop and transitions to active libc server mode.
3. **Command Protocol**:
   - `CMD_MESSAGE (0x80)`: Diagnostic message exchange.
   - `CMD_FOPEN (0x81)`: Open embedded virtual files (`pinmap.bin`, `keymap.bin`).
   - `CMD_FREAD (0x82)`: Read binary slice with 16-bit checksum verification.
   - `CMD_FWRITE (0x83)`: Guest `printf` / `stdout` output streaming to host console.
   - `CMD_FCLOSE (0x84)`: Close file handle.
   - `CMD_FSEEK (0x85)` / `CMD_FTELL (0x86)`: Cursor positioning and length queries.
   - `CMD_GETARGS (0x87)`: Ingest guest command line arguments (`argc` & `argv`).





