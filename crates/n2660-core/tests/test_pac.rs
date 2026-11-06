// SPDX-License-Identifier: MIT
//! Integration & unit tests for Spreadtrum .PAC firmware container in Rust

use n2660_core::pac::PacArchive;
use n2660_core::bootrom::SprdHeader;
use n2660_core::vm::T107Vm;
use std::fs;

#[test]
fn test_pac_pack_and_parse_header() {
    let mut pac = PacArchive::new("Nokia 2660 Flip", "TA-1480_v20.00");
    pac.add_file("FDL1", "fdl1.bin", 0x0000_6200, vec![1, 2, 3, 4]);
    pac.add_file("BOOT0", "boot0.bin", 0x0000_6200, vec![5, 6, 7, 8]);

    let packed = pac.pack();
    let parsed = PacArchive::parse(&packed).expect("Failed to parse PAC");

    assert_eq!(parsed.product_name, "Nokia 2660 Flip");
    assert_eq!(parsed.product_version, "TA-1480_v20.00");
    assert_eq!(parsed.files.len(), 2);
    assert_eq!(parsed.files[0].id, "FDL1");
    assert_eq!(parsed.files[0].data, vec![1, 2, 3, 4]);
    assert_eq!(parsed.files[1].id, "BOOT0");
    assert_eq!(parsed.files[1].data, vec![5, 6, 7, 8]);
}

#[test]
fn test_pac_extract_all_and_single() {
    let mut pac = PacArchive::new("Nokia 2660 Flip", "TA-1480");
    pac.add_file("FDL1", "fdl1.bin", 0x0000_6200, vec![10, 20, 30, 40]);
    pac.add_file("PARTITION_XML", "partition.xml", 0, b"<Partitions/>".to_vec());

    let td = std::env::temp_dir().join("test_pac_extract");
    let _ = fs::remove_dir_all(&td);

    let count = pac.extract_all(&td).expect("Extract all failed");
    assert_eq!(count, 2);

    let fdl1_content = fs::read(td.join("fdl1.bin")).expect("Read fdl1 failed");
    assert_eq!(fdl1_content, vec![10, 20, 30, 40]);

    let xml = pac.dump_xml();
    assert_eq!(xml, "<Partitions/>");

    let _ = fs::remove_dir_all(td);
}

#[test]
fn test_pac_firmware_staging_into_vm() {
    let mut pac = PacArchive::new("Nokia 2660 Flip", "TA-1480_v20.00");

    // 1. FDL1 partition (staged into SRAM 0x0000_6200)
    let fdl1_bytes = vec![0x11, 0x22, 0x33, 0x44];
    pac.add_file("FDL1", "fdl1.bin", 0x0000_6200, fdl1_bytes.clone());

    // 2. BOOT0 partition with valid SPRD header
    let boot0_payload = vec![0xAA, 0xBB, 0xCC, 0xDD, 0x00, 0x00, 0x00, 0xEA];
    let boot0_img = SprdHeader::create(&boot0_payload, 0x0000_6200, 0x0000_6200);
    pac.add_file("BOOT0", "boot0.bin", 0x0000_6200, boot0_img.clone());

    // 3. BOOT / Kernel partition (staged into DRAM 0x8000_0000)
    let kernel_bytes = vec![0x01, 0x00, 0xA0, 0xE3, 0xFE, 0xFF, 0xFF, 0xEA]; // mov r0, #1; b .
    pac.add_file("BOOT", "boot.img", 0x8000_0000, kernel_bytes.clone());

    // 4. SYSTEM partition (staged into SPI NAND flash storage)
    let system_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
    pac.add_file("SYSTEM", "system.img", 0, system_bytes.clone());

    // Pack into raw PAC container bytes and parse back
    let pac_bytes = pac.pack();
    let parsed_pac = PacArchive::parse(&pac_bytes).expect("Failed to parse vendor PAC");

    // Initialize VM
    let mut vm = T107Vm::new(true, false, false).expect("VM create failed");

    // Run PAC staging
    let entry_opt = vm.load_pac(&parsed_pac, 0x0000_6200, 0x8010_0000, 0x8000_0000)
        .expect("PAC staging failed");

    assert!(entry_opt.is_some());

    // Verify Kernel in DRAM
    let read_kernel = vm.read_bytes(0x8000_0000, 8).expect("Read DRAM failed");
    assert_eq!(read_kernel, kernel_bytes);

    // Verify BOOT0 in Flash Direct Window
    let read_flash = vm.read_bytes(0x6000_0000, boot0_img.len()).expect("Read Flash failed");
    assert_eq!(read_flash, boot0_img);

    // Verify SYSTEM in SPI NAND storage
    let sfc_data = &vm.uc.get_data().sfc.storage[..system_bytes.len()];
    assert_eq!(sfc_data, &system_bytes[..]);

    // Verify execution of staged Kernel
    vm.init_cpu(0x8000_0000).expect("Init CPU failed");
    assert_eq!(vm.get_pc(), 0x8000_0000);
    assert_eq!(vm.get_sp(), 0x8003_FF00); // DRAM stack pointer

    let step_res = vm.step(1, 100_000);
    assert!(step_res.is_ok());
    assert_eq!(vm.uc.reg_read(unicorn_engine::RegisterARM::R0).unwrap(), 1);
}
