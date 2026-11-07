// SPDX-License-Identifier: MIT
//! GalaxyCore GC9307 (SPI0) and Sitronix ST7735 (SPI1) TFT LCD controllers

use crate::types::{BootStage, GC9307_HEIGHT, GC9307_WIDTH, ST7735_HEIGHT, ST7735_WIDTH};

// Standard RGB565 Color Constants
pub const COLOR_BLACK: u16 = 0x0000;
pub const COLOR_WHITE: u16 = 0xFFFF;
pub const COLOR_NOKIA_BLUE: u16 = 0x016C; // Authentic deep Nokia blue
pub const COLOR_DARK_NAVY: u16 = 0x0842;
pub const COLOR_SLATE: u16 = 0x18E3;
pub const COLOR_CYAN: u16 = 0x07FF;
pub const COLOR_GREEN: u16 = 0x07E0;
pub const COLOR_YELLOW: u16 = 0xFFE0;
pub const COLOR_RED: u16 = 0xF800;
pub const COLOR_GRAY: u16 = 0x8410;
pub const COLOR_LIGHT_GRAY: u16 = 0xC618;

// Standard 5x7 ASCII font table (ASCII 32 ' ' to 127 '~')
pub const FONT5X7: [[u8; 5]; 96] = [
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

#[derive(Clone)]
pub struct LcdController<const W: usize, const H: usize> {
    pub dc_mode: bool,
    pub cmd: u8,
    pub params: Vec<u8>,
    pub col_start: usize,
    pub col_end: usize,
    pub row_start: usize,
    pub row_end: usize,
    pub cur_x: usize,
    pub cur_y: usize,
    pub pixel_byte: Option<u8>,
    pub framebuffer: Box<[[u16; W]; H]>,
}

impl<const W: usize, const H: usize> Default for LcdController<W, H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const H: usize> LcdController<W, H> {
    pub fn new() -> Self {
        Self {
            dc_mode: false,
            cmd: 0,
            params: Vec::with_capacity(16),
            col_start: 0,
            col_end: W.saturating_sub(1),
            row_start: 0,
            row_end: H.saturating_sub(1),
            cur_x: 0,
            cur_y: 0,
            pixel_byte: None,
            framebuffer: Box::new([[0u16; W]; H]),
        }
    }

    pub fn write_cmd(&mut self, cmd: u8) {
        self.cmd = cmd;
        self.params.clear();
        self.pixel_byte = None;
        if cmd == 0x2C {
            // RAMWR: reset to window origin
            self.cur_x = self.col_start.min(W.saturating_sub(1));
            self.cur_y = self.row_start.min(H.saturating_sub(1));
        }
    }

    pub fn write_data(&mut self, val: u8) {
        if self.cmd == 0x2C {
            // RAMWR pixel streaming
            if let Some(hi) = self.pixel_byte.take() {
                let pixel = ((hi as u16) << 8) | (val as u16);
                if self.cur_y < H && self.cur_x < W {
                    self.framebuffer[self.cur_y][self.cur_x] = pixel;
                }
                self.cur_x += 1;
                if self.cur_x > self.col_end || self.cur_x >= W {
                    self.cur_x = self.col_start.min(W.saturating_sub(1));
                    self.cur_y += 1;
                    if self.cur_y > self.row_end || self.cur_y >= H {
                        self.cur_y = self.row_start.min(H.saturating_sub(1));
                    }
                }
            } else {
                self.pixel_byte = Some(val);
            }
        } else {
            self.params.push(val);
            if self.cmd == 0x2A && self.params.len() == 4 {
                self.col_start = (((self.params[0] as usize) << 8) | (self.params[1] as usize)).min(W.saturating_sub(1));
                self.col_end = (((self.params[2] as usize) << 8) | (self.params[3] as usize)).min(W.saturating_sub(1));
                self.cur_x = self.col_start;
            } else if self.cmd == 0x2B && self.params.len() == 4 {
                self.row_start = (((self.params[0] as usize) << 8) | (self.params[1] as usize)).min(H.saturating_sub(1));
                self.row_end = (((self.params[2] as usize) << 8) | (self.params[3] as usize)).min(H.saturating_sub(1));
                self.cur_y = self.row_start;
            }
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u16) {
        for dy in 0..h {
            let py = y + dy;
            if py >= H { break; }
            for dx in 0..w {
                let px = x + dx;
                if px >= W { break; }
                self.framebuffer[py][px] = color;
            }
        }
    }

    pub fn draw_char(&mut self, x: usize, y: usize, c: u8, color: u16, bg: u16, scale: usize) {
        let idx = if (32..=127).contains(&c) {
            (c - 32) as usize
        } else {
            0
        };
        let glyph = FONT5X7[idx];
        for col in 0..5 {
            let bits = glyph[col];
            for row in 0..7 {
                let is_on = (bits & (1 << row)) != 0;
                let pixel_color = if is_on { color } else { bg };
                if pixel_color != bg || bg != 0 {
                    self.fill_rect(x + col * scale, y + row * scale, scale, scale, pixel_color);
                }
            }
        }
    }

    pub fn draw_text(&mut self, x: usize, y: usize, text: &str, color: u16, bg: u16, scale: usize) {
        let mut cx = x;
        for &b in text.as_bytes() {
            if cx + 6 * scale > W { break; }
            self.draw_char(cx, y, b, color, bg, scale);
            cx += 6 * scale;
        }
    }
}

pub type Gc9307Controller = LcdController<GC9307_WIDTH, GC9307_HEIGHT>;
pub type St7735Controller = LcdController<ST7735_WIDTH, ST7735_HEIGHT>;

impl Gc9307Controller {
    pub fn render_boot_stage(&mut self, stage: BootStage, sub_status: &str, pct: u32) {
        // Deep authentic Nokia blue background
        self.fill_rect(0, 0, GC9307_WIDTH, GC9307_HEIGHT, COLOR_NOKIA_BLUE);

        // Header Bar
        self.fill_rect(0, 0, GC9307_WIDTH, 26, COLOR_BLACK);
        self.draw_text(8, 8, "NOKIA 2660 FLIP (TA-1480)", COLOR_WHITE, COLOR_BLACK, 1);
        self.draw_text(180, 8, "BOOT", COLOR_CYAN, COLOR_BLACK, 1);

        // Bold Nokia Classic Wordmark
        self.draw_text(38, 55, "NOKIA", COLOR_WHITE, COLOR_NOKIA_BLUE, 5);
        self.draw_text(52, 102, "Connecting People", COLOR_CYAN, COLOR_NOKIA_BLUE, 1);

        // Stage Information Card
        self.fill_rect(10, 125, GC9307_WIDTH - 20, 120, COLOR_DARK_NAVY);
        self.fill_rect(10, 125, GC9307_WIDTH - 20, 2, COLOR_CYAN);
        self.fill_rect(10, 243, GC9307_WIDTH - 20, 2, COLOR_BLACK);

        match stage {
            BootStage::Boot0 => {
                self.draw_text(16, 133, "[STAGE 1/4] BOOT0 SPL (IRAM)", COLOR_CYAN, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 153, "* LPDDR2 Memory Init: OK", COLOR_WHITE, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 171, "* Internal SRAM 0x00006200 Ready", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 189, "* NAND Flash ID: 0xBAEF (OK)", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 207, "* Loading BOOT1 to DRAM 0x80100000...", COLOR_YELLOW, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 225, sub_status, COLOR_GREEN, COLOR_DARK_NAVY, 1);
            }
            BootStage::Boot1 => {
                self.draw_text(16, 133, "[STAGE 2/4] BOOT1 MINILOADER (DRAM)", COLOR_YELLOW, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 153, "* Mounting SCI FTL Partitions...", COLOR_WHITE, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 171, "* Checking Flash Translation Layer: OK", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 189, "* Verifying KernelImg Signature...", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 207, "* Staging KernelImg at 0x80A05E00", COLOR_GREEN, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 225, sub_status, COLOR_CYAN, COLOR_DARK_NAVY, 1);
            }
            BootStage::Kernel => {
                self.draw_text(16, 133, "[STAGE 3/4] MOCOR 20A RTOS KERNEL", COLOR_GREEN, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 153, "* CP15 SCTLR / Banked Stacks: OK", COLOR_WHITE, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 171, "* C Runtime __scatterload Done", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 189, "* ThreadX Preemptive Scheduler Active", COLOR_CYAN, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 207, "* Launching Task 0x15: MS_MMI_Main...", COLOR_YELLOW, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 225, sub_status, COLOR_WHITE, COLOR_DARK_NAVY, 1);
            }
            BootStage::User => {
                self.render_mmi_home("", false, 0);
                return;
            }
        }

        // Progress Bar
        self.fill_rect(16, 260, GC9307_WIDTH - 32, 14, COLOR_BLACK);
        let bar_w = ((GC9307_WIDTH - 36) * pct.min(100) as usize) / 100;
        self.fill_rect(18, 262, bar_w, 10, COLOR_GREEN);
        self.draw_text(85, 282, &format!("BOOTING: {}%", pct), COLOR_WHITE, COLOR_NOKIA_BLUE, 1);

        // Softkey Bar
        self.fill_rect(0, 300, GC9307_WIDTH, 20, COLOR_BLACK);
        self.draw_text(10, 305, "Unisoc T107", COLOR_GRAY, COLOR_BLACK, 1);
        self.draw_text(165, 305, "ARMv7-A", COLOR_GRAY, COLOR_BLACK, 1);
    }

    pub fn render_mmi_home(&mut self, dial_str: &str, in_call: bool, _menu_idx: usize) {
        // Deep authentic Nokia blue background
        self.fill_rect(0, 0, GC9307_WIDTH, GC9307_HEIGHT, COLOR_NOKIA_BLUE);

        // Status Bar (Top 24px, Black)
        self.fill_rect(0, 0, GC9307_WIDTH, 24, COLOR_BLACK);

        // Signal Reception Bars (4 ascending green bars)
        self.fill_rect(8, 16, 3, 5, COLOR_GREEN);
        self.fill_rect(13, 13, 3, 8, COLOR_GREEN);
        self.fill_rect(18, 10, 3, 11, COLOR_GREEN);
        self.fill_rect(23, 7, 3, 14, COLOR_GREEN);
        self.draw_text(30, 8, "Nokia 4G", COLOR_WHITE, COLOR_BLACK, 1);
        self.draw_text(85, 8, "VoLTE", COLOR_CYAN, COLOR_BLACK, 1);

        // Battery Meter (Top right)
        self.draw_text(172, 8, "85%", COLOR_CYAN, COLOR_BLACK, 1);
        self.fill_rect(198, 7, 24, 12, COLOR_WHITE);
        self.fill_rect(200, 9, 16, 8, COLOR_GREEN);
        self.fill_rect(222, 10, 2, 6, COLOR_WHITE);

        // Centered Digital Clock
        self.draw_text(60, 50, "12:34", COLOR_WHITE, COLOR_NOKIA_BLUE, 4);
        self.draw_text(52, 90, "Friday 07 Nov", COLOR_CYAN, COLOR_NOKIA_BLUE, 2);

        // Nokia Brand Line
        self.fill_rect(20, 118, GC9307_WIDTH - 40, 2, COLOR_CYAN);
        self.draw_text(42, 130, "NOKIA 2660 FLIP", COLOR_WHITE, COLOR_NOKIA_BLUE, 2);
        self.draw_text(68, 152, "Model TA-1480", COLOR_YELLOW, COLOR_NOKIA_BLUE, 1);
        self.fill_rect(20, 168, GC9307_WIDTH - 40, 2, COLOR_CYAN);

        // Interactive Dialing / Status Panel
        self.fill_rect(12, 180, GC9307_WIDTH - 24, 76, COLOR_DARK_NAVY);
        self.fill_rect(12, 180, GC9307_WIDTH - 24, 2, COLOR_CYAN);
        self.fill_rect(12, 254, GC9307_WIDTH - 24, 2, COLOR_BLACK);

        if !dial_str.is_empty() {
            self.draw_text(20, 190, "DIALPAD:", COLOR_YELLOW, COLOR_DARK_NAVY, 1);
            self.draw_text(20, 206, dial_str, COLOR_WHITE, COLOR_DARK_NAVY, 3);
            if in_call {
                self.draw_text(20, 238, "[IN CALL] 00:03 - Connected", COLOR_GREEN, COLOR_DARK_NAVY, 1);
            } else {
                self.draw_text(20, 238, "Press CALL to Connect", COLOR_GREEN, COLOR_DARK_NAVY, 1);
            }
        } else {
            self.draw_text(20, 192, "MOCOR 20A RTOS Active", COLOR_GREEN, COLOR_DARK_NAVY, 1);
            self.draw_text(20, 210, "Press any key to dial", COLOR_WHITE, COLOR_DARK_NAVY, 1);
            self.draw_text(20, 226, "T9 Keypad ready for input", COLOR_CYAN, COLOR_DARK_NAVY, 1);
            self.draw_text(20, 242, "Flip Open | Main Screen Active", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
        }

        // Softkeys (Bottom 24px)
        self.fill_rect(0, 296, GC9307_WIDTH, 24, COLOR_BLACK);
        self.draw_text(10, 302, "Menu", COLOR_CYAN, COLOR_BLACK, 1);
        self.draw_text(98, 302, "Select", COLOR_WHITE, COLOR_BLACK, 1);
        self.draw_text(175, 302, "Contacts", COLOR_CYAN, COLOR_BLACK, 1);
    }
}

impl St7735Controller {
    pub fn render_cover_screen(&mut self, stage: Option<BootStage>, is_open: bool) {
        self.fill_rect(0, 0, ST7735_WIDTH, ST7735_HEIGHT, COLOR_DARK_NAVY);

        // Header: Battery & status
        self.fill_rect(0, 0, ST7735_WIDTH, 18, COLOR_BLACK);
        self.draw_text(6, 5, "Nokia", COLOR_WHITE, COLOR_BLACK, 1);
        self.draw_text(75, 5, "85%", COLOR_GREEN, COLOR_BLACK, 1);

        if is_open {
            self.draw_text(22, 60, "FLIP OPEN", COLOR_YELLOW, COLOR_DARK_NAVY, 1);
            self.draw_text(14, 80, "Main Screen Active", COLOR_CYAN, COLOR_DARK_NAVY, 1);
        } else if let Some(stg) = stage {
            match stg {
                BootStage::Boot0 => {
                    self.draw_text(20, 45, "BOOTING", COLOR_WHITE, COLOR_DARK_NAVY, 2);
                    self.draw_text(25, 75, "Stage 1: BOOT0", COLOR_CYAN, COLOR_DARK_NAVY, 1);
                    self.draw_text(30, 95, "SPL Init...", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                }
                BootStage::Boot1 => {
                    self.draw_text(20, 45, "BOOTING", COLOR_WHITE, COLOR_DARK_NAVY, 2);
                    self.draw_text(25, 75, "Stage 2: BOOT1", COLOR_YELLOW, COLOR_DARK_NAVY, 1);
                    self.draw_text(30, 95, "Loading OS...", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                }
                BootStage::Kernel => {
                    self.draw_text(25, 45, "MOCOR", COLOR_WHITE, COLOR_DARK_NAVY, 2);
                    self.draw_text(25, 75, "Stage 3: Kernel", COLOR_GREEN, COLOR_DARK_NAVY, 1);
                    self.draw_text(30, 95, "RTOS Active", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                }
                BootStage::User => {
                    self.draw_text(28, 45, "12:34", COLOR_WHITE, COLOR_DARK_NAVY, 3);
                    self.draw_text(26, 80, "Fri, 07 Nov", COLOR_CYAN, COLOR_DARK_NAVY, 1);
                    self.draw_text(18, 105, "4G LTE Active", COLOR_GREEN, COLOR_DARK_NAVY, 1);
                }
            }
        } else {
            self.draw_text(28, 45, "12:34", COLOR_WHITE, COLOR_DARK_NAVY, 3);
            self.draw_text(26, 80, "Fri, 07 Nov", COLOR_CYAN, COLOR_DARK_NAVY, 1);
            self.draw_text(18, 105, "4G LTE Active", COLOR_GREEN, COLOR_DARK_NAVY, 1);
        }

        // Sub-screen bottom brand
        self.draw_text(40, 142, "NOKIA", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
    }
}

#[derive(Clone, Default)]
pub struct SprdDisplaySubsystem {
    pub gc9307: Gc9307Controller,
    pub st7735: St7735Controller,
}

impl SprdDisplaySubsystem {
    pub fn new() -> Self {
        Self {
            gc9307: Gc9307Controller::new(),
            st7735: St7735Controller::new(),
        }
    }

    pub fn read_spi0(&self, offset: u32) -> u32 {
        match offset {
            0x0C => 1 << 8, // SPI_STS1: TX FIFO Empty / ready
            _ => 0,
        }
    }

    pub fn write_spi0(&mut self, offset: u32, val: u32) {
        match offset {
            0x18 => {
                // SPI_CTL4_REG (D/C select: 0 = Command, 1 = Data)
                self.gc9307.dc_mode = (val & 0x1) != 0;
            }
            0x00 => {
                // SPI_TXD_REG: Transmit byte/word based on D/C flag
                if !self.gc9307.dc_mode {
                    self.gc9307.write_cmd((val & 0xFF) as u8);
                } else if val <= 0xFF {
                    self.gc9307.write_data(val as u8);
                } else if val <= 0xFFFF {
                    // 16-bit RGB565 pixel
                    self.gc9307.write_data((val >> 8) as u8);
                    self.gc9307.write_data((val & 0xFF) as u8);
                } else {
                    // 32-bit dual-pixel pair
                    self.gc9307.write_data((val >> 24) as u8);
                    self.gc9307.write_data((val >> 16) as u8);
                    self.gc9307.write_data((val >> 8) as u8);
                    self.gc9307.write_data((val & 0xFF) as u8);
                }
            }
            0x04 => {
                // Dedicated data register fallback
                if val <= 0xFF {
                    self.gc9307.write_data(val as u8);
                } else if val <= 0xFFFF {
                    self.gc9307.write_data((val >> 8) as u8);
                    self.gc9307.write_data((val & 0xFF) as u8);
                } else {
                    self.gc9307.write_data((val >> 24) as u8);
                    self.gc9307.write_data((val >> 16) as u8);
                    self.gc9307.write_data((val >> 8) as u8);
                    self.gc9307.write_data((val & 0xFF) as u8);
                }
            }
            _ => {}
        }
    }

    pub fn read_spi1(&self, offset: u32) -> u32 {
        match offset {
            0x0C => 1 << 8, // SPI_STS1: TX FIFO Empty / ready
            _ => 0,
        }
    }

    pub fn write_spi1(&mut self, offset: u32, val: u32) {
        match offset {
            0x18 => {
                // SPI_CTL4_REG (D/C select: 0 = Command, 1 = Data)
                self.st7735.dc_mode = (val & 0x1) != 0;
            }
            0x00 => {
                // SPI_TXD_REG: Transmit byte/word based on D/C flag
                if !self.st7735.dc_mode {
                    self.st7735.write_cmd((val & 0xFF) as u8);
                } else if val <= 0xFF {
                    self.st7735.write_data(val as u8);
                } else if val <= 0xFFFF {
                    // 16-bit RGB565 pixel
                    self.st7735.write_data((val >> 8) as u8);
                    self.st7735.write_data((val & 0xFF) as u8);
                } else {
                    // 32-bit dual-pixel pair
                    self.st7735.write_data((val >> 24) as u8);
                    self.st7735.write_data((val >> 16) as u8);
                    self.st7735.write_data((val >> 8) as u8);
                    self.st7735.write_data((val & 0xFF) as u8);
                }
            }
            0x04 => {
                // Dedicated data register fallback
                if val <= 0xFF {
                    self.st7735.write_data(val as u8);
                } else if val <= 0xFFFF {
                    self.st7735.write_data((val >> 8) as u8);
                    self.st7735.write_data((val & 0xFF) as u8);
                } else {
                    self.st7735.write_data((val >> 24) as u8);
                    self.st7735.write_data((val >> 16) as u8);
                    self.st7735.write_data((val >> 8) as u8);
                    self.st7735.write_data((val & 0xFF) as u8);
                }
            }
            _ => {}
        }
    }
}
