// SPDX-License-Identifier: MIT
//! Unit tests for memory map constants, peripherals, and register boundaries in Rust

use n2660_core::types::*;
use n2660_core::vm::T107Vm;

#[test]
fn test_sram_and_dram_boundaries() {
    assert_eq!(SRAM_BASE, 0x0000_0000);
    assert_eq!(SRAM_SIZE, 256 * 1024);
    assert_eq!(SRAM_BASE + SRAM_SIZE as u64, 0x0004_0000);

    assert_eq!(DRAM_BASE, 0x8000_0000);
    assert_eq!(DRAM_SIZE, 48 * 1024 * 1024);
    assert_eq!(DRAM_BASE + DRAM_SIZE as u64, 0x8300_0000);
}

#[test]
fn test_peripheral_base_addresses() {
    assert_eq!(SPRD_UART0_BASE, 0x5000_0000);
    assert_eq!(SPRD_TIMER_BASE, 0x5003_0000);
    assert_eq!(SPRD_GPIO_BASE,  0x5004_0000);
    assert_eq!(SPRD_INTC_BASE,  0x5005_0000);
    assert_eq!(SPRD_KPD_BASE,   0x5006_0000);
    assert_eq!(SPRD_SPI0_BASE,  0x5008_0000);
    assert_eq!(SPRD_SPI1_BASE,  0x5009_0000);
    assert_eq!(SPRD_SFC_BASE,   0x500A_0000);
    assert_eq!(SPRD_AUDIO_BASE, 0x500C_0000);
    assert_eq!(FLASH_DIRECT_BASE, 0x6000_0000);
}

#[test]
fn test_display_resolutions() {
    assert_eq!(GC9307_WIDTH, 240);
    assert_eq!(GC9307_HEIGHT, 320);
    assert_eq!(ST7735_WIDTH, 120);
    assert_eq!(ST7735_HEIGHT, 160);
}

#[test]
fn test_vm_memory_access() {
    let mut vm = T107Vm::new(true, false, false).expect("VM create failed");
    // Test write and read back in SRAM
    let sram_test = vec![0xDE, 0xAD, 0xBE, 0xEF];
    vm.load_bytes(&sram_test, 0x0000_1000).expect("SRAM write failed");
    let mut read_buf = vec![0u8; 4];
    vm.uc.mem_read(0x0000_1000, &mut read_buf).expect("SRAM read failed");
    assert_eq!(read_buf, sram_test);

    // Test write and read back in DRAM
    let dram_test = vec![0xCA, 0xFE, 0xBA, 0xBE];
    vm.load_bytes(&dram_test, 0x8002_0000).expect("DRAM write failed");
    vm.uc.mem_read(0x8002_0000, &mut read_buf).expect("DRAM read failed");
    assert_eq!(read_buf, dram_test);
}
