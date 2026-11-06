// SPDX-License-Identifier: MIT
//! Native Desktop Host GUI for Nokia 2660 Flip Emulator with Authentic Hardware Skin

use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use n2660_core::types::{GC9307_HEIGHT, GC9307_WIDTH, ST7735_HEIGHT, ST7735_WIDTH};
use n2660_core::vm::T107Vm;

pub const WIN_WIDTH: usize = 480;
pub const WIN_HEIGHT: usize = 840;

struct KeyButton {
    key_name: &'static str,
    _label: &'static str,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

pub fn run_desktop_gui(vm: &mut T107Vm, theme: &str, start_closed: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new(
        "Nokia 2660 Flip Emulator (Unisoc T107) - Native Host GUI",
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

    let body_color: u32 = match theme.to_lowercase().as_str() {
        "pop red" => 0x00A82025,
        "classic black" => 0x001E1E22,
        _ => 0x001B2E4B, // Deep Blue (default)
    };

    let buttons = vec![
        KeyButton { key_name: "soft1", _label: "Menu", x: 60, y: 440, w: 75, h: 40 },
        KeyButton { key_name: "up",    _label: "▲",    x: 215, y: 425, w: 50, h: 30 },
        KeyButton { key_name: "soft2", _label: "Back", x: 345, y: 440, w: 75, h: 40 },

        KeyButton { key_name: "left",  _label: "◀",    x: 165, y: 460, w: 45, h: 45 },
        KeyButton { key_name: "ok",    _label: "OK",   x: 215, y: 460, w: 50, h: 45 },
        KeyButton { key_name: "right", _label: "▶",    x: 270, y: 460, w: 45, h: 45 },

        KeyButton { key_name: "call",  _label: "CALL", x: 60, y: 495, w: 75, h: 40 },
        KeyButton { key_name: "down",  _label: "▼",    x: 215, y: 510, w: 50, h: 30 },
        KeyButton { key_name: "end",   _label: "END",  x: 345, y: 495, w: 75, h: 40 },

        KeyButton { key_name: "1", _label: "1", x: 75, y: 560, w: 90, h: 45 },
        KeyButton { key_name: "2", _label: "2", x: 195, y: 560, w: 90, h: 45 },
        KeyButton { key_name: "3", _label: "3", x: 315, y: 560, w: 90, h: 45 },

        KeyButton { key_name: "4", _label: "4", x: 75, y: 620, w: 90, h: 45 },
        KeyButton { key_name: "5", _label: "5", x: 195, y: 620, w: 90, h: 45 },
        KeyButton { key_name: "6", _label: "6", x: 315, y: 620, w: 90, h: 45 },

        KeyButton { key_name: "7", _label: "7", x: 75, y: 680, w: 90, h: 45 },
        KeyButton { key_name: "8", _label: "8", x: 195, y: 680, w: 90, h: 45 },
        KeyButton { key_name: "9", _label: "9", x: 315, y: 680, w: 90, h: 45 },

        KeyButton { key_name: "star", _label: "*", x: 75, y: 740, w: 90, h: 45 },
        KeyButton { key_name: "0",    _label: "0", x: 195, y: 740, w: 90, h: 45 },
        KeyButton { key_name: "hash", _label: "#", x: 315, y: 740, w: 90, h: 45 },
    ];

    let mut buf = vec![0u32; WIN_WIDTH * WIN_HEIGHT];
    let mut pressed_key: Option<&'static str> = None;
    let mut last_flip_key = false;

    println!("[Desktop GUI] Running native Cocoa/minifb window (Theme: {}).", theme);
    println!("[Desktop GUI] Controls: Mouse click on keypad or press 0-9, *, #, C, X, [, ], Arrows, Enter.");
    println!("[Desktop GUI] Press 'F' to toggle clamshell Open/Closed.");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // 1. Process Flip Toggle Key (F)
        let is_f_down = window.is_key_down(Key::F);
        if is_f_down && !last_flip_key {
            flip_open = !flip_open;
            vm.set_flip_state(flip_open);
        }
        last_flip_key = is_f_down;

        // 2. Process Keypad Clicks & Keyboard Input
        let mut active_key: Option<&'static str> = None;

        // Mouse click hit-testing
        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
            let is_mouse_down = window.get_mouse_down(MouseButton::Left);
            if is_mouse_down && flip_open {
                let mx_u = mx as usize;
                let my_u = my as usize;
                for btn in &buttons {
                    if mx_u >= btn.x && mx_u < btn.x + btn.w && my_u >= btn.y && my_u < btn.y + btn.h {
                        active_key = Some(btn.key_name);
                        break;
                    }
                }
            }
        }

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
            else if window.is_key_down(Key::X) { active_key = Some("end"); }
            else if window.is_key_down(Key::LeftBracket) { active_key = Some("soft1"); }
            else if window.is_key_down(Key::RightBracket) { active_key = Some("soft2"); }
            else if window.is_key_down(Key::Enter) { active_key = Some("ok"); }
            else if window.is_key_down(Key::Up) { active_key = Some("up"); }
            else if window.is_key_down(Key::Down) { active_key = Some("down"); }
            else if window.is_key_down(Key::Left) { active_key = Some("left"); }
            else if window.is_key_down(Key::Right) { active_key = Some("right"); }
        }

        // Apply key changes to VM
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
        let _ = vm.step(50_000, 25_000);

        // 4. Render Framebuffer
        buf.fill(0x00121418);

        // Outer chassis
        draw_rect(&mut buf, 30, 20, 420, 790, body_color);
        draw_rect(&mut buf, 34, 24, 412, 782, body_color.wrapping_add(0x000A0A0A));

        // Clamshell hinge separator
        draw_rect(&mut buf, 30, 400, 420, 10, 0x000F1216);
        draw_rect(&mut buf, 180, 398, 120, 14, 0x004A505A);

        if flip_open {
            draw_rect(&mut buf, 210, 35, 60, 6, 0x00222222);
            draw_rect(&mut buf, 200, 48, 80, 10, 0x00CCCCCC);

            // Main Display Screen Bezel
            draw_rect(&mut buf, 110, 68, GC9307_WIDTH + 8, GC9307_HEIGHT + 8, 0x000A0C0E);

            let fb = vm.get_gc9307_framebuffer();
            for dy in 0..GC9307_HEIGHT {
                for dx in 0..GC9307_WIDTH {
                    let rgb565 = fb[dy][dx];
                    let xrgb = rgb565_to_xrgb(rgb565);
                    buf[(72 + dy) * WIN_WIDTH + (114 + dx)] = xrgb;
                }
            }

            // Keypad Deck (Lower Clamshell)
            for btn in &buttons {
                let is_pressed = pressed_key == Some(btn.key_name);
                let btn_bg = if is_pressed {
                    0x0010151C
                } else if btn.key_name == "call" {
                    0x001D4828
                } else if btn.key_name == "end" {
                    0x005A1F22
                } else {
                    0x00242830
                };

                draw_rect(&mut buf, btn.x, btn.y, btn.w, btn.h, btn_bg);
                draw_rect(&mut buf, btn.x + 1, btn.y + 1, btn.w - 2, 2, btn_bg.wrapping_add(0x001E1E1E));
                draw_rect(&mut buf, btn.x + btn.w / 2 - 4, btn.y + btn.h / 2 - 2, 8, 4, 0x00FFFFFF);
            }
        } else {
            // Closed Flip: Camera Module + Flash + ST7735 Outer Sub-screen
            draw_rect(&mut buf, 190, 80, 100, 45, 0x000F1216);
            draw_rect(&mut buf, 210, 92, 22, 22, 0x00050505);
            draw_circle(&mut buf, 260, 103, 7, 0x00FFFFAA);

            draw_rect(&mut buf, 195, 140, 90, 14, 0x00D0D5DD);

            draw_rect(&mut buf, 176, 171, ST7735_WIDTH + 8, ST7735_HEIGHT + 8, 0x0008090C);
            let sub_fb = vm.get_st7735_framebuffer();
            for dy in 0..ST7735_HEIGHT {
                for dx in 0..ST7735_WIDTH {
                    let rgb565 = sub_fb[dy][dx];
                    let xrgb = rgb565_to_xrgb(rgb565);
                    buf[(175 + dy) * WIN_WIDTH + (180 + dx)] = xrgb;
                }
            }

            draw_rect(&mut buf, 80, 460, 320, 280, body_color.wrapping_sub(0x00060606));
            draw_rect(&mut buf, 190, 580, 100, 12, 0x00A0A5AE);
        }

        // Bottom Telemetry Bar
        draw_rect(&mut buf, 0, 816, WIN_WIDTH, 24, 0x000A0C0E);
        draw_rect(&mut buf, 10, 822, 8, 8, if flip_open { 0x0000FF00 } else { 0x00FFAA00 });

        window.update_with_buffer(&buf, WIN_WIDTH, WIN_HEIGHT)?;
    }

    Ok(())
}

fn draw_rect(buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    for cy in y..(y + h).min(WIN_HEIGHT) {
        for cx in x..(x + w).min(WIN_WIDTH) {
            buf[cy * WIN_WIDTH + cx] = color;
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

fn rgb565_to_xrgb(c: u16) -> u32 {
    let r = (((c >> 11) & 0x1F) * 255 / 31) as u32;
    let g = (((c >> 5) & 0x3F) * 255 / 63) as u32;
    let b = ((c & 0x1F) * 255 / 31) as u32;
    (r << 16) | (g << 8) | b
}
