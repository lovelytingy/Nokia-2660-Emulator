// SPDX-License-Identifier: MIT
//! Constant memory map addresses, display dimensions, and hardware identifiers for Nokia 2660 / Unisoc T107.

pub const SRAM_BASE: u64 = 0x0000_0000;
pub const SRAM_SIZE: usize = 256 * 1024; // 256 KB On-Chip SYSRAM/IRAM

pub const DRAM_BASE: u64 = 0x8000_0000;
pub const DRAM_SIZE: usize = 48 * 1024 * 1024; // 48 MB

pub const DRAM_EXT_BASE: u64 = 0x8300_0000;
pub const DRAM_EXT_SIZE: usize = 16 * 1024 * 1024; // 16 MB extended (completing full 64 MB DRAM window)

pub const USER_DRAM_BASE: u64 = 0x8800_0000;
pub const USER_DRAM_SIZE: usize = 32 * 1024 * 1024; // 32 MB userland application DRAM window

pub const SYS_HEAP_BASE: u64 = 0x7200_0000;
pub const SYS_HEAP_SIZE: usize = 224 * 1024 * 1024; // 224 MB ThreadX Task Stack & Dynamic Heap Window (0x7200_0000..0x8000_0000)

pub const FLASH_DIRECT_BASE: u64 = 0x6000_0000;
pub const FLASH_DIRECT_SIZE: usize = 128 * 1024 * 1024; // 128 MB

pub const MMIO_BASE: u64 = 0x5000_0000;
pub const MMIO_SIZE: usize = 16 * 1024 * 1024; // 16 MB Spreadtrum MMIO Window (0x5000_0000..0x5100_0000)

pub const PL011_UART_BASE: u64 = 0x0900_0000;
pub const PL011_UART_SIZE: usize = 0x1000; // 4 KB

pub const SPRD_AHB_SYS_BASE: u64 = 0x2000_0000;
pub const SPRD_AHB_SYS_SIZE: usize = 32 * 1024 * 1024; // 32 MB AHB, LCDC, HW SFC and System Registers (0x2000_0000..0x2200_0000)

pub const SPRD_GIC_BASE: u64 = 0x1200_0000;
pub const SPRD_GIC_SIZE: usize = 64 * 1024; // 64 KB ARM Cortex-A7 MPCore / GIC-400 (Distributor & CPU Interface)

pub const TRAP_BASE: u64 = 0x4000_0000;
pub const TRAP_SIZE: usize = 16 * 1024 * 1024; // 16 MB AON Peripheral & Control Window (0x4000_0000..0x4100_0000)

pub const SPRD_DMC_BASE: u64 = 0x3000_0000;
pub const SPRD_DMC_SIZE: usize = 64 * 1024 * 1024; // 64 MB DDR Controller, PHY & Co-Processor Window (0x3000_0000..0x3400_0000)

pub const SPRD_APB_PERIPH_BASE: u64 = 0x7000_0000;
pub const SPRD_APB_PERIPH_SIZE: usize = 32 * 1024 * 1024; // 32 MB APB Peripherals (0x7000_0000..0x7200_0000)

pub const HIGH_VECTORS_BASE: u64 = 0xFFFF_0000;
pub const HIGH_VECTORS_SIZE: usize = 64 * 1024; // 64 KB ARM High Exception Vectors Window

pub const SPRD_DEBUG_UART_BASE: u64 = 0x7010_0000;
pub const SPRD_DEBUG_UART_SIZE: usize = 0x0000_1000; // 4 KB Boot Debug UART

pub const SPRD_SYSTIMER_BASE: u64 = 0x4023_0000;
pub const SPRD_SYSTIMER_SIZE: usize = 0x0000_1000; // 4 KB AON System Timer (BOOT0/SPL)

pub const SPRD_KERNEL_SYSTIMER_BASE: u64 = 0x4004_0000;
pub const SPRD_KERNEL_SYSTIMER_SIZE: usize = 0x0000_1000; // 4 KB Kernel Hardware Systimer (MOCOR RTOS)

pub const SPRD_HW_SFC_BASE: u64 = 0x21A0_0000;
pub const SPRD_HW_SFC_SIZE: usize = 0x0001_0000; // 64 KB Hardware Serial Flash Controller (SFC)

pub const SPRD_LCDC_BASE: u64 = 0x2080_0000;
pub const SPRD_LCDC_SIZE: usize = 0x0000_2000; // 8 KB Display Controller

pub const SPRD_USB_BASE: u64 = 0x2020_0000;
pub const SPRD_USB_SIZE: usize = 0x0000_2000; // 8 KB USB 2.0 OTG Controller

pub const SPRD_LCM_BASE: u64 = 0x20A0_0000;
pub const SPRD_LCM_SIZE: usize = 0x0000_1000; // 4 KB LCM Controller

pub const SPRD_APB_SPI1_BASE: u64 = 0x70B0_0000;
pub const SPRD_APB_SPI1_SIZE: usize = 0x0000_1000; // 4 KB SPI1 Controller

pub const SPRD_AHB_BASE: u64 = 0x20E0_0000;
pub const SPRD_AHB_SIZE: usize = 0x0000_4000; // 16 KB AHB Clocks & Power

pub const SPRD_KEYPAD_BASE: u64 = 0x4025_0000;
pub const SPRD_KEYPAD_SIZE: usize = 0x0000_1000; // 4 KB Keypad Matrix

pub const SPRD_AON_PINMUX_BASE: u64 = 0x402A_0000;
pub const SPRD_AON_PINMUX_SIZE: usize = 0x0000_2000; // 8 KB AON Pinmux

pub const SPRD_SECURE_BOOT_BASE: u64 = 0x402E_0000;
pub const SPRD_SECURE_BOOT_SIZE: usize = 0x0000_4000; // 16 KB Security & System Control

pub const SPRD_PMIC_ADI_BASE: u64 = 0x4060_0000;
pub const SPRD_PMIC_ADI_SIZE: usize = 0x0000_2000; // 8 KB PMIC ADI Base

pub const SPRD_PMIC_EIC_BASE: u64 = 0x4060_8000;
pub const SPRD_PMIC_EIC_SIZE: usize = 0x0000_4000; // 16 KB PMIC EIC & APB

pub const SPRD_APB_CLK_BASE: u64 = 0x7130_0000;
pub const SPRD_APB_CLK_SIZE: usize = 0x0000_1000; // 4 KB APB Clock Gates

pub const SPRD_ADI_CTRL_BASE: u64 = 0xA160_0000;
pub const SPRD_ADI_CTRL_SIZE: usize = 0x0000_2000; // 8 KB ADI Controller

pub const SPRD_ADI_ANALOG_BASE: u64 = 0xA161_0000;
pub const SPRD_ADI_ANALOG_SIZE: usize = 0x0000_2000; // 8 KB Analog Die Window

pub const SPRD_MODEM_PERIPH_BASE: u64 = 0xE000_0000;
pub const SPRD_MODEM_PERIPH_SIZE: usize = 16 * 1024 * 1024; // 16 MB Unisoc Modem APB Window (GPTA, RF, CoreSight)

pub const SPRD_MODEM_SHMEM_BASE: u64 = 0xD100_0000;
pub const SPRD_MODEM_SHMEM_SIZE: usize = 16 * 1024 * 1024; // 16 MB Unisoc Modem / DSP Shared Memory & Mailbox

pub const SPRD_MODEM_GPTA_BASE: u64 = 0xE002_0000;
pub const SPRD_MODEM_GPTA_SIZE: usize = 0x1000; // 4 KB Modem GPTA / Free Running Counter (FRC)

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

/// Nokia 2660 / MOCOR Multi-Stage Boot Sequence Stages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    Boot0,
    Boot1,
    Kernel,
    User,
}

impl BootStage {
    pub fn name(&self) -> &'static str {
        match self {
            BootStage::Boot0 => "BOOT0 (SPL)",
            BootStage::Boot1 => "BOOT1 (Miniloader)",
            BootStage::Kernel => "Kernel (MOCOR RTOS)",
            BootStage::User => "Userland (MS_MMI_Main)",
        }
    }

    pub fn progress_pct(&self) -> u32 {
        match self {
            BootStage::Boot0 => 25,
            BootStage::Boot1 => 50,
            BootStage::Kernel => 75,
            BootStage::User => 100,
        }
    }
}
