// SPDX-License-Identifier: MIT
//! Integration & unit tests for SPRD Interrupt Controller (INTC) in Rust

use n2660_core::peripherals::intc::SprdIntc;
use n2660_core::types::*;
use n2660_core::vm::T107Vm;
use std::path::Path;

#[test]
fn test_intc_reset_state() {
    let intc = SprdIntc::default();
    assert_eq!(intc.read(0x0000), 0); // IRQ_RAW_STS
    assert_eq!(intc.read(0x0004), 0); // IRQ_MASK_STS
    assert_eq!(intc.read(0x0008), 0); // IRQ_EN
    assert_eq!(intc.read(0x0010), 0); // IRQ_SOFT
    assert_eq!(intc.read(0x0020), 0); // FIQ_MASK_STS
}

#[test]
fn test_intc_enable_and_clear() {
    let mut intc = SprdIntc::default();
    // Enable IRQ 0, 5, 15
    intc.write(0x0008, (1 << 0) | (1 << 5) | (1 << 15));
    assert_eq!(intc.read(0x0008), (1 << 0) | (1 << 5) | (1 << 15));

    // Clear IRQ 5 via IRQ_EN_CLR (0x000C)
    intc.write(0x000C, 1 << 5);
    assert_eq!(intc.read(0x0008), (1 << 0) | (1 << 15));
}

#[test]
fn test_intc_software_interrupt_and_masking() {
    let mut intc = SprdIntc::default();
    // Enable IRQ 15 only
    intc.write(0x0008, 1 << 15);

    // Trigger soft IRQ 10 and 15
    intc.write(0x0010, (1 << 10) | (1 << 15));

    // Raw status reflects all active lines
    let raw = intc.read(0x0000);
    assert_eq!(raw & (1 << 10), 1 << 10);
    assert_eq!(raw & (1 << 15), 1 << 15);

    // Masked status reflects only enabled lines
    let mask = intc.read(0x0004);
    assert_eq!(mask, 1 << 15);

    // Clear soft IRQ 15 via IRQ_SOFT_CLR (0x0014)
    intc.write(0x0014, 1 << 15);
    let mask_after = intc.read(0x0004);
    assert_eq!(mask_after, 0);
}

#[test]
fn test_intc_fiq_type_routing() {
    let mut intc = SprdIntc::default();
    // Route IRQ 1 to FIQ (bit 1 = 1) via INTC_TYPE (0x0034)
    intc.write(0x0034, 1 << 1);
    // Enable both IRQ and FIQ for bit 1
    intc.write(0x0008, 1 << 1);
    intc.write(0x0024, 1 << 1);

    // Assert hardware interrupt on line 1
    intc.assert_irq(1);

    // Line 1 is routed to FIQ, so IRQ masked status must be 0
    let irq_mask = intc.read(0x0004);
    assert_eq!(irq_mask & (1 << 1), 0);

    // FIQ masked status should reflect line 1
    let fiq_mask = intc.read(0x0020);
    assert_eq!(fiq_mask & (1 << 1), 1 << 1);
}

#[test]
fn test_intc_peripheral_assertion() {
    let mut intc = SprdIntc::default();
    intc.write(0x0008, (1 << SPRD_IRQ_TIMER) | (1 << SPRD_IRQ_KPD));

    intc.assert_irq(SPRD_IRQ_TIMER);
    assert_eq!(intc.read(0x0004), 1 << SPRD_IRQ_TIMER);

    intc.assert_irq(SPRD_IRQ_KPD);
    assert_eq!(intc.read(0x0004), (1 << SPRD_IRQ_TIMER) | (1 << SPRD_IRQ_KPD));

    intc.clear_irq(SPRD_IRQ_TIMER);
    assert_eq!(intc.read(0x0004), 1 << SPRD_IRQ_KPD);
}

#[test]
fn test_intc_baremetal_execution() {
    let fw_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/baremetal/test_firmware.bin");
    assert!(fw_path.exists());

    let mut vm = T107Vm::new(true, false, false).expect("Failed to create VM");
    let data = std::fs::read(&fw_path).expect("Failed to read test_firmware.bin");
    vm.load_bytes(&data, 0x8000_0000).expect("Failed to load binary");
    vm.init_cpu(0x8000_0000).expect("Failed to init CPU");

    for _ in 0..100 {
        let _ = vm.step(50_000, 25_000);
        let buf = vm.get_uart_tx_buffer();
        let s = String::from_utf8_lossy(&buf);
        if s.contains("ALL PHASE 1, 2, 3 & 4") && s.contains("CHECKS PASSED") {
            assert!(s.contains("[INTC] Software IRQ 15 successfully executed"));
            assert!(s.contains("[INTC] Keypad IRQ 5 successfully executed"));
            return;
        }
    }
    panic!("INTC baremetal test did not complete within step budget");
}
