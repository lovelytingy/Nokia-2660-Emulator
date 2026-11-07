// SPDX-License-Identifier: MIT
//! Nokia 2660 Flip (Model TA-1480, Unisoc T107) Desktop Host GUI
//! Replicates the authentic Nokia 2660 Flip slender hardware proportions,
//! exact keys & parts layout from the official manual and reference photo,
//! and 60 FPS clamshell flip mechanics. Supports both Phone-Only view and Split Dev Deck.

use std::time::Instant;
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use n2660_core::types::{GC9307_HEIGHT, GC9307_WIDTH, ST7735_HEIGHT, ST7735_WIDTH};
use n2660_core::vm::T107Vm;

pub const WIN_WIDTH: usize = 990;
pub const WIN_HEIGHT: usize = 860;

// Authentic slender Nokia 2660 Flip proportions (55mm width, 108mm closed, ~202mm open)
pub const CHASSIS_W: usize = 270;  // 240px display + 15px slim side bezels
pub const CHASSIS_H: usize = 365;  // Base lower half spans 405..770
pub const CHASSIS_Y: usize = 405;  // Base chassis starts at 405
pub const HINGE_Y: usize = 405;    // Center hinge line
pub const LID_HEIGHT: usize = 380; // Upper flip lid spans 380px (25..405)

// Authentic Nokia 2660 Flip Matte Black / Charcoal Color Palette
pub const COLOR_WINDOW_BG: u32 = 0x000B0D10;
pub const COLOR_PHONE_BODY: u32 = 0x001B1D21;      // Matte Charcoal / Black composite
pub const COLOR_PHONE_BEVEL: u32 = 0x00121417;     // Darker satin bevel
pub const COLOR_PHONE_EDGE: u32 = 0x00282C32;      // Fine specular highlight rim
pub const COLOR_GLASS_PANEL: u32 = 0x000A0B0E;     // Obsidian black glossy cover panel
pub const COLOR_GLASS_RIM: u32 = 0x0022262C;       // Glass frame rim
pub const COLOR_KEYPAD_PLATE: u32 = 0x00141619;    // Keypad backplate
pub const COLOR_KEY_SURFACE: u32 = 0x001E2024;     // Individual keycap tone
pub const COLOR_KEY_PRESSED: u32 = 0x00131518;     // Pressed keycap
pub const COLOR_KEY_GRID: u32 = 0x002B2E34;        // Hairline separator grid
pub const COLOR_DPAD_RING: u32 = 0x0024272D;       // D-Pad squircle rim
pub const COLOR_DPAD_BORDER: u32 = 0x003A3F49;     // D-Pad metallic highlight edge
pub const COLOR_DPAD_CENTER: u32 = 0x00181A1D;     // D-Pad OK select button
pub const COLOR_CALL_GREEN: u32 = 0x0027AE60;      // Nokia green for Call key
pub const COLOR_END_RED: u32 = 0x00E74C3C;         // Nokia red for End/Power key
pub const COLOR_TEXT_WHITE: u32 = 0x00F0F3F8;      // Crisp white lettering
pub const COLOR_TEXT_MUTED: u32 = 0x009BA3B2;      // Soft gray for sub-labels
pub const COLOR_SILVER_NOKIA: u32 = 0x00D0D7E4;    // Chrome/silver Nokia wordmark

// Embedded 5x7 Font for hardware labels, telemetry, and live UART0 console
static FONT_5X7: [[u8; 5]; 96] = [
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
    [0x14, 0x08, 0x3E, 0x08, 0x14], // 42 *
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
    [0x08, 0x14, 0x22, 0x41, 0x00], // 60 <
    [0x14, 0x14, 0x14, 0x14, 0x14], // 61 =
    [0x00, 0x41, 0x22, 0x14, 0x08], // 62 >
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

pub struct FlipAnimation {
    pub is_animating: bool,
    pub target_open: bool,
    pub progress: f32, // 0.0 (closed) to 1.0 (open)
    pub last_tick: Instant,
    pub speed: f32, // 1.0 / duration in seconds
}

impl FlipAnimation {
    pub fn new(init_open: bool) -> Self {
        Self {
            is_animating: false,
            target_open: init_open,
            progress: if init_open { 1.0 } else { 0.0 },
            last_tick: Instant::now(),
            speed: 4.0, // 250ms smooth transition
        }
    }

    pub fn toggle(&mut self) {
        self.target_open = !self.target_open;
        self.is_animating = true;
        self.last_tick = Instant::now();
    }

    pub fn update(&mut self, dt: f32) -> bool {
        if !self.is_animating {
            return false;
        }

        let mut completed = false;
        if self.target_open {
            self.progress = (self.progress + dt * self.speed).min(1.0);
            if self.progress >= 1.0 {
                self.is_animating = false;
                completed = true;
            }
        } else {
            self.progress = (self.progress - dt * self.speed).max(0.0);
            if self.progress <= 0.0 {
                self.is_animating = false;
                completed = true;
            }
        }
        completed
    }

    pub fn smoothed(&self) -> f32 {
        0.5 - 0.5 * (self.progress * std::f32::consts::PI).cos()
    }
}

/// Smooth 60 FPS Layout Transition between Phone-Only and Split Dev Deck views
pub struct LayoutAnimation {
    pub is_animating: bool,
    pub target_show_deck: bool,
    pub progress: f32, // 0.0 = Phone Only (centered), 1.0 = Split Dev Deck (left parked)
    pub speed: f32,    // 5.0 = 200ms smooth transition
}

impl LayoutAnimation {
    pub fn new(init_show_deck: bool) -> Self {
        Self {
            is_animating: false,
            target_show_deck: init_show_deck,
            progress: if init_show_deck { 1.0 } else { 0.0 },
            speed: 5.0,
        }
    }

    pub fn toggle(&mut self) {
        self.target_show_deck = !self.target_show_deck;
        self.is_animating = true;
    }

    pub fn set_target(&mut self, show: bool) {
        if self.target_show_deck != show {
            self.target_show_deck = show;
            self.is_animating = true;
        }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        if !self.is_animating {
            return false;
        }

        let mut completed = false;
        if self.target_show_deck {
            self.progress = (self.progress + dt * self.speed).min(1.0);
            if self.progress >= 1.0 {
                self.is_animating = false;
                completed = true;
            }
        } else {
            self.progress = (self.progress - dt * self.speed).max(0.0);
            if self.progress <= 0.0 {
                self.is_animating = false;
                completed = true;
            }
        }
        completed
    }

    pub fn smoothed(&self) -> f32 {
        0.5 - 0.5 * (self.progress * std::f32::consts::PI).cos()
    }
}

/// Copies text to host OS clipboard across macOS, Linux, and Windows
pub fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        if let Ok(mut child) = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().is_ok() {
                return true;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().is_ok() {
                return true;
            }
        }
        if let Ok(mut child) = Command::new("xclip").args(["-selection", "clipboard"]).stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().is_ok() {
                return true;
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        if let Ok(mut child) = Command::new("clip.exe").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().is_ok() {
                return true;
            }
        }
    }

    false
}

struct KeyButton {
    key_name: &'static str,
    main_label: &'static str,
    sub_label: &'static str,
    rel_x: usize,
    rel_y: usize,
    w: usize,
    h: usize,
    accent: u32,
    is_nav: bool,
    number_right: bool, // When true, number is on the right and letters on left (Col 3)
}

pub fn run_desktop_gui(
    vm: &mut T107Vm,
    start_closed: bool,
    _theme: Option<&str>,
    start_phone_only: bool,
    screenshot_path: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new(
        "Nokia 2660 Flip (Unisoc T107) - Native Host Emulator",
        WIN_WIDTH,
        WIN_HEIGHT,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )?;

    window.set_target_fps(60);

    let mut flip_anim = FlipAnimation::new(!start_closed);
    vm.set_flip_state(!start_closed);

    let mut layout_anim = LayoutAnimation::new(!start_phone_only);

    let mut uart_scroll: usize = 0;
    let mut selection_anchor: Option<usize> = None;
    let mut selection_active: Option<usize> = None;
    let mut is_dragging_selection = false;
    let mut copy_feedback_time: Option<Instant> = None;

    let mut paused = false;
    let mut step_requested = false;
    let start_time = Instant::now();
    let mut last_frame_time = Instant::now();

    let mut pressed_key: Option<&'static str> = None;
    let mut last_f_key = false;
    let mut last_l_key = false;
    let mut last_mouse_left = false;
    let mut last_mouse_right = false;
    let mut flip_drag_start: Option<(f32, f32)> = None;

    // Complete physical Nokia 2660 Flip Keypad Layout (Exact match with user guide & photo)
    let buttons = vec![
        // Left Column (Column 0: rel_x = 12, w = 80)
        // 1. Left Selection Key (horizontal dash)
        KeyButton { key_name: "soft1",    main_label: "-",     sub_label: "",     rel_x: 12, rel_y: 422, w: 80, h: 32, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        // 2. Shortcut Key (overlapping squares / app menu)
        KeyButton { key_name: "shortcut", main_label: "#",     sub_label: "",     rel_x: 12, rel_y: 456, w: 80, h: 32, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        // 3. Call Key (green handset)
        KeyButton { key_name: "call",     main_label: "CALL",  sub_label: "",     rel_x: 12, rel_y: 490, w: 80, h: 34, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },

        // Right Column (Column 2: rel_x = 178, w = 80)
        // 6. Right Selection Key (horizontal dash)
        KeyButton { key_name: "soft2",    main_label: "-",     sub_label: "",     rel_x: 178, rel_y: 422, w: 80, h: 32, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        // 7. Back Key (return arrow)
        KeyButton { key_name: "back",     main_label: "BACK",  sub_label: "",     rel_x: 178, rel_y: 456, w: 80, h: 32, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        // 8. Power / End Key (red handset + power)
        KeyButton { key_name: "end",      main_label: "END",   sub_label: "",     rel_x: 178, rel_y: 490, w: 80, h: 34, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },

        // Center Column: Chrome Squircle D-Pad Navigation Rim & OK Key (spans 422..524)
        KeyButton { key_name: "up",       main_label: "^",     sub_label: "",     rel_x: 100, rel_y: 422, w: 70, h: 28, accent: COLOR_DPAD_RING,   is_nav: true,  number_right: false },
        KeyButton { key_name: "down",     main_label: "v",     sub_label: "",     rel_x: 100, rel_y: 496, w: 70, h: 28, accent: COLOR_DPAD_RING,   is_nav: true,  number_right: false },
        KeyButton { key_name: "left",     main_label: "<",     sub_label: "",     rel_x: 95,  rel_y: 440, w: 22, h: 66, accent: COLOR_DPAD_RING,   is_nav: true,  number_right: false },
        KeyButton { key_name: "right",    main_label: ">",     sub_label: "",     rel_x: 153, rel_y: 440, w: 22, h: 66, accent: COLOR_DPAD_RING,   is_nav: true,  number_right: false },
        KeyButton { key_name: "ok",       main_label: "OK",    sub_label: "",     rel_x: 117, rel_y: 450, w: 36, h: 46, accent: COLOR_DPAD_CENTER, is_nav: true,  number_right: false },

        // Row 1 Numeric Keys (1, 2, 3)
        KeyButton { key_name: "1", main_label: "1", sub_label: "oo",   rel_x: 12,  rel_y: 530, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        KeyButton { key_name: "2", main_label: "2", sub_label: "abc",  rel_x: 95,  rel_y: 530, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        KeyButton { key_name: "3", main_label: "3", sub_label: "def",  rel_x: 178, rel_y: 530, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: true },

        // Row 2 Numeric Keys (4, 5, 6)
        KeyButton { key_name: "4", main_label: "4", sub_label: "ghi",  rel_x: 12,  rel_y: 579, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        KeyButton { key_name: "5", main_label: "5", sub_label: "jkl",  rel_x: 95,  rel_y: 579, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        KeyButton { key_name: "6", main_label: "6", sub_label: "mno",  rel_x: 178, rel_y: 579, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: true },

        // Row 3 Numeric Keys (7, 8, 9)
        KeyButton { key_name: "7", main_label: "7", sub_label: "pqrs", rel_x: 12,  rel_y: 628, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        KeyButton { key_name: "8", main_label: "8", sub_label: "tuv",  rel_x: 95,  rel_y: 628, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        KeyButton { key_name: "9", main_label: "9", sub_label: "wxyz", rel_x: 178, rel_y: 628, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: true },

        // Row 4 Symbol Keys (*, 0, #)
        KeyButton { key_name: "star", main_label: "*", sub_label: "+",   rel_x: 12,  rel_y: 677, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        KeyButton { key_name: "0",    main_label: "0", sub_label: "_",   rel_x: 95,  rel_y: 677, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: false },
        KeyButton { key_name: "hash", main_label: "#", sub_label: "^",   rel_x: 178, rel_y: 677, w: 80, h: 46, accent: COLOR_KEY_SURFACE, is_nav: false, number_right: true },
    ];

    let mut buf = vec![0u32; WIN_WIDTH * WIN_HEIGHT];
    let mut frame_count: usize = 0;

    println!("[Desktop GUI] Running native Cocoa host GUI with slender Nokia 2660 Flip physical skin.");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = last_frame_time.elapsed().as_secs_f32();
        last_frame_time = Instant::now();

        // 1. Process Hotkeys
        let is_f_down = window.is_key_down(Key::F);
        if is_f_down && !last_f_key {
            flip_anim.toggle();
        }
        last_f_key = is_f_down;

        let is_l_down = window.is_key_down(Key::L);
        if is_l_down && !last_l_key {
            layout_anim.toggle();
        }
        last_l_key = is_l_down;

        // Clipboard Copy shortcut (Cmd+C / Ctrl+C)
        let is_super_or_ctrl = window.is_key_down(Key::LeftSuper)
            || window.is_key_down(Key::RightSuper)
            || window.is_key_down(Key::LeftCtrl)
            || window.is_key_down(Key::RightCtrl);
        if is_super_or_ctrl && window.is_key_down(Key::C) {
            let uart_raw = vm.get_uart_tx_buffer();
            let uart_str = String::from_utf8_lossy(&uart_raw);
            let lines: Vec<&str> = uart_str.lines().collect();
            if let (Some(a), Some(b)) = (selection_anchor, selection_active) {
                let min_idx = a.min(b);
                let max_idx = a.max(b);
                if !lines.is_empty() && min_idx < lines.len() {
                    let sel_text = lines[min_idx..=max_idx.min(lines.len() - 1)].join("\n");
                    if copy_to_clipboard(&sel_text) {
                        copy_feedback_time = Some(Instant::now());
                    }
                }
            } else if !uart_str.is_empty() {
                if copy_to_clipboard(&uart_str) {
                    copy_feedback_time = Some(Instant::now());
                }
            }
        }

        if window.is_key_down(Key::P) {
            paused = !paused;
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        if window.is_key_down(Key::S) {
            step_requested = true;
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        if window.is_key_down(Key::R) {
            let _ = vm.init_cpu(vm.entry_point);
            paused = false;
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        if window.is_key_down(Key::Q) {
            break;
        }

        // Update flip animation state
        if flip_anim.update(dt) {
            vm.set_flip_state(flip_anim.target_open);
        } else if flip_anim.is_animating {
            let is_hall_open = flip_anim.progress >= 0.5;
            if vm.is_flip_open() != is_hall_open {
                vm.set_flip_state(is_hall_open);
            }
        }

        // Update layout animation state (smooth transition between Phone-Only & Split Dev Deck)
        layout_anim.update(dt);
        let deck_factor = layout_anim.smoothed(); // 0.0 = Phone Only, 1.0 = Dev Deck fully open

        // Slender chassis horizontal alignment (smooth sliding interpolation)
        let center_phone_x = (WIN_WIDTH - CHASSIS_W) as f32 / 2.0;
        let left_phone_x = 45.0;
        let phone_x = ((1.0 - deck_factor) * center_phone_x + deck_factor * left_phone_x) as usize;

        let deck_slide = ((1.0 - deck_factor) * 650.0) as usize;
        let cur_bar_x = 355 + deck_slide;

        let dock_slide_y = (deck_factor * 90.0) as usize;
        let cur_dock_y = 794 + dock_slide_y;

        // Mouse Wheel scrolling over UART logs console
        if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel() {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
                let mx_u = mx as usize;
                let my_u = my as usize;
                if deck_factor > 0.3 && mx_u >= cur_bar_x && mx_u < cur_bar_x + 580 && my_u >= 240 && my_u < 840 {
                    if scroll_y > 0.0 {
                        uart_scroll = uart_scroll.saturating_add(3);
                    } else if scroll_y < 0.0 {
                        uart_scroll = uart_scroll.saturating_sub(3);
                    }
                }
            }
        }

        // Prepare live UART log buffers and scroll offsets for input & rendering
        let uart_raw = vm.get_uart_tx_buffer();
        let uart_str = String::from_utf8_lossy(&uart_raw);
        let lines: Vec<&str> = uart_str.lines().collect();
        let total_lines = lines.len();
        let lines_per_page = 46;
        let max_scroll = total_lines.saturating_sub(lines_per_page);
        let scroll = uart_scroll.min(max_scroll);
        let start_idx = total_lines.saturating_sub(lines_per_page + scroll);
        let end_idx = (start_idx + lines_per_page).min(total_lines);
        let display_lines = if total_lines > 0 { &lines[start_idx..end_idx] } else { &[] };
        let display_count = display_lines.len();

        let has_selection = selection_anchor.is_some() && selection_active.is_some();
        let (sel_min, sel_max) = if let (Some(a), Some(b)) = (selection_anchor, selection_active) {
            (a.min(b), a.max(b))
        } else {
            (0, 0)
        };

        // 2. Mouse Hit Testing, Drag Gestures & Control Buttons
        let mut active_key: Option<&'static str> = None;
        let is_mouse_down = window.get_mouse_down(MouseButton::Left);
        let is_mouse_right = window.get_mouse_down(MouseButton::Right);

        // Host Right-Click Copy on OS (copies selected text or line under cursor)
        if is_mouse_right && !last_mouse_right {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
                let mx_u = mx as usize;
                let my_u = my as usize;
                if deck_factor > 0.3 && mx_u >= cur_bar_x && mx_u < cur_bar_x + 580 && my_u >= 240 && my_u < 840 {
                    if has_selection {
                        if !lines.is_empty() && sel_min < lines.len() {
                            let sel_text = lines[sel_min..=sel_max.min(lines.len() - 1)].join("\n");
                            if copy_to_clipboard(&sel_text) {
                                copy_feedback_time = Some(Instant::now());
                            }
                        }
                    } else if my_u >= 272 && my_u < 272 + display_count * 12 {
                        let clicked_disp = (my_u - 272) / 12;
                        let clicked_global = start_idx + clicked_disp;
                        if clicked_global < lines.len() {
                            if copy_to_clipboard(lines[clicked_global]) {
                                copy_feedback_time = Some(Instant::now());
                            }
                        }
                    } else if !uart_str.is_empty() {
                        if copy_to_clipboard(&uart_str) {
                            copy_feedback_time = Some(Instant::now());
                        }
                    }
                }
            }
        }
        last_mouse_right = is_mouse_right;

        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
            let mx_u = mx as usize;
            let my_u = my as usize;

            // Clamshell Screen Drag Gestures:
            // Drag screen DOWN to close, Drag screen UP to open
            if is_mouse_down {
                if !last_mouse_left {
                    if mx >= (phone_x as f32 - 10.0) && mx <= (phone_x as f32 + CHASSIS_W as f32 + 10.0) {
                        if flip_anim.target_open {
                            // Upper screen / lid area when open (y = 20..410)
                            if my >= 20.0 && my <= (HINGE_Y as f32 + 10.0) {
                                flip_drag_start = Some((mx, my));
                            }
                        } else {
                            // Front cover area when closed (y = 400..790)
                            if my >= (HINGE_Y as f32 - 10.0) && my <= (HINGE_Y as f32 + LID_HEIGHT as f32 + 10.0) {
                                flip_drag_start = Some((mx, my));
                            }
                        }
                    }
                } else if let Some((_start_x, start_y)) = flip_drag_start {
                    let dy = my - start_y;
                    if flip_anim.target_open && dy > 35.0 {
                        // Dragged screen DOWN -> Close Flip
                        flip_anim.target_open = false;
                        flip_anim.is_animating = true;
                        flip_drag_start = None;
                    } else if !flip_anim.target_open && dy < -35.0 {
                        // Dragged screen UP -> Open Flip
                        flip_anim.target_open = true;
                        flip_anim.is_animating = true;
                        flip_drag_start = None;
                    }
                }
            } else {
                flip_drag_start = None;
            }

            if is_mouse_down {
                if !last_mouse_left {
                    let mut clicked_ui = false;

                    // A. Dev Deck UI Clicks
                    if deck_factor > 0.3 && mx_u >= cur_bar_x && mx_u < cur_bar_x + 580 {
                        // Action Toolbar (y = 18..54)
                        if my_u >= 18 && my_u < 54 {
                            clicked_ui = true;
                            if mx_u >= cur_bar_x && mx_u < cur_bar_x + 92 {
                                flip_anim.toggle();
                            } else if mx_u >= cur_bar_x + 98 && mx_u < cur_bar_x + 208 {
                                layout_anim.set_target(false);
                            // [PAUSE] & [STEP] commented out per user request
                            // } else if mx_u >= cur_bar_x + 206 && mx_u < cur_bar_x + 283 {
                            //     paused = !paused;
                            // } else if mx_u >= cur_bar_x + 289 && mx_u < cur_bar_x + 346 {
                            //     step_requested = true;
                            } else if mx_u >= cur_bar_x + 214 && mx_u < cur_bar_x + 289 {
                                let _ = vm.init_cpu(vm.entry_point);
                                paused = false;
                            } else if mx_u >= cur_bar_x + 295 && mx_u < cur_bar_x + 395 {
                                // Power Off
                                break;
                            }
                        }

                        // UART Header Bar (y = 240..268)
                        if my_u >= 240 && my_u < 268 {
                            clicked_ui = true;
                            if uart_scroll > 0 && mx_u >= cur_bar_x + 180 && mx_u < cur_bar_x + 245 {
                                uart_scroll = 0;
                            }
                        }

                        // UART Console Text Line Click (Start Drag Selection)
                        if my_u >= 272 && my_u < 272 + display_count * 12 && mx_u >= cur_bar_x + 8 && mx_u < cur_bar_x + 568 {
                            clicked_ui = true;
                            let clicked_disp = (my_u - 272) / 12;
                            let clicked_global = start_idx + clicked_disp;
                            selection_anchor = Some(clicked_global);
                            selection_active = Some(clicked_global);
                            is_dragging_selection = true;
                        }
                    }

                    // B. Phone-Only Bottom Dock Clicks
                    if deck_factor < 0.8 {
                        let dock_w = 440;
                        let dock_x = (WIN_WIDTH - dock_w) / 2;
                        if my_u >= cur_dock_y && my_u < cur_dock_y + 38 && mx_u >= dock_x && mx_u < dock_x + dock_w {
                            clicked_ui = true;
                            if mx_u >= dock_x + 8 && mx_u < dock_x + 118 {
                                flip_anim.toggle();
                            } else if mx_u >= dock_x + 124 && mx_u < dock_x + 229 {
                                layout_anim.set_target(true);
                            // [PAUSE] commented out per user request
                            // } else if mx_u >= dock_x + 225 && mx_u < dock_x + 305 {
                            //     paused = !paused;
                            } else if mx_u >= dock_x + 235 && mx_u < dock_x + 310 {
                                let _ = vm.init_cpu(vm.entry_point);
                                paused = false;
                            } else if mx_u >= dock_x + 316 && mx_u < dock_x + 420 {
                                // Power Off
                                break;
                            }
                        }
                    }

                    // C. Phone Hinge Click
                    if mx_u >= phone_x + 80 && mx_u < phone_x + 190 && my_u >= HINGE_Y - 8 && my_u < HINGE_Y + 16 {
                        flip_anim.toggle();
                        clicked_ui = true;
                    }

                    // D. Side Keys Clicks (Right flank of bottom base chassis: Vol Up, Vol Down, and SOS at bottom)
                    if mx_u >= phone_x + CHASSIS_W - 4 && mx_u <= phone_x + CHASSIS_W + 14 {
                        if my_u >= 442 && my_u < 478 {
                            active_key = Some("vol_up");
                            clicked_ui = true;
                        } else if my_u >= 482 && my_u < 518 {
                            active_key = Some("vol_down");
                            clicked_ui = true;
                        } else if my_u >= 700 && my_u <= 748 {
                            active_key = Some("sos");
                            clicked_ui = true;
                        }
                    }

                    // Clear selection if clicking background or phone outside console
                    if !clicked_ui {
                        selection_anchor = None;
                        selection_active = None;
                    }
                } else if is_dragging_selection && deck_factor > 0.3 {
                    // Dragging selection across lines
                    if my_u >= 272 && my_u < 832 {
                        let cur_disp = ((my_u - 272) / 12).min(display_count.saturating_sub(1));
                        selection_active = Some(start_idx + cur_disp);
                    }
                }

                // Keypad hit testing (when phone is open)
                if flip_anim.progress > 0.8 {
                    for btn in &buttons {
                        let bx = phone_x + btn.rel_x;
                        let by = btn.rel_y;
                        if mx_u >= bx && mx_u < bx + btn.w && my_u >= by && my_u < by + btn.h {
                            active_key = Some(btn.key_name);
                            break;
                        }
                    }
                }
            } else {
                is_dragging_selection = false;
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
            else if window.is_key_down(Key::RightBracket) { active_key = Some("soft2"); }
            else if window.is_key_down(Key::Tab) || window.is_key_down(Key::A) { active_key = Some("shortcut"); }
            else if window.is_key_down(Key::Backspace) || window.is_key_down(Key::B) { active_key = Some("back"); }
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
            let _ = vm.step(100_000, 0);
            step_requested = false;
            if !vm.is_flip_open() {
                vm.transfer_main_to_sub();
            }
        }

        // 4. Render Studio Background
        buf.fill(COLOR_WINDOW_BG);

        // Ambient Studio Drop Shadow beneath Phone
        draw_rounded_rect(&mut buf, phone_x.saturating_sub(6), CHASSIS_Y + 6, CHASSIS_W + 12, CHASSIS_H + 8, 28, 0x00040507);
        draw_rounded_rect(&mut buf, phone_x.saturating_sub(3), CHASSIS_Y + 3, CHASSIS_W + 6, CHASSIS_H + 4, 26, 0x0007090C);

        // ==========================================
        // LOWER CLAMSHELL CHASSIS (Base & Keypad)
        // ==========================================
        draw_rounded_rect(&mut buf, phone_x, CHASSIS_Y, CHASSIS_W, CHASSIS_H, 24, COLOR_PHONE_BEVEL);
        draw_rounded_rect(&mut buf, phone_x + 3, CHASSIS_Y + 3, CHASSIS_W - 6, CHASSIS_H - 6, 22, COLOR_PHONE_BODY);
        draw_rounded_rect_outline(&mut buf, phone_x + 3, CHASSIS_Y + 3, CHASSIS_W - 6, CHASSIS_H - 6, 22, 1, COLOR_PHONE_EDGE);

        // Physical Side Buttons & Ports (Authentic Nokia 2660 Flip TA-1480 placement on lower chassis)
        // Right flank: Volume Up, Volume Down, and Dedicated SOS Emergency Key
        let vol_up_color = if pressed_key == Some("vol_up") { COLOR_KEY_PRESSED } else { COLOR_KEY_SURFACE };
        let vol_down_color = if pressed_key == Some("vol_down") { COLOR_KEY_PRESSED } else { COLOR_KEY_SURFACE };
        let sos_color = if pressed_key == Some("sos") { 0x00E74C3C } else { 0x008A2228 };

        // Volume Up Rocker (Upper Right)
        draw_rounded_rect(&mut buf, phone_x + CHASSIS_W - 2, 445, 6, 30, 2, vol_up_color);
        draw_rect(&mut buf, phone_x + CHASSIS_W + 1, 447, 2, 26, COLOR_PHONE_EDGE);

        // Volume Down Rocker (Mid Right)
        draw_rounded_rect(&mut buf, phone_x + CHASSIS_W - 2, 485, 6, 30, 2, vol_down_color);
        draw_rect(&mut buf, phone_x + CHASSIS_W + 1, 487, 2, 26, COLOR_PHONE_EDGE);

        // Dedicated SOS Emergency Button (At bottom of right flank, textured crimson accent)
        draw_rounded_rect(&mut buf, phone_x + CHASSIS_W - 3, 705, 7, 38, 2, sos_color);
        draw_rect(&mut buf, phone_x + CHASSIS_W + 1, 707, 2, 34, 0x00C0392B);

        // Left flank: 3.5mm Headset Jack & Micro-USB Charging Port
        draw_rounded_rect(&mut buf, phone_x.saturating_sub(3), 450, 4, 18, 2, 0x000E1014);
        draw_rect(&mut buf, phone_x.saturating_sub(1), 453, 2, 12, 0x001B2028);

        draw_rounded_rect(&mut buf, phone_x.saturating_sub(4), 488, 5, 26, 2, 0x000E1014);
        draw_rect(&mut buf, phone_x.saturating_sub(2), 492, 2, 18, 0x002B3038);

        // Lower Clamshell Keypad Surface (when open or opening)
        if flip_anim.progress > 0.05 {
            // Keypad Deck Backplate
            draw_rounded_rect(&mut buf, phone_x + 8, 415, CHASSIS_W - 16, 315, 14, COLOR_KEYPAD_PLATE);

            // Subtle hairline grid grooves
            draw_rect(&mut buf, phone_x + 93, 420, 2, 305, COLOR_KEY_GRID);
            draw_rect(&mut buf, phone_x + 176, 420, 2, 305, COLOR_KEY_GRID);
            draw_rect(&mut buf, phone_x + 10, 526, CHASSIS_W - 20, 2, COLOR_KEY_GRID);
            draw_rect(&mut buf, phone_x + 10, 576, CHASSIS_W - 20, 2, COLOR_KEY_GRID);
            draw_rect(&mut buf, phone_x + 10, 625, CHASSIS_W - 20, 2, COLOR_KEY_GRID);
            draw_rect(&mut buf, phone_x + 10, 674, CHASSIS_W - 20, 2, COLOR_KEY_GRID);

            // D-Pad Chrome / Metallic Navigation Ring
            draw_rounded_rect(&mut buf, phone_x + 95, 422, 80, 102, 14, COLOR_DPAD_BORDER);
            draw_rounded_rect(&mut buf, phone_x + 97, 424, 76, 98, 12, COLOR_DPAD_RING);
            draw_rounded_rect(&mut buf, phone_x + 117, 450, 36, 46, 7, COLOR_DPAD_CENTER);
            draw_rounded_rect_outline(&mut buf, phone_x + 117, 450, 36, 46, 7, 1, COLOR_DPAD_BORDER);

            for btn in &buttons {
                let is_pressed = pressed_key == Some(btn.key_name);
                let bx = phone_x + btn.rel_x;
                let by = btn.rel_y;

                if btn.is_nav {
                    // D-Pad navigation press
                    let color = if is_pressed { COLOR_KEY_PRESSED } else { btn.accent };
                    if btn.key_name == "ok" {
                        draw_rounded_rect(&mut buf, bx, by, btn.w, btn.h, 6, color);
                        draw_text(&mut buf, bx + (btn.w.saturating_sub(12)) / 2, by + (btn.h.saturating_sub(7)) / 2, "OK", COLOR_TEXT_WHITE, 1);
                    } else {
                        let (arrow_x, arrow_y) = match btn.key_name {
                            "up" => (bx + btn.w / 2 - 3, by + 8),
                            "down" => (bx + btn.w / 2 - 3, by + 12),
                            "left" => (bx + 6, by + btn.h / 2 - 4),
                            "right" => (bx + btn.w - 12, by + btn.h / 2 - 4),
                            _ => (bx, by),
                        };
                        draw_text(&mut buf, arrow_x, arrow_y, btn.main_label, COLOR_TEXT_WHITE, 1);
                    }
                } else {
                    let bg_color = if is_pressed { COLOR_KEY_PRESSED } else { COLOR_KEY_SURFACE };

                    // Individual keycap surface
                    draw_rounded_rect(&mut buf, bx, by, btn.w, btn.h, 4, bg_color);
                    draw_rect(&mut buf, bx + 2, by + 1, btn.w - 4, 1, bg_color.wrapping_add(0x00121212));
                    draw_rect(&mut buf, bx + 2, by + btn.h - 2, btn.w - 4, 1, COLOR_KEYPAD_PLATE);

                    if btn.key_name == "soft1" || btn.key_name == "soft2" {
                        // Left & Right Selection Keys: Horizontal bar
                        draw_rect(&mut buf, bx + 20, by + 14, btn.w - 40, 3, COLOR_TEXT_WHITE);
                    } else if btn.key_name == "shortcut" {
                        // Shortcut Key: Overlapping double squares icon
                        draw_rect(&mut buf, bx + 30, by + 10, 14, 14, COLOR_TEXT_MUTED);
                        draw_rect(&mut buf, bx + 32, by + 12, 10, 10, bg_color);
                        draw_rect(&mut buf, bx + 36, by + 16, 14, 14, COLOR_TEXT_WHITE);
                        draw_rect(&mut buf, bx + 38, by + 18, 10, 10, bg_color);
                    } else if btn.key_name == "back" {
                        // Back Key: Curved return arrow
                        draw_text(&mut buf, bx + 32, by + 12, "<-", COLOR_TEXT_WHITE, 1);
                    } else if btn.key_name == "call" {
                        // Call Key: Green phone handset
                        draw_text(&mut buf, bx + 24, by + 12, "CALL", COLOR_CALL_GREEN, 1);
                        draw_rect(&mut buf, bx + 16, by + 26, btn.w - 32, 2, COLOR_CALL_GREEN);
                    } else if btn.key_name == "end" {
                        // End Key: Red phone handset
                        draw_text(&mut buf, bx + 28, by + 12, "END", COLOR_END_RED, 1);
                        draw_rect(&mut buf, bx + 16, by + 26, btn.w - 32, 2, COLOR_END_RED);
                    } else {
                        // Numeric Key: Big bold number + letters sub-label
                        let text_color = if is_pressed { 0x008A92A0 } else { COLOR_TEXT_WHITE };

                        if btn.number_right {
                            // Column 3: Letters on left, big number on right (exact photo match)
                            draw_text(&mut buf, bx + btn.w - 28, by + 14, btn.main_label, text_color, 2);
                            if !btn.sub_label.is_empty() {
                                draw_text(&mut buf, bx + 14, by + 19, btn.sub_label, COLOR_TEXT_MUTED, 1);
                            }
                        } else {
                            // Column 1 & 2: Big number on left, letters on right
                            draw_text(&mut buf, bx + 16, by + 14, btn.main_label, text_color, 2);
                            if !btn.sub_label.is_empty() {
                                draw_text(&mut buf, bx + 42, by + 19, btn.sub_label, COLOR_TEXT_MUTED, 1);
                            }
                        }

                        // Tactile locator bump on Key '5' (authentic accessibility feature)
                        if btn.key_name == "5" {
                            draw_rect(&mut buf, bx + 17, by + 32, 8, 2, 0x00E0E6F0);
                        }
                    }
                }
            }

            // Central horizontal microphone slit on the lower chin (matching photo)
            draw_rounded_rect(&mut buf, phone_x + 120, 744, 30, 3, 1, 0x000A0C0E);
        }

        // ==========================================
        // FLIP ANIMATION & TOP CLAMSHELL PRESENTATION
        // ==========================================
        let smoothed = flip_anim.smoothed();

        if smoothed >= 0.5 {
            // ------------------------------------------
            // FLIP OPEN / OPENING (Inner Face & Main Screen)
            // ------------------------------------------
            let vh = ((smoothed - 0.5) * 2.0 * LID_HEIGHT as f32) as usize;
            let vh = vh.max(8);
            let lid_top = HINGE_Y.saturating_sub(vh);

            // Upper Clamshell Body
            draw_rounded_rect(&mut buf, phone_x, lid_top, CHASSIS_W, vh, 24, COLOR_PHONE_BEVEL);
            draw_rounded_rect(&mut buf, phone_x + 3, lid_top + 3, CHASSIS_W - 6, vh.saturating_sub(6), 22, COLOR_PHONE_BODY);
            draw_rounded_rect_outline(&mut buf, phone_x + 3, lid_top + 3, CHASSIS_W - 6, vh.saturating_sub(6), 22, 1, COLOR_PHONE_EDGE);

            // Earpiece Speaker Slit & Nokia Brand Wordmark
            if vh > 90 {
                let ear_y = lid_top + 14;
                // Earpiece horizontal slit
                draw_rounded_rect(&mut buf, phone_x + 100, ear_y, 70, 4, 2, 0x00080A0C);
                // Silver acoustic mesh dots
                for mx in (phone_x + 104..phone_x + 166).step_by(4) {
                    draw_rect(&mut buf, mx, ear_y + 1, 2, 2, 0x006E7888);
                }

                // Authentic NOKIA branding right after the earpiece in iconic Nokia fonts
                if vh > 120 {
                    let logo_x = phone_x + (CHASSIS_W - 54) / 2;
                    let logo_y = ear_y + 9;
                    draw_nokia_logo(&mut buf, logo_x, logo_y, COLOR_SILVER_NOKIA);
                }
            }

            // 2.8" QVGA 240x320 Main Screen (Slim 15px side bezels)
            if vh > 60 {
                let screen_h = (vh * GC9307_HEIGHT / LID_HEIGHT).min(GC9307_HEIGHT).max(4);
                let screen_top = (HINGE_Y - 22).saturating_sub(screen_h);

                // Screen Glass Frame
                draw_rounded_rect(&mut buf, phone_x + 13, screen_top.saturating_sub(2), GC9307_WIDTH + 4, screen_h + 4, 3, 0x0006080B);

                let fb = vm.get_gc9307_framebuffer();
                for dy in 0..screen_h {
                    let py = screen_top + dy;
                    if py >= WIN_HEIGHT { break; }
                    let src_y = (dy * GC9307_HEIGHT) / screen_h;
                    let row_start = py * WIN_WIDTH;
                    let fb_row = &fb[src_y.min(GC9307_HEIGHT - 1)];

                    for dx in 0..GC9307_WIDTH {
                        let px = phone_x + 15 + dx;
                        if px < WIN_WIDTH {
                            buf[row_start + px] = rgb565_to_xrgb(fb_row[dx]);
                        }
                    }
                }

                // Silicone corner nubs (bottom bezel remains clean meeting hinge, without text)
                if vh > 350 {
                    draw_circle(&mut buf, phone_x + 8, screen_top + 8, 2, 0x00141619);
                    draw_circle(&mut buf, phone_x + CHASSIS_W - 8, screen_top + 8, 2, 0x00141619);
                }
            }
        } else {
            // ------------------------------------------
            // FLIP CLOSED / CLOSING (Outer Face & Sub-Display)
            // Replicates exactly the closed device in images.jpg
            // ------------------------------------------
            let vh = ((0.5 - smoothed) * 2.0 * LID_HEIGHT as f32) as usize;
            let vh = vh.max(8);

            // Front Clamshell Outer Cover
            draw_rounded_rect(&mut buf, phone_x, HINGE_Y, CHASSIS_W, vh, 24, COLOR_PHONE_BEVEL);
            draw_rounded_rect(&mut buf, phone_x + 3, HINGE_Y + 3, CHASSIS_W - 6, vh.saturating_sub(6), 22, COLOR_PHONE_BODY);
            draw_rounded_rect_outline(&mut buf, phone_x + 3, HINGE_Y + 3, CHASSIS_W - 6, vh.saturating_sub(6), 22, 1, COLOR_PHONE_EDGE);

            // Glossy Obsidian Black Glass Cover Panel
            if vh > 50 {
                let glass_h = vh.saturating_sub(20);
                draw_rounded_rect(&mut buf, phone_x + 14, HINGE_Y + 10, CHASSIS_W - 28, glass_h, 20, COLOR_GLASS_PANEL);
                draw_rounded_rect_outline(&mut buf, phone_x + 14, HINGE_Y + 10, CHASSIS_W - 28, glass_h, 20, 1, COLOR_GLASS_RIM);

                // Camera Module Pill & LED Flash (at top of glass panel)
                if vh > 80 {
                    let cam_y = HINGE_Y + 24;
                    draw_rounded_rect(&mut buf, phone_x + 85, cam_y, 100, 32, 16, 0x00141619);

                    // Camera Lens (multi-coated optics)
                    draw_circle(&mut buf, phone_x + 110, cam_y + 16, 10, 0x0008090A);
                    draw_circle(&mut buf, phone_x + 110, cam_y + 16, 6,  0x00192A3E); // Anti-reflective blue
                    draw_circle(&mut buf, phone_x + 110, cam_y + 16, 3,  0x00020304); // Aperture
                    draw_circle(&mut buf, phone_x + 108, cam_y + 14, 1,  0x0080B0E0); // Specular gleam

                    // LED Flash Emitter
                    draw_circle(&mut buf, phone_x + 160, cam_y + 16, 7, 0x00FFEAA7); // Warm emitter
                    draw_circle(&mut buf, phone_x + 160, cam_y + 16, 3, 0x00FFFFFF);
                }

                // 1.77" ST7735 Outer Sub-Display (120x160) - Positioned a little down for authentic balance
                if vh > 140 {
                    let sub_h = (vh * ST7735_HEIGHT / LID_HEIGHT).min(ST7735_HEIGHT).max(4);
                    let sub_y = HINGE_Y + 100; // Shifted down to visual center of front faceplate
                    let sub_x = phone_x + (CHASSIS_W - ST7735_WIDTH) / 2;

                    draw_rounded_rect(&mut buf, sub_x.saturating_sub(3), sub_y.saturating_sub(3), ST7735_WIDTH + 6, sub_h + 6, 3, 0x0006080B);

                    let sub_fb = vm.get_st7735_framebuffer();
                    for dy in 0..sub_h {
                        let py = sub_y + dy;
                        if py >= WIN_HEIGHT { break; }
                        let src_y = (dy * ST7735_HEIGHT) / sub_h;
                        let row_start = py * WIN_WIDTH;
                        let fb_row = &sub_fb[src_y.min(ST7735_HEIGHT - 1)];

                        for dx in 0..ST7735_WIDTH {
                            let px = sub_x + dx;
                            if px < WIN_WIDTH {
                                let c = fb_row[dx];
                                buf[row_start + px] = rgb565_to_xrgb(c);
                            }
                        }
                    }
                }

                // Bottom of Glass Face: Silver Nokia Wordmark in authentic Nokia font & Mic Pinhole
                if vh > 290 {
                    let logo_x = phone_x + (CHASSIS_W - 54) / 2;
                    draw_nokia_logo(&mut buf, logo_x, HINGE_Y + 280, COLOR_SILVER_NOKIA);
                    draw_circle(&mut buf, phone_x + 172, HINGE_Y + 284, 2, 0x00050709);
                }
            }
        }

        // ==========================================
        // MATTE BLACK HINGE BARREL
        // ==========================================
        draw_rect(&mut buf, phone_x + 12, HINGE_Y - 2, CHASSIS_W - 24, 4, 0x000A0C0E); // Deep seam
        draw_rounded_rect(&mut buf, phone_x + 75, HINGE_Y - 7, 120, 14, 5, COLOR_PHONE_BODY);
        draw_rect(&mut buf, phone_x + 77, HINGE_Y - 6, 116, 2, COLOR_PHONE_EDGE); // Specular highlight
        draw_rounded_rect(&mut buf, phone_x + 62, HINGE_Y - 8, 14, 16, 4, COLOR_PHONE_BEVEL);
        draw_rounded_rect(&mut buf, phone_x + 194, HINGE_Y - 8, 14, 16, 4, COLOR_PHONE_BEVEL);

        // ==========================================
        // COMPANION DEV DECK & LIVE SERIAL CONSOLE
        // ==========================================
        if deck_factor > 0.0 && cur_bar_x < WIN_WIDTH {
            // Sliding Divider Bar
            let div_x = 335 + deck_slide;
            if div_x < WIN_WIDTH {
                draw_rect(&mut buf, div_x, 10, 2, WIN_HEIGHT - 20, 0x00181B20);
            }

            // Action Toolbar (y = 18..54)
            let flip_btn_color = if flip_anim.target_open { 0x001B6328 } else { 0x00785014 };
            draw_deck_button(&mut buf, cur_bar_x, 18, 92, 36, "FLIP [F]", flip_btn_color);
            draw_deck_button(&mut buf, cur_bar_x + 98, 18, 110, 36, "PHONE ONLY", 0x001E2228);

            // [PAUSE] & [STEP] commented out per user request
            // let pause_label = if paused { "RESUME" } else { "PAUSE" };
            // let pause_btn_color = if paused { 0x00781E22 } else { 0x001E2838 };
            // draw_deck_button(&mut buf, cur_bar_x + 206, 18, 77, 36, pause_label, pause_btn_color);
            // draw_deck_button(&mut buf, cur_bar_x + 289, 18, 57, 36, "STEP", 0x00242830);

            draw_deck_button(&mut buf, cur_bar_x + 214, 18, 75, 36, "RESET", 0x002E242E);
            draw_deck_button(&mut buf, cur_bar_x + 295, 18, 95, 36, "POWER OFF", 0x00781E22);

            // Hardware Telemetry Box
            draw_rounded_rect(&mut buf, cur_bar_x, 66, 580, 160, 6, 0x000E1014);
            draw_rounded_rect(&mut buf, cur_bar_x, 66, 580, 24, 6, 0x0014171E);
            draw_text(&mut buf, cur_bar_x + 12, 73, "HARDWARE TELEMETRY (UNISOC T107 CORTEX-A7)", 0x0000E5FF, 1);

            let pc = vm.get_pc();
            let sp = vm.get_sp();
            let lr = vm.get_lr();
            let cpsr = vm.get_cpsr();
            let total_insns = vm.get_instruction_count();
            let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
            let mips = (total_insns as f64 / elapsed) / 1_000_000.0;

            draw_text(&mut buf, cur_bar_x + 12, 101, &format!("PC   : 0x{:08X}", pc), COLOR_TEXT_WHITE, 1);
            draw_text(&mut buf, cur_bar_x + 260, 101, &format!("SP   : 0x{:08X}", sp), COLOR_TEXT_WHITE, 1);
            draw_text(&mut buf, cur_bar_x + 12, 121, &format!("LR   : 0x{:08X}", lr), COLOR_TEXT_WHITE, 1);
            draw_text(&mut buf, cur_bar_x + 260, 121, &format!("CPSR : 0x{:08X} (SVC)", cpsr), COLOR_TEXT_WHITE, 1);

            draw_text(&mut buf, cur_bar_x + 12, 144, &format!("Speed: {:.2} MIPS | Insns: {}", mips, total_insns), 0x0078FF80, 1);
            draw_text(&mut buf, cur_bar_x + 12, 164, &format!("Hall Sensor: Pin 0 = {} ({})", if flip_anim.target_open { "1" } else { "0" }, if flip_anim.target_open { "OPEN" } else { "CLOSED" }), 0x00FFD54F, 1);
            draw_text(&mut buf, cur_bar_x + 260, 164, &format!("Active Key: {}", pressed_key.unwrap_or("NONE")), 0x00FFD54F, 1);

            let stage_str = match vm.get_boot_stage() {
                Some(n2660_core::types::BootStage::Boot0) => "BOOT0 SPL (SRAM)",
                Some(n2660_core::types::BootStage::Boot1) => "BOOT1 LK (DRAM Handoff)",
                Some(n2660_core::types::BootStage::Kernel) => "KernelImg (MOCOR RTOS)",
                Some(n2660_core::types::BootStage::User) => "UserImg (Nokia MMI Shell)",
                None => "Reset / BootROM",
            };
            draw_text(&mut buf, cur_bar_x + 12, 188, &format!("Boot Stage: {}", stage_str), 0x0080D8FF, 1);

            // Live UART0 Serial Console Box (with Interactive Drag Selection & Host Right-Click Copy)
            draw_rounded_rect(&mut buf, cur_bar_x, 240, 580, 600, 6, 0x000B0D10);
            draw_rounded_rect(&mut buf, cur_bar_x, 240, 580, 28, 6, 0x0013161C);
            draw_text(&mut buf, cur_bar_x + 12, 249, "UART0 BOOT LOGS (MOCOR RTOS)", 0x00FFD54F, 1);

            if let Some(t) = copy_feedback_time {
                if t.elapsed().as_secs_f32() < 1.5 {
                    draw_text(&mut buf, cur_bar_x + 360, 249, "[COPIED TO CLIPBOARD]", 0x0000E676, 1);
                }
            }

            // Jump to live bottom button if scrolled
            if uart_scroll > 0 {
                draw_pill_button(&mut buf, cur_bar_x + 240, 243, 65, 22, "v LIVE", 0x002A3644);
            }

            // Log lines
            let mut text_y = 274;
            for (disp_idx, line) in display_lines.iter().enumerate() {
                let global_idx = start_idx + disp_idx;
                let is_selected = if let (Some(a), Some(b)) = (selection_anchor, selection_active) {
                    global_idx >= a.min(b) && global_idx <= a.max(b)
                } else {
                    false
                };

                if is_selected {
                    draw_rect(&mut buf, cur_bar_x + 8, text_y - 1, 560, 12, 0x001B4B8A);
                }

                let color = if is_selected {
                    0x00FFFFFF
                } else if line.starts_with("[PASS]") || line.starts_with("[RESULT]") {
                    0x0000E676
                } else if line.starts_with("[TEST]") {
                    0x0040C4FF
                } else if line.starts_with("[N2660Emu MMIO TRAP]") {
                    0x00FFAB40
                } else if line.contains("suc") || line.contains("OK") {
                    0x0069F0AE
                } else {
                    0x00A0B2C6
                };

                let truncated = if line.len() > 80 { &line[..80] } else { line };
                draw_text(&mut buf, cur_bar_x + 12, text_y, truncated, color, 1);
                text_y += 12;
            }

            // Scrollbar track and thumb on right edge
            if total_lines > lines_per_page {
                draw_rect(&mut buf, cur_bar_x + 572, 274, 4, 556, 0x0014171E);
                let thumb_h = ((lines_per_page as f32 / total_lines as f32) * 556.0).max(16.0) as usize;
                let thumb_max_y = 556 - thumb_h;
                let thumb_y_offset = ((start_idx as f32 / max_scroll.max(1) as f32) * thumb_max_y as f32) as usize;
                draw_rounded_rect(&mut buf, cur_bar_x + 572, 274 + thumb_y_offset, 4, thumb_h, 2, 0x003A404C);
            }
        }

        // ==========================================
        // PHONE ONLY VIEW: Sleek Floating Bottom Dock (No buttons on top of phone!)
        // ==========================================
        if deck_factor < 1.0 && cur_dock_y < WIN_HEIGHT {
            let dock_w = 440;
            let dock_h = 38;
            let dock_x = (WIN_WIDTH - dock_w) / 2;

            // Translucent rounded glass pill dock (sits comfortably at the bottom below phone)
            draw_rounded_rect(&mut buf, dock_x, cur_dock_y, dock_w, dock_h, 19, 0x0012151A);
            draw_rounded_rect_outline(&mut buf, dock_x, cur_dock_y, dock_w, dock_h, 19, 1, 0x00262B34);

            let flip_color = if flip_anim.target_open { 0x001B6328 } else { 0x00785014 };
            draw_pill_button(&mut buf, dock_x + 8, cur_dock_y + 4, 110, 30, "FLIP [F]", flip_color);

            draw_pill_button(&mut buf, dock_x + 124, cur_dock_y + 4, 105, 30, "SHOW LOGS", 0x001E2532);

            // [PAUSE] commented out per user request
            // let pause_label = if paused { "RESUME" } else { "PAUSE" };
            // let pause_color = if paused { 0x00781E22 } else { 0x001E2838 };
            // draw_pill_button(&mut buf, dock_x + 225, cur_dock_y + 4, 80, 30, pause_label, pause_color);

            draw_pill_button(&mut buf, dock_x + 235, cur_dock_y + 4, 75, 30, "RESET", 0x002E242E);
            draw_pill_button(&mut buf, dock_x + 316, cur_dock_y + 4, 104, 30, "POWER OFF", 0x00781E22);

            // Bottom subtle hotkey legend
            let status_line = "[F] Flip  |  [L] Show Logs  |  [R] Reset  |  [Esc] Power Off";
            let text_x = (WIN_WIDTH.saturating_sub(status_line.len() * 6)) / 2;
            let text_y = cur_dock_y + 44;
            if text_y + 10 < WIN_HEIGHT {
                draw_text(&mut buf, text_x, text_y, status_line, 0x004A5568, 1);
            }
        }

        frame_count += 1;
        window.update_with_buffer(&buf, WIN_WIDTH, WIN_HEIGHT)?;

        if let Some(target_path) = screenshot_path {
            if frame_count >= 80 {
                save_framebuffer_image(&buf, WIN_WIDTH, WIN_HEIGHT, target_path)?;
                println!("[Desktop GUI] Screenshot ({}x{}) saved to {:?}", WIN_WIDTH, WIN_HEIGHT, target_path);
                return Ok(());
            }
        }
    }

    Ok(())
}

pub fn save_framebuffer_image(buf: &[u32], width: usize, height: usize, path: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write;
    let path_str = path.to_string_lossy();
    if path_str.ends_with(".png") {
        let ppm_path = path.with_extension("ppm");
        let mut file = std::fs::File::create(&ppm_path)?;
        writeln!(file, "P6\n{} {}\n255", width, height)?;
        let mut raw = Vec::with_capacity(width * height * 3);
        for &px in buf {
            raw.push(((px >> 16) & 0xFF) as u8);
            raw.push(((px >> 8) & 0xFF) as u8);
            raw.push((px & 0xFF) as u8);
        }
        file.write_all(&raw)?;
        let _ = std::process::Command::new("sips")
            .arg("-s")
            .arg("format")
            .arg("png")
            .arg(&ppm_path)
            .arg("--out")
            .arg(path)
            .output();
        let _ = std::fs::remove_file(ppm_path);
    } else {
        let mut file = std::fs::File::create(path)?;
        writeln!(file, "P6\n{} {}\n255", width, height)?;
        let mut raw = Vec::with_capacity(width * height * 3);
        for &px in buf {
            raw.push(((px >> 16) & 0xFF) as u8);
            raw.push(((px >> 8) & 0xFF) as u8);
            raw.push((px & 0xFF) as u8);
        }
        file.write_all(&raw)?;
    }
    Ok(())
}

fn draw_deck_button(buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, label: &str, color: u32) {
    draw_rounded_rect(buf, x, y, w, h, 6, color);
    draw_rect(buf, x + 2, y + 1, w - 4, 1, color.wrapping_add(0x00161616));
    draw_rect(buf, x + 2, y + h - 2, w - 4, 1, 0x0006080A);
    let text_x = x + (w.saturating_sub(label.len() * 6)) / 2;
    draw_text(buf, text_x, y + (h.saturating_sub(7)) / 2, label, COLOR_TEXT_WHITE, 1);
}

fn draw_pill_button(buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, label: &str, color: u32) {
    draw_rounded_rect(buf, x, y, w, h, h / 2, color);
    let text_x = x + (w.saturating_sub(label.len() * 6)) / 2;
    draw_text(buf, text_x, y + (h.saturating_sub(7)) / 2, label, COLOR_TEXT_WHITE, 1);
}

fn draw_rect(buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    for cy in y..(y + h).min(WIN_HEIGHT) {
        let row_start = cy * WIN_WIDTH;
        for cx in x..(x + w).min(WIN_WIDTH) {
            buf[row_start + cx] = color;
        }
    }
}

fn draw_rounded_rect(buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, r: usize, color: u32) {
    if w == 0 || h == 0 { return; }
    let r = r.min(w / 2).min(h / 2);
    let r_sq = (r * r) as isize;

    for dy in 0..h {
        let cy = y + dy;
        if cy >= WIN_HEIGHT { break; }
        let row_start = cy * WIN_WIDTH;

        for dx in 0..w {
            let cx = x + dx;
            if cx >= WIN_WIDTH { break; }

            let in_corner = if dx < r && dy < r {
                let ox = (r - 1 - dx) as isize;
                let oy = (r - 1 - dy) as isize;
                ox * ox + oy * oy > r_sq
            } else if dx >= w - r && dy < r {
                let ox = (dx - (w - r)) as isize;
                let oy = (r - 1 - dy) as isize;
                ox * ox + oy * oy > r_sq
            } else if dx < r && dy >= h - r {
                let ox = (r - 1 - dx) as isize;
                let oy = (dy - (h - r)) as isize;
                ox * ox + oy * oy > r_sq
            } else if dx >= w - r && dy >= h - r {
                let ox = (dx - (w - r)) as isize;
                let oy = (dy - (h - r)) as isize;
                ox * ox + oy * oy > r_sq
            } else {
                false
            };

            if !in_corner {
                buf[row_start + cx] = color;
            }
        }
    }
}

fn draw_rounded_rect_outline(buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, r: usize, thickness: usize, color: u32) {
    if w <= thickness * 2 || h <= thickness * 2 { return; }
    let r = r.min(w / 2).min(h / 2);
    let r_sq = (r * r) as isize;
    let r_inner = r.saturating_sub(thickness);
    let r_inner_sq = (r_inner * r_inner) as isize;

    for dy in 0..h {
        let cy = y + dy;
        if cy >= WIN_HEIGHT { break; }
        let row_start = cy * WIN_WIDTH;

        for dx in 0..w {
            let cx = x + dx;
            if cx >= WIN_WIDTH { break; }

            let is_edge = dx < thickness || dx >= w - thickness || dy < thickness || dy >= h - thickness;

            let in_corner = if dx < r && dy < r {
                let ox = (r - 1 - dx) as isize;
                let oy = (r - 1 - dy) as isize;
                let d2 = ox * ox + oy * oy;
                d2 <= r_sq && d2 >= r_inner_sq
            } else if dx >= w - r && dy < r {
                let ox = (dx - (w - r)) as isize;
                let oy = (r - 1 - dy) as isize;
                let d2 = ox * ox + oy * oy;
                d2 <= r_sq && d2 >= r_inner_sq
            } else if dx < r && dy >= h - r {
                let ox = (r - 1 - dx) as isize;
                let oy = (dy - (h - r)) as isize;
                let d2 = ox * ox + oy * oy;
                d2 <= r_sq && d2 >= r_inner_sq
            } else if dx >= w - r && dy >= h - r {
                let ox = (dx - (w - r)) as isize;
                let oy = (dy - (h - r)) as isize;
                let d2 = ox * ox + oy * oy;
                d2 <= r_sq && d2 >= r_inner_sq
            } else {
                is_edge
            };

            if in_corner {
                buf[row_start + cx] = color;
            }
        }
    }
}

fn draw_circle(buf: &mut [u32], cx: usize, cy: usize, radius: usize, color: u32) {
    let r_sq = (radius * radius) as isize;
    let r = radius as isize;
    for dy in -r..=r {
        let py = (cy as isize) + dy;
        if py < 0 || py >= WIN_HEIGHT as isize { continue; }
        let row_start = (py as usize) * WIN_WIDTH;
        for dx in -r..=r {
            let px = (cx as isize) + dx;
            if px < 0 || px >= WIN_WIDTH as isize { continue; }
            if dx * dx + dy * dy <= r_sq {
                buf[row_start + (px as usize)] = color;
            }
        }
    }
}

fn draw_char(buf: &mut [u32], x: usize, y: usize, c: char, color: u32, scale: usize) {
    let idx = if (c as usize) >= 32 && (c as usize) < 128 {
        (c as usize) - 32
    } else {
        0
    };
    let bitmap = FONT_5X7[idx];
    for (col_idx, &col_byte) in bitmap.iter().enumerate() {
        for bit_idx in 0..7 {
            if (col_byte & (1 << bit_idx)) != 0 {
                let px = x + col_idx * scale;
                let py = y + bit_idx * scale;
                for sy in 0..scale {
                    let cy = py + sy;
                    if cy >= WIN_HEIGHT { break; }
                    let row_start = cy * WIN_WIDTH;
                    for sx in 0..scale {
                        let cx = px + sx;
                        if cx >= WIN_WIDTH { break; }
                        buf[row_start + cx] = color;
                    }
                }
            }
        }
    }
}

fn draw_text(buf: &mut [u32], x: usize, y: usize, text: &str, color: u32, scale: usize) {
    let char_w = 6 * scale;
    for (i, c) in text.chars().enumerate() {
        draw_char(buf, x + i * char_w, y, c, color, scale);
    }
}

// Authentic Nokia logotype branding alpha mask (54x9, rasterized from official vector wordmark)
static NOKIA_LOGO_54X9: [u8; 486] = [
    0xDD, 0xDD, 0xDD, 0xDD, 0xA7, 0x02, 0x00, 0x00, 0x00, 0x67, 0xDD, 0xDD, 0x17, 0x1D, 0xB7, 0xE5, 0xFB, 0xFF, 0xFF, 0xFF, 0xFF, 0xF3, 0xD7, 0x6F, 0x00, 0x7B, 0xDD, 0xDD, 0x1C, 0x00, 0x00, 0x08, 0xA7, 0xDD, 0xDD, 0x99, 0x05, 0x96, 0xD9, 0xD6, 0x00, 0x00, 0x00, 0x00, 0x0B, 0xCE, 0xDD, 0xDD, 0xB1, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F, 0x00, 0x00, 0x00, 0x77, 0xFF, 0xFF, 0x1B, 0xA8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0x2E, 0x8E, 0xFF, 0xFF, 0x20, 0x00, 0x0C, 0xBA, 0xFF, 0xFF, 0x9C, 0x05, 0x00, 0xB2, 0xFF, 0xFD, 0x00, 0x00, 0x00, 0x00, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0x50, 0x00, 0x00, 0x00, 0x00,
    0xFF, 0xFF, 0xCB, 0xFB, 0xFF, 0xFD, 0x49, 0x00, 0x00, 0x77, 0xFF, 0xFF, 0x1B, 0xD3, 0xFF, 0xBF, 0x3A, 0x33, 0x33, 0x33, 0x33, 0x5A, 0xFF, 0xFF, 0x59, 0x8E, 0xFF, 0xFF, 0x20, 0x13, 0xC8, 0xFF, 0xFE, 0x80, 0x00, 0x00, 0x00, 0xB2, 0xFF, 0xFD, 0x00, 0x00, 0x00, 0x16, 0xF2, 0xFF, 0xC8, 0xF5, 0xFF, 0xD7, 0x04, 0x00, 0x00, 0x00,
    0xFF, 0xFF, 0x90, 0x75, 0xFF, 0xFF, 0xEE, 0x22, 0x00, 0x77, 0xFF, 0xFF, 0x1B, 0xDC, 0xFF, 0x8E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xFF, 0xFF, 0x64, 0x8E, 0xFF, 0xFF, 0x3F, 0xD9, 0xFF, 0xF7, 0x58, 0x00, 0x00, 0x00, 0x00, 0xB2, 0xFF, 0xFD, 0x00, 0x00, 0x00, 0x95, 0xFF, 0xFE, 0x37, 0x88, 0xFF, 0xFF, 0x66, 0x00, 0x00, 0x00,
    0xFF, 0xFF, 0x90, 0x01, 0xAA, 0xFF, 0xFF, 0xD0, 0x0A, 0x77, 0xFF, 0xFF, 0x1B, 0xDC, 0xFF, 0x8D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xFF, 0xFF, 0x64, 0x8E, 0xFF, 0xFF, 0xB2, 0xFF, 0xFF, 0xBF, 0x0A, 0x00, 0x00, 0x00, 0x00, 0xB2, 0xFF, 0xFD, 0x00, 0x00, 0x26, 0xFA, 0xFF, 0xAC, 0x00, 0x10, 0xED, 0xFF, 0xE9, 0x0D, 0x00, 0x00,
    0xFF, 0xFF, 0x90, 0x00, 0x0C, 0xD5, 0xFF, 0xFF, 0xA3, 0x77, 0xFF, 0xFF, 0x1B, 0xDC, 0xFF, 0x8E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xFF, 0xFF, 0x64, 0x8E, 0xFF, 0xFF, 0x23, 0x9A, 0xFF, 0xFF, 0xD0, 0x1C, 0x00, 0x00, 0x00, 0xB2, 0xFF, 0xFD, 0x00, 0x00, 0xAB, 0xFF, 0xFF, 0xAA, 0x88, 0x88, 0xD1, 0xFF, 0xFF, 0x7D, 0x00, 0x00,
    0xFF, 0xFF, 0x90, 0x00, 0x00, 0x29, 0xF2, 0xFF, 0xFF, 0xD2, 0xFF, 0xFF, 0x1B, 0xD3, 0xFF, 0xC1, 0x41, 0x33, 0x33, 0x33, 0x34, 0x62, 0xFF, 0xFF, 0x59, 0x8E, 0xFF, 0xFF, 0x20, 0x01, 0x88, 0xFF, 0xFF, 0xE4, 0x30, 0x00, 0x00, 0xB2, 0xFF, 0xFD, 0x00, 0x36, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xF3, 0x18, 0x00,
    0xFF, 0xFF, 0x90, 0x00, 0x00, 0x00, 0x4F, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0x1B, 0xA7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0x2E, 0x8E, 0xFF, 0xFF, 0x20, 0x00, 0x00, 0x77, 0xFE, 0xFF, 0xF2, 0x4A, 0x00, 0xB2, 0xFF, 0xFD, 0x00, 0xC2, 0xFF, 0xFD, 0x64, 0x55, 0x55, 0x55, 0x55, 0x9C, 0xFF, 0xFF, 0x94, 0x00,
    0xDD, 0xDD, 0x7D, 0x00, 0x00, 0x00, 0x00, 0x84, 0xDD, 0xDD, 0xDD, 0xDD, 0x17, 0x1D, 0xB5, 0xE5, 0xFB, 0xFF, 0xFF, 0xFF, 0xFF, 0xF2, 0xD6, 0x6F, 0x00, 0x7B, 0xDD, 0xDD, 0x1C, 0x00, 0x00, 0x00, 0x66, 0xDD, 0xDD, 0xD8, 0x45, 0x9D, 0xE2, 0xDF, 0x3C, 0xDD, 0xDD, 0x97, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0E, 0xD2, 0xDD, 0xD8, 0x19,
];

fn draw_nokia_logo(buf: &mut [u32], start_x: usize, start_y: usize, color: u32) {
    const LOGO_W: usize = 54;
    const LOGO_H: usize = 9;
    let fg_r = ((color >> 16) & 0xFF) as u32;
    let fg_g = ((color >> 8) & 0xFF) as u32;
    let fg_b = (color & 0xFF) as u32;

    for y in 0..LOGO_H {
        let py = start_y + y;
        if py >= WIN_HEIGHT { break; }
        let row_start = py * WIN_WIDTH;
        for x in 0..LOGO_W {
            let px = start_x + x;
            if px >= WIN_WIDTH { break; }
            let alpha = NOKIA_LOGO_54X9[y * LOGO_W + x] as u32;
            if alpha == 0 { continue; }
            if alpha >= 250 {
                buf[row_start + px] = color;
            } else {
                let bg = buf[row_start + px];
                let bg_r = (bg >> 16) & 0xFF;
                let bg_g = (bg >> 8) & 0xFF;
                let bg_b = bg & 0xFF;
                let inv = 255 - alpha;
                let r = (fg_r * alpha + bg_r * inv) / 255;
                let g = (fg_g * alpha + bg_g * inv) / 255;
                let b = (fg_b * alpha + bg_b * inv) / 255;
                buf[row_start + px] = (r << 16) | (g << 8) | b;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_animation_transition() {
        let mut anim = LayoutAnimation::new(false);
        assert_eq!(anim.progress, 0.0);
        assert!(!anim.is_animating);

        // Toggle to show deck
        anim.toggle();
        assert!(anim.is_animating);
        assert!(anim.target_show_deck);

        // Step 0.1s (speed is 5.0 -> 0.1 * 5.0 = 0.5)
        anim.update(0.1);
        assert!((anim.progress - 0.5).abs() < 1e-4);
        let factor = anim.smoothed();
        assert!((factor - 0.5).abs() < 1e-4);

        // Complete animation
        anim.update(0.15);
        assert!((anim.progress - 1.0).abs() < 1e-4);
        assert!(!anim.is_animating);
    }

    #[test]
    fn test_copy_to_clipboard_basic() {
        let sample = "N2660Emu Unit Test Clipboard Copy";
        let success = copy_to_clipboard(sample);
        #[cfg(target_os = "macos")]
        assert!(success);
    }

    #[test]
    fn test_font_character_f_and_p_distinction() {
        let f_idx = ('F' as usize) - 32;
        let p_idx = ('P' as usize) - 32;
        let f_bitmap = FONT_5X7[f_idx];
        let p_bitmap = FONT_5X7[p_idx];

        // 'F' and 'P' must never share the same glyph bitmap
        assert_ne!(f_bitmap, p_bitmap);

        // 'F' column 4 should only have the top bar (bit 0), not a closed loop (0x06)
        assert_eq!(f_bitmap[4], 0x01);
        // 'P' column 4 should have the closed right loop
        assert_eq!(p_bitmap[4], 0x06);
    }

    #[test]
    fn test_nokia_logo_dimensions_and_raster() {
        let mut test_buf = vec![0u32; WIN_WIDTH * WIN_HEIGHT];
        draw_nokia_logo(&mut test_buf, 100, 100, COLOR_SILVER_NOKIA);

        let mut non_zero = 0;
        for y in 100..109 {
            for x in 100..154 {
                if test_buf[y * WIN_WIDTH + x] != 0 {
                    non_zero += 1;
                }
            }
        }
        assert!(non_zero > 100, "Nokia logo must draw visible pixels");
        // Verify pixels outside bounding box remain untouched
        assert_eq!(test_buf[99 * WIN_WIDTH + 100], 0);
        assert_eq!(test_buf[109 * WIN_WIDTH + 100], 0);
    }
}
