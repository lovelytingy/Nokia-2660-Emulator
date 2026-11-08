// SPDX-License-Identifier: MIT
//! Integration test for booting physical Nokia 2660 device dump

use n2660_core::vm::T107Vm;
use std::path::Path;

#[test]
fn test_device_dump_boot0_and_boot1() {
    let dump_dir = if Path::new("device-dump").exists() {
        Path::new("device-dump")
    } else if Path::new("../../device-dump").exists() {
        Path::new("../../device-dump")
    } else {
        return;
    };

    let mut vm = T107Vm::new(true, true, false).expect("Failed to create VM");
    let entry = vm.load_dump(dump_dir, 0x0000_6400).expect("Failed to load device dump");
    println!("Loaded dump entry: {:?}", entry);
    assert_eq!(entry, Some(0x0000_6400));

    vm.set_entry_point(entry.unwrap()).expect("Set entry point");
    for _ in 0..350 {
        let _ = vm.step(50_000, 25_000);
    }
    println!("Total instructions: {}", vm.get_instruction_count());
    println!("Final PC: 0x{:08X}", vm.get_pc());
    let fb = vm.get_gc9307_framebuffer();
    println!("fb[0][0] = 0x{:04X}", fb[0][0]);
    println!("fb[37][40] = 0x{:04X}", fb[37][40]);
    println!("fb[100][100] = 0x{:04X}", fb[100][100]);
    println!("fb[200][100] = 0x{:04X}", fb[200][100]);
}
