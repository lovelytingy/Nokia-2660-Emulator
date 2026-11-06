// SPDX-License-Identifier: MIT
//! Integration & unit tests for GUI rendering conversions, downsampler, and keypad in Rust

use n2660_core::vm::T107Vm;

fn rgb565_to_rgb888(c: u16) -> (u8, u8, u8) {
    let r = (((c >> 11) & 0x1F) * 255 / 31) as u8;
    let g = (((c >> 5) & 0x3F) * 255 / 63) as u8;
    let b = ((c & 0x1F) * 255 / 31) as u8;
    (r, g, b)
}

fn rgb565_to_xrgb(c: u16) -> u32 {
    let (r, g, b) = rgb565_to_rgb888(c);
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[test]
fn test_rgb565_color_conversions() {
    // Black
    assert_eq!(rgb565_to_rgb888(0x0000), (0, 0, 0));
    assert_eq!(rgb565_to_xrgb(0x0000), 0x00000000);

    // White
    assert_eq!(rgb565_to_rgb888(0xFFFF), (255, 255, 255));
    assert_eq!(rgb565_to_xrgb(0xFFFF), 0x00FFFFFF);

    // Pure Red (0xF800)
    assert_eq!(rgb565_to_rgb888(0xF800), (255, 0, 0));
    assert_eq!(rgb565_to_xrgb(0xF800), 0x00FF0000);

    // Pure Green (0x07E0)
    assert_eq!(rgb565_to_rgb888(0x07E0), (0, 255, 0));
    assert_eq!(rgb565_to_xrgb(0x07E0), 0x0000FF00);

    // Pure Blue (0x001F)
    assert_eq!(rgb565_to_rgb888(0x001F), (0, 0, 255));
    assert_eq!(rgb565_to_xrgb(0x001F), 0x000000FF);
}

#[test]
fn test_keypad_injection_and_flip_state() {
    let mut vm = T107Vm::new(true, false, false).expect("VM init failed");
    assert!(vm.is_flip_open());

    // Toggle flip
    vm.set_flip_state(false);
    assert!(!vm.is_flip_open());
    vm.set_flip_state(true);
    assert!(vm.is_flip_open());

    // Inject keypresses
    assert!(vm.press_key("5"));
    assert!(vm.release_key("5"));
    assert!(vm.press_key("call"));
    assert!(vm.release_key("call"));
    assert!(vm.press_key("ok"));
    assert!(vm.release_key("ok"));
}
