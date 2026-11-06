// SPDX-License-Identifier: MIT
//! Integration & unit tests for ARM Boot Tags (ATAGS) in Rust

use n2660_core::atags::*;
use n2660_core::vm::T107Vm;
use unicorn_engine::RegisterARM;

#[test]
fn test_builder_individual_tags() {
    let builder = AtagsBuilder::new()
        .with_core(0x12, 8192, 0x0102)
        .with_ram(0x8000_0000, 32 * 1024 * 1024)
        .with_revision(2)
        .with_serial(0x1122_3344, 0x5566_7788)
        .with_initrd2(0x8080_0000, 0x0020_0000);

    let raw = builder.build();
    assert_eq!(raw.len() % 4, 0);

    let tags = AtagsParser::parse(&raw, 0x8000_0100).expect("Failed to parse ATAGS");
    let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec!["ATAG_CORE", "ATAG_MEM", "ATAG_REVISION", "ATAG_SERIAL", "ATAG_INITRD2", "ATAG_CMDLINE", "ATAG_NONE"]);
}

#[test]
fn test_cmdline_padding_and_alignment() {
    let builder = AtagsBuilder::new()
        .with_cmdline(Some("test=1".to_string()));
    let raw = builder.build();
    assert_eq!(raw.len() % 4, 0);

    let tags = AtagsParser::parse(&raw, 0x8000_0100).expect("Failed to parse ATAGS");
    let cmd_tag = tags.iter().find(|t| t.name == "ATAG_CMDLINE").expect("Missing ATAG_CMDLINE");
    assert!(cmd_tag.details.contains("test=1"));
}

#[test]
fn test_vm_staging_and_register_setup() {
    let mut vm = T107Vm::new(true, false, false).expect("Failed to create VM");
    let staged_addr = 0x8003_0000u64;
    let test_cmd = "console=ttyS0,115200 root=/dev/mtdblock3 earlycon".to_string();

    vm.setup_atags(staged_addr, 0x9117, Some(test_cmd.clone())).expect("Failed to setup ATAGS");

    // Verify ARM kernel handoff registers: r0=0, r1=0x9117, r2=staged_addr
    assert_eq!(vm.uc.reg_read(RegisterARM::R0).unwrap(), 0);
    assert_eq!(vm.uc.reg_read(RegisterARM::R1).unwrap(), 0x9117);
    assert_eq!(vm.uc.reg_read(RegisterARM::R2).unwrap(), staged_addr);

    // Read back staged memory from Unicorn VM DRAM
    let mut mem_data = vec![0u8; 128];
    vm.uc.mem_read(staged_addr, &mut mem_data).expect("Failed to read DRAM");
    let tags = AtagsParser::parse(&mem_data, staged_addr as u32).expect("Failed to parse DRAM ATAGS");
    assert_eq!(tags[0].name, "ATAG_CORE");
    assert_eq!(tags[1].name, "ATAG_MEM");
    let cmd_tag = tags.iter().find(|t| t.name == "ATAG_CMDLINE").expect("Missing ATAG_CMDLINE");
    assert!(cmd_tag.details.contains(&test_cmd));
}
