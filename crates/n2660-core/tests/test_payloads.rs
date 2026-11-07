// SPDX-License-Identifier: MIT
//! Verification tests running real Unisoc UMS9117 bare-metal payloads from Nokia-2660-RE.

use n2660_core::vm::T107Vm;
use std::fs;
use std::path::Path;

#[test]
fn test_baremetal_asm_payload_mailbox() {
    let payload_path = Path::new("../../Nokia-2660-RE/tools/ram_payload/bin/baremetal_asm.bin");
    if !payload_path.exists() {
        eprintln!("Skipping test: {:?} does not exist", payload_path);
        return;
    }

    let data = fs::read(payload_path).expect("Failed to read baremetal_asm.bin");
    let mut vm = T107Vm::new(true, false, false).expect("VM create failed");

    // Load payload at DRAM 0x80100000, entry 0x80100200
    vm.load_bytes(&data, 0x8010_0000).expect("Load failed");
    vm.set_entry_point(0x8010_0200).expect("Set entry failed");

    // Run 5 million instructions (baremetal_asm finishes loop in ~1.05 million instructions)
    let _ = vm.step(2_500_000, 2_000_000);

    // Read Mailbox diagnostics at 0x80700100
    let mbox_bytes = vm.read_bytes(0x8070_0100, 20).expect("Read mailbox failed");
    let marker1 = u32::from_le_bytes([mbox_bytes[0], mbox_bytes[1], mbox_bytes[2], mbox_bytes[3]]);
    let marker2 = u32::from_le_bytes([mbox_bytes[4], mbox_bytes[5], mbox_bytes[6], mbox_bytes[7]]);
    let marker3 = u32::from_le_bytes([mbox_bytes[8], mbox_bytes[9], mbox_bytes[10], mbox_bytes[11]]);
    let counter = u32::from_le_bytes([mbox_bytes[12], mbox_bytes[13], mbox_bytes[14], mbox_bytes[15]]);
    let status  = u32::from_le_bytes([mbox_bytes[16], mbox_bytes[17], mbox_bytes[18], mbox_bytes[19]]);

    assert_eq!(marker1, 0x14802660, "Marker 1 (Nokia 2660 TA-1480 model ID) mismatch");
    assert_eq!(marker2, 0xCAFEBABE, "Marker 2 (Execution confirmed) mismatch");
    assert_eq!(marker3, 0xDEADBEEF, "Marker 3 (Alive marker) mismatch");
    assert_eq!(counter, 1048575, "Loop counter mismatch");
    assert_eq!(status, 1, "Status flag (SUCCESS) mismatch");
}

#[test]
fn test_baremetal_lcdc_framebuffer_refresh() {
    let payload_path = Path::new("../../Nokia-2660-RE/tools/ram_payload/bin/baremetal_lcd.bin");
    if !payload_path.exists() {
        eprintln!("Skipping test: {:?} does not exist", payload_path);
        return;
    }

    let data = fs::read(payload_path).expect("Failed to read baremetal_lcd.bin");
    let mut vm = T107Vm::new(true, false, false).expect("VM create failed");

    // Load payload at DRAM 0x80100000, entry 0x80100200
    vm.load_bytes(&data, 0x8010_0000).expect("Load failed");
    vm.set_entry_point(0x8010_0200).expect("Set entry failed");

    // Execute first frame render
    let _ = vm.step(500_000, 500_000);

    // Verify LCDC controller refreshed into virtual GC9307 framebuffer
    let fb = vm.get_gc9307_framebuffer();
    let mut non_zero_pixels = 0;
    for row in fb.iter() {
        for &pixel in row.iter() {
            if pixel != 0 {
                non_zero_pixels += 1;
            }
        }
    }

    assert!(non_zero_pixels > 1000, "Expected active LCD frame pixels rendered by baremetal_lcd.bin");
}

#[test]
fn test_keypad_payload_dram_execution() {
    let payload_path_1 = Path::new("../../Nokia-2660-RE/tools/ram_payload/bin/keypad_test.bin");
    let payload_path_2 = Path::new("../Nokia-2660-RE/tools/ram_payload/bin/keypad_test.bin");
    let payload_path = if payload_path_1.exists() {
        payload_path_1
    } else if payload_path_2.exists() {
        payload_path_2
    } else {
        eprintln!("Skipping test: keypad_test.bin not found");
        return;
    };

    let data = fs::read(payload_path).expect("Failed to read keypad_test.bin");
    let mut vm = T107Vm::new(true, false, false).expect("VM create failed");

    // Load payload at DRAM 0x80100000, entry 0x80100200
    vm.load_bytes(&data, 0x8010_0000).expect("Load failed");
    vm.set_entry_point(0x8010_0200).expect("Set entry failed");

    // Boot payload and let it initialize USB, heap, pinmap, keymap, and GC9307 LCD
    let _ = vm.step(2_000_000, 2_000_000);

    // Verify key 5 press injection
    vm.press_key("5");
    let _ = vm.step(1_000_000, 1_000_000);

    // Verify key 5 release injection
    vm.release_key("5");
    let _ = vm.step(1_000_000, 1_000_000);

    let stdout_bytes = vm.get_usb_stdout();
    let stdout_str = String::from_utf8_lossy(stdout_bytes);

    assert!(stdout_str.contains("fopen(\"pinmap.bin\")"), "Payload must load pinmap.bin via USB libc server");
    assert!(stdout_str.contains("fopen(\"keymap.bin\")"), "Payload must load keymap.bin via USB libc server");
    assert!(stdout_str.contains("GC9307 Controller"), "Payload must identify GC9307 SPI display");
    assert!(stdout_str.contains("KEY 5"), "Payload must detect key 5 event");
    assert!(stdout_str.contains("DOWN [PRESSED]"), "Payload must detect DOWN key transition");
    assert!(stdout_str.contains("UP [RELEASED]"), "Payload must detect UP key transition");
    assert!(stdout_str.contains("PB: IDLE"), "Dedicated Power/End button must report IDLE");

    // Verify LCD framebuffer has been rendered by draw_keypad_ui
    let fb = vm.get_gc9307_framebuffer();
    let mut non_zero_pixels = 0;
    for row in fb.iter() {
        for &pixel in row.iter() {
            if pixel != 0 {
                non_zero_pixels += 1;
            }
        }
    }
    assert!(non_zero_pixels > 5000, "Expected GC9307 display frame rendered by keypad_test.bin");
}
