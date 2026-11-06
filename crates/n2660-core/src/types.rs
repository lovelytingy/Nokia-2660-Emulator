// SPDX-License-Identifier: MIT
//! Constant memory map addresses, display dimensions, and hardware identifiers for Nokia 2660 / Unisoc T107.

pub const SRAM_BASE: u64 = 0x0000_0000;
pub const SRAM_SIZE: usize = 256 * 1024; // 256 KB On-Chip SYSRAM/IRAM

pub const DRAM_BASE: u64 = 0x8000_0000;
pub const DRAM_SIZE: usize = 48 * 1024 * 1024; // 48 MB

pub const FLASH_DIRECT_BASE: u64 = 0x6000_0000;
pub const FLASH_DIRECT_SIZE: usize = 128 * 1024 * 1024; // 128 MB

pub const MMIO_BASE: u64 = 0x5000_0000;
pub const MMIO_SIZE: usize = 0x0010_0000; // 1 MB

pub const PL011_UART_BASE: u64 = 0x0900_0000;
pub const PL011_UART_SIZE: usize = 0x1000; // 4 KB

pub const TRAP_BASE: u64 = 0x4000_0000;
pub const TRAP_SIZE: usize = 0x0001_0000; // 64 KB

pub const SPRD_DMC_BASE: u64 = 0x3000_0000;
pub const SPRD_DMC_SIZE: usize = 0x0001_0000; // 64 KB DDR Controller & PHY

pub const SPRD_DEBUG_UART_BASE: u64 = 0x7010_0000;
pub const SPRD_DEBUG_UART_SIZE: usize = 0x0000_1000; // 4 KB Boot Debug UART

pub const SPRD_SYSTIMER_BASE: u64 = 0x4023_0000;
pub const SPRD_SYSTIMER_SIZE: usize = 0x0000_1000; // 4 KB AON System Timer

pub const SPRD_HW_SFC_BASE: u64 = 0x21A0_0000;
pub const SPRD_HW_SFC_SIZE: usize = 0x0001_0000; // 64 KB Hardware Serial Flash Controller (SFC)

// Peripheral MMIO Offsets
pub const SPRD_UART0_BASE: u32 = 0x5000_0000;
pub const SPRD_UART1_BASE: u32 = 0x5001_0000;
pub const SPRD_UART2_BASE: u32 = 0x5002_0000;
pub const SPRD_TIMER_BASE: u32 = 0x5003_0000;
pub const SPRD_GPIO_BASE: u32  = 0x5004_0000;
pub const SPRD_INTC_BASE: u32  = 0x5005_0000;
pub const SPRD_KPD_BASE: u32   = 0x5006_0000;
pub const SPRD_EIC_BASE: u32   = 0x5007_0000;
pub const SPRD_SPI0_BASE: u32  = 0x5008_0000;
pub const SPRD_SPI1_BASE: u32  = 0x5009_0000;
pub const SPRD_SFC_BASE: u32   = 0x500A_0000;
pub const SPRD_I2C_BASE: u32   = 0x500B_0000;
pub const SPRD_AUDIO_BASE: u32 = 0x500C_0000;
pub const SPRD_ADI_BASE: u32   = 0x500D_0000;
pub const SPRD_CLK_BASE: u32   = 0x500E_0000;
pub const SPRD_PIN_BASE: u32   = 0x500F_0000;

// Displays
pub const GC9307_WIDTH: usize = 240;
pub const GC9307_HEIGHT: usize = 320;
pub const ST7735_WIDTH: usize = 120;
pub const ST7735_HEIGHT: usize = 160;

// IRQ Channels
pub const SPRD_IRQ_UART0: u32 = 0;
pub const SPRD_IRQ_TIMER: u32 = 4;
pub const SPRD_IRQ_KPD: u32 = 5;
pub const SPRD_IRQ_GPIO: u32 = 6;
pub const SPRD_IRQ_SPI0: u32 = 8;
pub const SPRD_IRQ_SPI1: u32 = 9;

// Nokia 2660 Key Codes
pub const NOKIA_KEYS: &[&str] = &[
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
    "*", "#", "call", "end", "lsk", "rsk",
    "up", "down", "left", "right", "ok",
    "vol_up", "vol_down", "sos",
];
