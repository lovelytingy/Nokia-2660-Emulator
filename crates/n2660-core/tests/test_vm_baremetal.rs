// SPDX-License-Identifier: MIT
//! End-to-end baremetal firmware verification in Rust T107Vm

use n2660_core::vm::T107Vm;
use std::path::Path;

#[test]
fn test_baremetal_test_firmware_execution() {
    let mut vm = T107Vm::new(true, true, false).expect("Failed to initialize T107 VM");

    let bin_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("tests/baremetal/test_firmware.bin");

    assert!(bin_path.exists(), "test_firmware.bin must exist at {:?}", bin_path);

    let bytes_loaded = vm.load_binary(bin_path.to_str().unwrap(), 0x8000_0000)
        .expect("Failed to load test_firmware.bin");
    assert!(bytes_loaded > 0);

    vm.init_cpu(0x8000_0000).expect("Failed to init CPU");

    println!("[VM] Starting Rust VM execution at PC = 0x80000000...");

    // Step up to 2,000,000 instructions or until test completes
    let mut finished = false;
    for _ in 0..100 {
        let _ = vm.step(20_000, 50_000);
        let buf = vm.get_uart_tx_buffer();
        let uart_out = String::from_utf8_lossy(&buf);
        if uart_out.contains("ALL PHASE 1, 2, 3 & 4") && uart_out.contains("CHECKS PASSED") {
            finished = true;
            break;
        }
        // If we hit a breakpoint/halt or unmapped trap
        let pc = vm.get_pc();
        if pc == 0 {
            break;
        }
    }

    let buf = vm.get_uart_tx_buffer();
    let uart_out = String::from_utf8_lossy(&buf);
    println!("\n=== Rust VM Output Summary ===");
    println!("Executed {} instructions.", vm.get_instruction_count());
    println!("Final PC: 0x{:08X}", vm.get_pc());

    assert!(finished, "Firmware test did not complete with all checks passed!\nUART buffer:\n{}", uart_out);
    assert!(uart_out.contains("Internal SRAM (IRAM) verified successfully"));
    assert!(uart_out.contains("High DRAM (LPDDR2) verified successfully"));
    assert!(uart_out.contains("SPRD System Timer counter down-counting verified"));
    assert!(uart_out.contains("SPRD GPIO pin direction and output state verified"));
    assert!(uart_out.contains("GC9307 display rendering test completed"));
    assert!(uart_out.contains("Keypad matrix and flip sensor interface verified"));
    assert!(uart_out.contains("Outer sub-display rendering test completed"));
    assert!(uart_out.contains("SPI NAND JEDEC ID, page programming, cache readback & XIP verified"));
    assert!(uart_out.contains("Spreadtrum BSL protocol framing, CRC16 & FDL1 loading verified"));
    assert!(uart_out.contains("SPRD Audio DAC, PCM FIFO streaming & tone buzzer verified"));
    assert!(uart_out.contains("BootROM reset vector, SPRD header validation & BOOT0 staging verified"));
    assert!(uart_out.contains("ARM Boot Tags (ATAG_CORE, MEM, CMDLINE, SERIAL) & kernel handoff verified"));
    assert!(uart_out.contains("SPRD Interrupt Controller, software IRQs & hardware IRQ vectors verified"));
}
