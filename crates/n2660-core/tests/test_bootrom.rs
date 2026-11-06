// SPDX-License-Identifier: MIT
//! Integration & unit tests for BootROM & SPRD BOOT0 loader in Rust

use n2660_core::bootrom::*;
use n2660_core::vm::T107Vm;
use std::fs;

#[test]
fn test_mksprd_img_header_creation_and_verification() {
    let test_payload = vec![0x2A, 0x00, 0xA0, 0xE3, 0xFE, 0xFF, 0xFF, 0xEA];
    let image = SprdHeader::create(&test_payload, 0x0000_6200, 0x0000_6200);
    assert_eq!(image.len(), 32 + test_payload.len());

    let hdr = SprdHeader::parse(&image).expect("Failed to parse SPRD header");
    assert_eq!(hdr.magic, SPRD_MAGIC);
    assert_eq!(hdr.load_addr, 0x0000_6200);
    assert_eq!(hdr.entry_point, 0x0000_6200);
    assert_eq!(hdr.payload_size as usize, test_payload.len());

    // Checksum verification
    let csum = SprdHeader::calculate_checksum(&test_payload);
    assert_eq!(hdr.checksum, csum);
}

#[test]
fn test_mksprd_img_flash_inject() {
    let test_payload = vec![0x2A, 0x00, 0xA0, 0xE3, 0xFE, 0xFF, 0xFF, 0xEA];
    let image = SprdHeader::create(&test_payload, 0x0000_6200, 0x0000_6200);

    let temp_flash = std::env::temp_dir().join("test_flash_boot0.bin");
    SprdHeader::inject_into_flash(&image, &temp_flash, 0, 128 * 1024 * 1024)
        .expect("Failed to inject BOOT0 into flash");

    let read_back = fs::read(&temp_flash).expect("Failed to read back flash");
    assert_eq!(&read_back[..image.len()], &image[..]);
    let _ = fs::remove_file(temp_flash);
}

#[test]
fn test_bootrom_normal_boot_staging() {
    let test_payload = vec![0x2A, 0x00, 0xA0, 0xE3, 0xFE, 0xFF, 0xFF, 0xEA];
    let image = SprdHeader::create(&test_payload, 0x0000_6200, 0x0000_6200);

    let mut vm = T107Vm::new(true, false, false).expect("Failed to create VM");
    vm.load_bytes(&image, 0x6000_0000).expect("Failed to stage to XIP flash");

    // Load payload into SRAM 0x00006200
    let hdr = SprdHeader::parse(&image).expect("Valid SPRD");
    vm.load_bytes(&image[32..], hdr.load_addr as u64).expect("Failed to load SRAM");

    let mut sram_staged = vec![0u8; test_payload.len()];
    vm.uc.mem_read(0x0000_6200, &mut sram_staged).expect("Failed to read SRAM");
    assert_eq!(sram_staged, test_payload);
}
