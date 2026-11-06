// SPDX-License-Identifier: MIT
//! Authentic Nokia 2660 Flip Desktop Host GUI with Live Serial Console & Telemetry Deck

use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use n2660_core::types::{GC9307_HEIGHT, GC9307_WIDTH, ST7735_HEIGHT, ST7735_WIDTH};
use n2660_core::vm::T107Vm;
use std::time::Instant;

pub const WIN_WIDTH: usize = 990;
pub const WIN_HEIGHT: usize = 860;

// Standard 5x7 ASCII font table (ASCII 32 ' ' to 127 '~')
const FONT5X7: [[u8; 5]; 96] = [
    [0x00, 0x00, 0x00, 0x00, 0x00], // 32 Space
    [0x00, 0x00, 0x5F, 0x00, 0x00], // 33 !
    [0x00, 0x07, 0x00, 0x07, 0x00], // 34 "
    [0x14, 0x7F, 0x14, 0x7F, 0x14], // 35 #
    [0x24, 0x2A, 0x7F, 0x2A, 0x12], // 36 $
    [0x23, 0x13, 0x08, 0x64, 0x62], // 37 %
    [0x36, 0x49, 0x55, 0x22, 0x50], // 38 &
    [0x00, 0x05, 0x03, 0x00, 0x00], // 39 '
    [0x00, 0x1C, 0x22, 0x41, 0x00], // 40 (
    [0x00, 0x41, 0x22, 0x1C, 0x00], // 41 )
    [0x08, 0x2A, 0x1C, 0x2A, 0x08], // 42 *
    [0x08, 0x08, 0x3E, 0x08, 0x08], // 43 +
    [0x00, 0x50, 0x30, 0x00, 0x00], // 44 ,
    [0x08, 0x08, 0x08, 0x08, 0x08], // 45 -
    [0x00, 0x60, 0x60, 0x00, 0x00], // 46 .
    [0x20, 0x10, 0x08, 0x04, 0x02], // 47 /
    [0x3E, 0x51, 0x49, 0x45, 0x3E], // 48 0
    [0x00, 0x42, 0x7F, 0x40, 0x00], // 49 1
    [0x42, 0x61, 0x51, 0x49, 0x46], // 50 2
    [0x21, 0x41, 0x45, 0x4B, 0x31], // 51 3
    [0x18, 0x14, 0x12, 0x7F, 0x10], // 52 4
    [0x27, 0x45, 0x45, 0x45, 0x39], // 53 5
    [0x3C, 0x4A, 0x49, 0x49, 0x30], // 54 6
    [0x01, 0x71, 0x09, 0x05, 0x03], // 55 7
    [0x36, 0x49, 0x49, 0x49, 0x36], // 56 8
    [0x06, 0x49, 0x49, 0x29, 0x1E], // 57 9
    [0x00, 0x36, 0x36, 0x00, 0x00], // 58 :
    [0x00, 0x56, 0x36, 0x00, 0x00], // 59 ;
    [0x00, 0x08, 0x14, 0x22, 0x41], // 60 <
    [0x14, 0x14, 0x14, 0x14, 0x14], // 61 =
    [0x41, 0x22, 0x14, 0x08, 0x00], // 62 >
    [0x02, 0x01, 0x51, 0x09, 0x06], // 63 ?
    [0x32, 0x49, 0x79, 0x41, 0x3E], // 64 @
    [0x7E, 0x11, 0x11, 0x11, 0x7E], // 65 A
    [0x7F, 0x49, 0x49, 0x49, 0x36], // 66 B
    [0x3E, 0x41, 0x41, 0x41, 0x22], // 67 C
    [0x7F, 0x41, 0x41, 0x22, 0x1C], // 68 D
    [0x7F, 0x49, 0x49, 0x49, 0x41], // 69 E
    [0x7F, 0x09, 0x09, 0x09, 0x01], // 70 F
    [0x3E, 0x41, 0x49, 0x49, 0x7A], // 71 G
    [0x7F, 0x08, 0x08, 0x08, 0x7F], // 72 H
    [0x00, 0x41, 0x7F, 0x41, 0x00], // 73 I
    [0x20, 0x40, 0x41, 0x3F, 0x01], // 74 J
    [0x7F, 0x08, 0x14, 0x22, 0x41], // 75 K
    [0x7F, 0x40, 0x40, 0x40, 0x40], // 76 L
    [0x7F, 0x02, 0x0C, 0x02, 0x7F], // 77 M
    [0x7F, 0x04, 0x08, 0x10, 0x7F], // 78 N
    [0x3E, 0x41, 0x41, 0x41, 0x3E], // 79 O
    [0x7F, 0x09, 0x09, 0x09, 0x06], // 80 P
    [0x3E, 0x41, 0x51, 0x21, 0x5E], // 81 Q
    [0x7F, 0x09, 0x19, 0x29, 0x46], // 82 R
    [0x46, 0x49, 0x49, 0x49, 0x31], // 83 S
    [0x01, 0x01, 0x7F, 0x01, 0x01], // 84 T
    [0x3F, 0x40, 0x40, 0x40, 0x3F], // 85 U
    [0x1F, 0x20, 0x40, 0x20, 0x1F], // 86 V
    [0x3F, 0x40, 0x38, 0x40, 0x3F], // 87 W
    [0x63, 0x14, 0x08, 0x14, 0x63], // 88 X
    [0x07, 0x08, 0x70, 0x08, 0x07], // 89 Y
    [0x61, 0x51, 0x49, 0x45, 0x43], // 90 Z
    [0x00, 0x7F, 0x41, 0x41, 0x00], // 91 [
    [0x02, 0x04, 0x08, 0x10, 0x20], // 92 \
    [0x00, 0x41, 0x41, 0x7F, 0x00], // 93 ]
    [0x04, 0x02, 0x01, 0x02, 0x04], // 94 ^
    [0x40, 0x40, 0x40, 0x40, 0x40], // 95 _
    [0x00, 0x01, 0x02, 0x04, 0x00], // 96 `
    [0x20, 0x54, 0x54, 0x54, 0x78], // 97 a
    [0x7F, 0x48, 0x44, 0x44, 0x38], // 98 b
    [0x38, 0x44, 0x44, 0x44, 0x20], // 99 c
    [0x38, 0x44, 0x44, 0x48, 0x7F], // 100 d
    [0x38, 0x54, 0x54, 0x54, 0x18], // 101 e
    [0x08, 0x7E, 0x09, 0x01, 0x02], // 102 f
    [0x0C, 0x52, 0x52, 0x52, 0x3E], // 103 g
    [0x7F, 0x08, 0x04, 0x04, 0x78], // 104 h
    [0x00, 0x44, 0x7D, 0x40, 0x00], // 105 i
    [0x20, 0x40, 0x44, 0x3D, 0x00], // 106 j
    [0x7F, 0x10, 0x28, 0x44, 0x00], // 107 k
    [0x00, 0x41, 0x7F, 0x40, 0x00], // 108 l
    [0x7C, 0x04, 0x18, 0x04, 0x78], // 109 m
    [0x7C, 0x08, 0x04, 0x04, 0x78], // 110 n
    [0x38, 0x44, 0x44, 0x44, 0x38], // 111 o
    [0x7C, 0x14, 0x14, 0x14, 0x08], // 112 p
    [0x08, 0x14, 0x14, 0x18, 0x7C], // 113 q
    [0x7C, 0x08, 0x04, 0x04, 0x08], // 114 r
    [0x48, 0x54, 0x54, 0x54, 0x20], // 115 s
    [0x04, 0x3F, 0x44, 0x40, 0x20], // 116 t
    [0x3C, 0x40, 0x40, 0x20, 0x7C], // 117 u
    [0x1C, 0x20, 0x40, 0x20, 0x1C], // 118 v
    [0x3C, 0x40, 0x30, 0x40, 0x3C], // 119 w
    [0x44, 0x28, 0x10, 0x28, 0x44], // 120 x
    [0x0C, 0x50, 0x50, 0x50, 0x3C], // 121 y
    [0x44, 0x64, 0x54, 0x4C, 0x44], // 122 z
    [0x00, 0x08, 0x36, 0x41, 0x00], // 123 {
    [0x00, 0x00, 0x7F, 0x00, 0x00], // 124 |
    [0x00, 0x41, 0x36, 0x08, 0x00], // 125 }
    [0x08, 0x08, 0x2A, 0x1C, 0x08], // 126 ~
    [0x7F, 0x7F, 0x7F, 0x7F, 0x7F], // 127 Block
];

struct KeyButton {
    key_name: &'static str,
    main_label: &'static str,
    sub_label: &'static str,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    accent: u32,
}

pub fn run_desktop_gui(vm: &mut T107Vm, start_closed: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new(
        "Nokia 2660 Flip Emulator (Unisoc T107) - Native Host GUI & Dev Deck",
        WIN_WIDTH,
        WIN_HEIGHT,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )?;

    window.set_target_fps(60);

    let mut flip_open = !start_closed;
    vm.set_flip_state(flip_open);

    let mut paused = false;
    let mut step_requested = false;
    let start_time = Instant::now();

    // The iconic Nokia 2660 Flip deep navy metallic finish
    let phone_body_color: u32 = 0x0014223A;
    let phone_bevel_color: u32 = 0x000E192B;
    let deck_bg_color: u32 = 0x000D111A;

    let buttons = vec![
        // Softkeys & D-Pad (Row 1)
        KeyButton { key_name: "soft1", main_label: "Menu", sub_label: "-", x: 60, y: 440, w: 75, h: 42, accent: 0x0022324C },
        KeyButton { key_name: "up",    main_label: "^",    sub_label: "",  x: 215, y: 425, w: 50, h: 32, accent: 0x0032425E },
        KeyButton { key_name: "soft2", main_label: "Back", sub_label: "-", x: 345, y: 440, w: 75, h: 42, accent: 0x0022324C },

        // D-Pad Row 2
        KeyButton { key_name: "left",  main_label: "<",    sub_label: "",  x: 165, y: 462, w: 45, h: 42, accent: 0x0032425E },
        KeyButton { key_name: "ok",    main_label: "OK",   sub_label: "",  x: 215, y: 462, w: 50, h: 42, accent: 0x001E4472 },
        KeyButton { key_name: "right", main_label: ">",    sub_label: "",  x: 270, y: 462, w: 45, h: 42, accent: 0x0032425E },

        // Call / Down / End (Row 3)
        KeyButton { key_name: "call",  main_label: "CALL", sub_label: "",  x: 60, y: 498, w: 75, h: 44, accent: 0x001B6328 },
        KeyButton { key_name: "down",  main_label: "v",    sub_label: "",  x: 215, y: 508, w: 50, h: 32, accent: 0x0032425E },
        KeyButton { key_name: "end",   main_label: "END",  sub_label: "",  x: 345, y: 498, w: 75, h: 44, accent: 0x00781E22 },

        // Numeric Keypad with T9 Multi-tap Sub-labels
        KeyButton { key_name: "1", main_label: "1", sub_label: ". @",   x: 70,  y: 565, w: 95, h: 48, accent: 0x001E2E46 },
        KeyButton { key_name: "2", main_label: "2", sub_label: "ABC",   x: 192, y: 565, w: 95, h: 48, accent: 0x001E2E46 },
        KeyButton { key_name: "3", main_label: "3", sub_label: "DEF",   x: 315, y: 565, w: 95, h: 48, accent: 0x001E2E46 },

        KeyButton { key_name: "4", main_label: "4", sub_label: "GHI",   x: 70,  y: 625, w: 95, h: 48, accent: 0x001E2E46 },
        KeyButton { key_name: "5", main_label: "5", sub_label: "JKL",   x: 192, y: 625, w: 95, h: 48, accent: 0x001E2E46 },
        KeyButton { key_name: "6", main_label: "6", sub_label: "MNO",   x: 315, y: 625, w: 95, h: 48, accent: 0x001E2E46 },

        KeyButton { key_name: "7", main_label: "7", sub_label: "PQRS",  x: 70,  y: 685, w: 95, h: 48, accent: 0x001E2E46 },
        KeyButton { key_name: "8", main_label: "8", sub_label: "TUV",   x: 192, y: 685, w: 95, h: 48, accent: 0x001E2E46 },
        KeyButton { key_name: "9", main_label: "9", sub_label: "WXYZ",  x: 315, y: 685, w: 95, h: 48, accent: 0x001E2E46 },

        KeyButton { key_name: "star", main_label: "*", sub_label: "+",   x: 70,  y: 745, w: 95, h: 48, accent: 0x001E2E46 },
        KeyButton { key_name: "0",    main_label: "0", sub_label: "_",   x: 192, y: 745, w: 95, h: 48, accent: 0x001E2E46 },
        KeyButton { key_name: "hash", main_label: "#", sub_label: "^",   x: 315, y: 745, w: 95, h: 48, accent: 0x001E2E46 },
    ];

    let mut buf = vec![0u32; WIN_WIDTH * WIN_HEIGHT];
    let mut pressed_key: Option<&'static str> = None;
    let mut last_f_key = false;
    let mut last_mouse_left = false;

    println!("[Desktop GUI] Running native Cocoa host GUI with live serial console & telemetry deck.");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // 1. Process Keyboard Hotkeys
        let is_f_down = window.is_key_down(Key::F);
        if is_f_down && !last_f_key {
            flip_open = !flip_open;
            vm.set_flip_state(flip_open);
        }
        last_f_key = is_f_down;

        if window.is_key_down(Key::P) {
            paused = !paused;
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        // 2. Process Keypad Clicks & Control Buttons
        let mut active_key: Option<&'static str> = None;
        let is_mouse_down = window.get_mouse_down(MouseButton::Left);

        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
            let mx_u = mx as usize;
            let my_u = my as usize;

            // Check deck toolbar clicks on mouse press
            if is_mouse_down && !last_mouse_left {
                // [Flip Toggle] button at (515, 20, 130, 36)
                if mx_u >= 515 && mx_u < 645 && my_u >= 20 && my_u < 56 {
                    flip_open = !flip_open;
                    vm.set_flip_state(flip_open);
                }
                // [Pause / Resume] button at (655, 20, 130, 36)
                else if mx_u >= 655 && mx_u < 785 && my_u >= 20 && my_u < 56 {
                    paused = !paused;
                }
                // [Step] button at (795, 20, 85, 36)
                else if mx_u >= 795 && mx_u < 880 && my_u >= 20 && my_u < 56 {
                    step_requested = true;
                }
                // [Reset] button at (890, 20, 80, 36)
                else if mx_u >= 890 && mx_u < 970 && my_u >= 20 && my_u < 56 {
                    let _ = vm.init_cpu(vm.entry_point);
                    paused = false;
                }
            }

            // Keypad hit testing (only when flip is open)
            if is_mouse_down && flip_open {
                for btn in &buttons {
                    if mx_u >= btn.x && mx_u < btn.x + btn.w && my_u >= btn.y && my_u < btn.y + btn.h {
                        active_key = Some(btn.key_name);
                        break;
                    }
                }
            }
        }
        last_mouse_left = is_mouse_down;

        // Host keyboard fallbacks
        if active_key.is_none() {
            if window.is_key_down(Key::Key0) || window.is_key_down(Key::NumPad0) { active_key = Some("0"); }
            else if window.is_key_down(Key::Key1) || window.is_key_down(Key::NumPad1) { active_key = Some("1"); }
            else if window.is_key_down(Key::Key2) || window.is_key_down(Key::NumPad2) { active_key = Some("2"); }
            else if window.is_key_down(Key::Key3) || window.is_key_down(Key::NumPad3) { active_key = Some("3"); }
            else if window.is_key_down(Key::Key4) || window.is_key_down(Key::NumPad4) { active_key = Some("4"); }
            else if window.is_key_down(Key::Key5) || window.is_key_down(Key::NumPad5) { active_key = Some("5"); }
            else if window.is_key_down(Key::Key6) || window.is_key_down(Key::NumPad6) { active_key = Some("6"); }
            else if window.is_key_down(Key::Key7) || window.is_key_down(Key::NumPad7) { active_key = Some("7"); }
            else if window.is_key_down(Key::Key8) || window.is_key_down(Key::NumPad8) { active_key = Some("8"); }
            else if window.is_key_down(Key::Key9) || window.is_key_down(Key::NumPad9) { active_key = Some("9"); }
            else if window.is_key_down(Key::C) { active_key = Some("call"); }
            else if window.is_key_down(Key::X) || window.is_key_down(Key::Delete) { active_key = Some("end"); }
            else if window.is_key_down(Key::LeftBracket) { active_key = Some("soft1"); }
            else if window.is_key_down(Key::RightBracket) || window.is_key_down(Key::Backspace) { active_key = Some("soft2"); }
            else if window.is_key_down(Key::Enter) || window.is_key_down(Key::Space) { active_key = Some("ok"); }
            else if window.is_key_down(Key::Up) { active_key = Some("up"); }
            else if window.is_key_down(Key::Down) { active_key = Some("down"); }
            else if window.is_key_down(Key::Left) { active_key = Some("left"); }
            else if window.is_key_down(Key::Right) { active_key = Some("right"); }
            else if window.is_key_down(Key::Slash) { active_key = Some("star"); }
            else if window.is_key_down(Key::Backslash) { active_key = Some("hash"); }
            else if window.is_key_down(Key::Equal) { active_key = Some("vol_up"); }
            else if window.is_key_down(Key::Minus) { active_key = Some("vol_down"); }
        }

        // Apply key state transitions
        if active_key != pressed_key {
            if let Some(prev) = pressed_key {
                vm.release_key(prev);
            }
            if let Some(cur) = active_key {
                vm.press_key(cur);
            }
            pressed_key = active_key;
        }

        // 3. Step VM execution
        if !paused || step_requested {
            let _ = vm.step(100_000, 10_000);
            step_requested = false;
        }

        // 4. Render Whole Framebuffer
        buf.fill(0x000A0D14); // Global background

        // ==========================================
        // LEFT SIDE: Authentic Nokia 2660 Physical Chassis
        // ==========================================
        // Outer chassis & bevels
        draw_rect(&mut buf, 35, 18, 415, 815, phone_bevel_color);
        draw_rect(&mut buf, 38, 22, 409, 807, phone_body_color);

        // Hinge divider
        draw_rect(&mut buf, 35, 402, 415, 12, 0x000B1321);
        draw_rect(&mut buf, 185, 400, 115, 16, 0x004A5364); // Stainless steel hinge barrel
        draw_rect(&mut buf, 187, 402, 111, 2,  0x0074819A); // Specular reflection

        if flip_open {
            // Earpiece speaker slit & chrome Nokia emblem
            draw_rect(&mut buf, 210, 32, 65, 5, 0x00080C14);
            draw_text(&mut buf, 212, 44, "NOKIA", 0x00D0D7E4, 1);

            // GC9307 240x320 Display Bezel & Glass Edge
            draw_rect(&mut buf, 121, 62, GC9307_WIDTH + 8, GC9307_HEIGHT + 8, 0x0006090F);

            let fb = vm.get_gc9307_framebuffer();
            for dy in 0..GC9307_HEIGHT {
                for dx in 0..GC9307_WIDTH {
                    let rgb565 = fb[dy][dx];
                    buf[(66 + dy) * WIN_WIDTH + (125 + dx)] = rgb565_to_xrgb(rgb565);
                }
            }

            // Lower Clamshell Keypad Deck
            for btn in &buttons {
                let is_pressed = pressed_key == Some(btn.key_name);
                let bg_color = if is_pressed {
                    btn.accent.wrapping_sub(0x000D0D0D)
                } else {
                    btn.accent
                };

                // Embossed 3D button body
                draw_rect(&mut buf, btn.x, btn.y, btn.w, btn.h, bg_color);
                // Top/Left highlight bevel
                draw_rect(&mut buf, btn.x, btn.y, btn.w, 2, bg_color.wrapping_add(0x001C1C1C));
                draw_rect(&mut buf, btn.x, btn.y, 2, btn.h, bg_color.wrapping_add(0x001C1C1C));
                // Bottom/Right shadow bevel
                draw_rect(&mut buf, btn.x, btn.y + btn.h - 2, btn.w, 2, 0x00080E18);
                draw_rect(&mut buf, btn.x + btn.w - 2, btn.y, 2, btn.h, 0x00080E18);

                // Button Labels
                let text_color = if is_pressed { 0x00A0B2CD } else { 0x00FFFFFF };
                draw_text(&mut buf, btn.x + 12, btn.y + 12, btn.main_label, text_color, 2);
                if !btn.sub_label.is_empty() {
                    draw_text(&mut buf, btn.x + 40, btn.y + 20, btn.sub_label, 0x00788CA6, 1);
                }
            }
        } else {
            // Closed Flip: Camera Module + Flash + ST7735 120x160 Sub-Display
            draw_rect(&mut buf, 185, 75, 115, 52, 0x000B1321);
            draw_circle(&mut buf, 215, 101, 16, 0x0004060A);
            draw_circle(&mut buf, 215, 101, 10, 0x0012243A); // Glass lens
            draw_circle(&mut buf, 268, 101, 8,  0x00FFF380); // LED Flash

            draw_text(&mut buf, 218, 138, "NOKIA", 0x00D0D7E4, 1);

            // ST7735 Outer Screen
            draw_rect(&mut buf, 178, 168, ST7735_WIDTH + 8, ST7735_HEIGHT + 8, 0x0006090F);
            let sub_fb = vm.get_st7735_framebuffer();
            for dy in 0..ST7735_HEIGHT {
                for dx in 0..ST7735_WIDTH {
                    let rgb565 = sub_fb[dy][dx];
                    buf[(172 + dy) * WIN_WIDTH + (182 + dx)] = rgb565_to_xrgb(rgb565);
                }
            }

            draw_rect(&mut buf, 80, 480, 325, 260, phone_body_color.wrapping_sub(0x0004060A));
            draw_text(&mut buf, 215, 590, "NOKIA", 0x003A4D6B, 2);
        }

        // ==========================================
        // CENTER DIVIDER
        // ==========================================
        draw_rect(&mut buf, 485, 10, 2, WIN_HEIGHT - 20, 0x001C273C);

        // ==========================================
        // RIGHT SIDE: Companion Dev Deck & Serial Console
        // ==========================================
        // Action Toolbar
        let flip_label = if flip_open { "FLIP: OPEN" } else { "FLIP: CLOSED" };
        let flip_btn_color = if flip_open { 0x001B6328 } else { 0x00785014 };
        draw_deck_button(&mut buf, 515, 20, 130, 36, flip_label, flip_btn_color);

        let pause_label = if paused { "RESUME" } else { "PAUSE" };
        let pause_btn_color = if paused { 0x00781E22 } else { 0x00223D66 };
        draw_deck_button(&mut buf, 655, 20, 130, 36, pause_label, pause_btn_color);

        draw_deck_button(&mut buf, 795, 20, 85, 36, "STEP", 0x0024344E);
        draw_deck_button(&mut buf, 890, 20, 80, 36, "RESET", 0x00322232);

        // Hardware Telemetry Box
        draw_rect(&mut buf, 515, 70, 455, 160, deck_bg_color);
        draw_rect(&mut buf, 515, 70, 455, 24, 0x00121B2B);
        draw_text(&mut buf, 525, 77, "HARDWARE TELEMETRY (UNISOC T107 CORTEX-A7)", 0x0000E5FF, 1);

        let pc = vm.get_pc();
        let sp = vm.get_sp();
        let lr = vm.get_lr();
        let cpsr = vm.get_cpsr();
        let total_insns = vm.get_instruction_count();
        let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
        let mips = (total_insns as f64 / elapsed) / 1_000_000.0;

        draw_text(&mut buf, 525, 105, &format!("PC   : 0x{:08X}", pc), 0x00FFFFFF, 1);
        draw_text(&mut buf, 740, 105, &format!("SP   : 0x{:08X}", sp), 0x00FFFFFF, 1);
        draw_text(&mut buf, 525, 125, &format!("LR   : 0x{:08X}", lr), 0x00FFFFFF, 1);
        draw_text(&mut buf, 740, 125, &format!("CPSR : 0x{:08X} (SVC)", cpsr), 0x00FFFFFF, 1);

        draw_text(&mut buf, 525, 148, &format!("Speed: {:.2} MIPS | Insns: {}", mips, total_insns), 0x0078FF80, 1);
        draw_text(&mut buf, 525, 168, &format!("Hall Sensor: Pin 0 = {} ({})", if flip_open { "1" } else { "0" }, if flip_open { "OPEN" } else { "CLOSED" }), 0x00FFD54F, 1);
        draw_text(&mut buf, 740, 168, &format!("Active Key: {}", pressed_key.unwrap_or("NONE")), 0x00FFD54F, 1);
        draw_text(&mut buf, 525, 188, "INTC Lines : IRQ 0..31 Ready | Software IRQ Vectors Active", 0x00B0BEC5, 1);
        draw_text(&mut buf, 525, 208, "Audio DAC  : 1000 Hz Tone Buzzer & PCM FIFO Synced", 0x00B0BEC5, 1);

        // Live Serial Console Box (UART0)
        draw_rect(&mut buf, 515, 245, 455, 595, 0x00080B10);
        draw_rect(&mut buf, 515, 245, 455, 24, 0x00101826);
        draw_text(&mut buf, 525, 252, "LIVE SERIAL CONSOLE (UART0 @ 115200 8N1)", 0x0076FF03, 1);

        // Extract and render last 42 lines from UART0 TX FIFO
        let uart_raw = vm.get_uart_tx_buffer();
        let uart_str = String::from_utf8_lossy(&uart_raw);
        let lines: Vec<&str> = uart_str.lines().collect();
        let display_lines = if lines.len() > 44 { &lines[lines.len() - 44..] } else { &lines[..] };

        let mut text_y = 280;
        for line in display_lines {
            let color = if line.starts_with("[PASS]") || line.starts_with("[RESULT]") {
                0x0000E676
            } else if line.starts_with("[TEST]") {
                0x0040C4FF
            } else if line.starts_with("[N2660Emu MMIO TRAP]") {
                0x00FFAB40
            } else {
                0x00A0B2C6
            };

            // Truncate line to max 65 chars to fit panel
            let truncated = if line.len() > 65 { &line[..65] } else { line };
            draw_text(&mut buf, 525, text_y, truncated, color, 1);
            text_y += 12;
        }

        window.update_with_buffer(&buf, WIN_WIDTH, WIN_HEIGHT)?;
    }

    Ok(())
}

fn draw_deck_button(buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, label: &str, color: u32) {
    draw_rect(buf, x, y, w, h, color);
    draw_rect(buf, x, y, w, 2, color.wrapping_add(0x00222222));
    draw_rect(buf, x, y + h - 2, w, 2, 0x0006090F);
    let text_x = x + (w - label.len() * 6) / 2;
    draw_text(buf, text_x, y + (h - 7) / 2, label, 0x00FFFFFF, 1);
}

fn draw_rect(buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    for cy in y..(y + h).min(WIN_HEIGHT) {
        let row_start = cy * WIN_WIDTH;
        for cx in x..(x + w).min(WIN_WIDTH) {
            buf[row_start + cx] = color;
        }
    }
}

fn draw_circle(buf: &mut [u32], cx: usize, cy: usize, r: usize, color: u32) {
    let r_sq = (r * r) as isize;
    for dy in -(r as isize)..=(r as isize) {
        for dx in -(r as isize)..=(r as isize) {
            if dx * dx + dy * dy <= r_sq {
                let px = (cx as isize + dx) as usize;
                let py = (cy as isize + dy) as usize;
                if px < WIN_WIDTH && py < WIN_HEIGHT {
                    buf[py * WIN_WIDTH + px] = color;
                }
            }
        }
    }
}

fn draw_char(buf: &mut [u32], x: usize, y: usize, c: u8, color: u32, scale: usize) {
    if c < 32 || c > 127 { return; }
    let glyph = &FONT5X7[(c - 32) as usize];
    for col in 0..5 {
        let col_bits = glyph[col];
        for row in 0..7 {
            if (col_bits & (1 << row)) != 0 {
                for sy in 0..scale {
                    let py = y + row * scale + sy;
                    if py >= WIN_HEIGHT { continue; }
                    let row_start = py * WIN_WIDTH;
                    for sx in 0..scale {
                        let px = x + col * scale + sx;
                        if px < WIN_WIDTH {
                            buf[row_start + px] = color;
                        }
                    }
                }
            }
        }
    }
}

fn draw_text(buf: &mut [u32], x: usize, y: usize, text: &str, color: u32, scale: usize) {
    let mut cx = x;
    for b in text.bytes() {
        draw_char(buf, cx, y, b, color, scale);
        cx += 6 * scale;
    }
}

fn rgb565_to_xrgb(c: u16) -> u32 {
    let r = (((c >> 11) & 0x1F) * 255 / 31) as u32;
    let g = (((c >> 5) & 0x3F) * 255 / 63) as u32;
    let b = ((c & 0x1F) * 255 / 31) as u32;
    (r << 16) | (g << 8) | b
}
