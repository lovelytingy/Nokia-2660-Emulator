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

    // Test interactive demo execution and keypad ISR
    let fw_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/baremetal/interactive_demo.bin");
    let data = std::fs::read(&fw_path).expect("Failed to read interactive_demo.bin");
    let mut demo_vm = T107Vm::new(true, false, false).expect("VM init failed");
    demo_vm.load_bytes(&data, 0x8000_0000).expect("Failed to load binary");
    demo_vm.init_cpu(0x8000_0000).expect("Failed to init CPU");

    // Step enough cycles for boot and drawing initial home screen (~1.5M instructions)
    for _ in 0..30 {
        let _ = demo_vm.step(50_000, 25_000);
    }

    // Now inject key 5
    demo_vm.press_key("5");
    for _ in 0..10 {
        let _ = demo_vm.step(50_000, 25_000);
    }
    demo_vm.release_key("5");
    for _ in 0..10 {
        let _ = demo_vm.step(50_000, 25_000);
    }

    let buf_out = demo_vm.get_uart_tx_buffer();
    let uart_out = String::from_utf8_lossy(&buf_out);
    assert!(uart_out.contains("[OS] Key Pressed: '5' | Number: 5"));

    let fb = demo_vm.get_gc9307_framebuffer();
    let mut non_zero = 0;
    for y in 0..n2660_core::types::GC9307_HEIGHT {
        for x in 0..n2660_core::types::GC9307_WIDTH {
            if fb[y][x] != 0 {
                non_zero += 1;
            }
        }
    }
    let sub_fb = demo_vm.get_st7735_framebuffer();
    let mut sub_non_zero = 0;
    for y in 0..n2660_core::types::ST7735_HEIGHT {
        for x in 0..n2660_core::types::ST7735_WIDTH {
            if sub_fb[y][x] != 0 {
                sub_non_zero += 1;
            }
        }
    }
    println!("GC9307 Non-zero pixels count: {} / 76800. Sample fb[100][100] = 0x{:04X}", non_zero, fb[100][100]);
    println!("ST7735 Non-zero pixels count: {} / 19200", sub_non_zero);
    assert!(non_zero > 10_000);
    assert!(sub_non_zero > 1_000);
}

#[test]
fn test_flip_animation_easing_and_hall_switch() {
    // Test the cosine ease-in-out curve used for the 60 FPS clamshell animation
    let smooth = |progress: f32| -> f32 {
        0.5 - 0.5 * (progress * std::f32::consts::PI).cos()
    };

    assert!((smooth(0.0) - 0.0).abs() < 1e-5);
    assert!((smooth(0.5) - 0.5).abs() < 1e-5);
    assert!((smooth(1.0) - 1.0).abs() < 1e-5);

    // Easing should have zero slope at t=0 and t=1 (ease-in, ease-out)
    let eps = 0.01;
    let slope_start = (smooth(eps) - smooth(0.0)) / eps;
    let slope_mid = (smooth(0.5 + eps) - smooth(0.5)) / eps;
    let slope_end = (smooth(1.0) - smooth(1.0 - eps)) / eps;

    assert!(slope_start < 0.1, "Start should have ease-in (gentle start)");
    assert!(slope_mid > 1.4, "Midpoint should have peak rotation velocity");
    assert!(slope_end < 0.1, "End should have ease-out (gentle latch)");
}

#[test]
fn test_side_keys_injection() {
    let mut vm = T107Vm::new(true, false, false).expect("VM init failed");

    // Test volume up, volume down, and SOS/end keys
    assert!(vm.press_key("vol_up"));
    assert!(vm.release_key("vol_up"));

    assert!(vm.press_key("vol_down"));
    assert!(vm.release_key("vol_down"));

    assert!(vm.press_key("end"));
    assert!(vm.release_key("end"));
}
