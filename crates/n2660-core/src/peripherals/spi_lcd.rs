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
    pub id_read_idx: usize,
    pub streamed_pixels_count: usize,
    pub has_custom_framebuffer: bool,
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
            id_read_idx: 0,
            streamed_pixels_count: 0,
            has_custom_framebuffer: false,
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
                    self.streamed_pixels_count += 1;
                    if self.streamed_pixels_count >= 1000 {
                        self.has_custom_framebuffer = true;
                    }
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

    /// Loads and decodes an authentic vendor splash bitmap into the framebuffer.
    ///
    /// Supports Spreadtrum RLE16 compressed bitmaps (`compression = 0x00001003`) from
    /// Unisoc `mmi_res` packages, as well as standard uncompressed RGB565 BMPs.
    /// Correctly honors standard BMP bottom-up scanline ordering (`biHeight > 0`),
    /// automatically handles center-cropping (e.g. 128x160 logo onto 120x160 sub-display)
    /// or center-fitting, and marks `has_custom_framebuffer = true`.
    pub fn load_splash(&mut self, bmp_data: &[u8]) -> bool {
        let (bmp_w, bmp_h, is_bottom_up) = if bmp_data.len() >= 26 && &bmp_data[0..2] == b"BM" {
            let w = (i32::from_le_bytes([bmp_data[18], bmp_data[19], bmp_data[20], bmp_data[21]])).unsigned_abs() as usize;
            let raw_h = i32::from_le_bytes([bmp_data[22], bmp_data[23], bmp_data[24], bmp_data[25]]);
            let h = raw_h.unsigned_abs() as usize;
            let bottom_up = raw_h > 0;
            if w > 0 && h > 0 { (w, h, bottom_up) } else { (W, H, true) }
        } else {
            (W, H, true)
        };

        if let Some(pixels) = decode_sprd_rle16(bmp_data, bmp_w, bmp_h) {
            if bmp_w == W && bmp_h == H {
                for sy in 0..H {
                    let stream_y = if is_bottom_up { H - 1 - sy } else { sy };
                    for sx in 0..W {
                        self.framebuffer[sy][sx] = pixels[stream_y * W + sx];
                    }
                }
            } else if bmp_w >= W && bmp_h >= H {
                // Center-crop (e.g. 128x160 into 120x160)
                let x_off = (bmp_w - W) / 2;
                let y_off = (bmp_h - H) / 2;
                for sy in 0..H {
                    let stream_y = if is_bottom_up {
                        (bmp_h - 1) - (sy + y_off)
                    } else {
                        sy + y_off
                    };
                    for sx in 0..W {
                        self.framebuffer[sy][sx] = pixels[stream_y * bmp_w + (sx + x_off)];
                    }
                }
            } else {
                // Center-fit with zero-padding
                self.framebuffer.fill([0u16; W]);
                let x_off = (W.saturating_sub(bmp_w)) / 2;
                let y_off = (H.saturating_sub(bmp_h)) / 2;
                for img_y in 0..bmp_h.min(H) {
                    let stream_y = if is_bottom_up { (bmp_h - 1) - img_y } else { img_y };
                    let sy = y_off + img_y;
                    for sx in 0..bmp_w.min(W) {
                        self.framebuffer[sy][x_off + sx] = pixels[stream_y * bmp_w + sx];
                    }
                }
            }
            self.has_custom_framebuffer = true;
            true
        } else {
            false
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
        if self.has_custom_framebuffer {
            return;
        }
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
                self.draw_text(16, 151, "* CP15 SCTLR & Banked Stacks: OK", COLOR_WHITE, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 169, "* C Runtime __scatterload Done", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 187, "* ThreadX Preemptive Scheduler: OK", COLOR_CYAN, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 205, "* Task Manager: MS_MMI_Main Ready", COLOR_YELLOW, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 223, sub_status, COLOR_WHITE, COLOR_DARK_NAVY, 1);
            }
            BootStage::User => {
                self.draw_text(16, 133, "[STAGE 4/4] USERLAND MMI FRAMEWORK", COLOR_CYAN, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 153, "* MS_MMI_Main Application Started", COLOR_WHITE, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 171, "* UI Window Manager Initialized", COLOR_LIGHT_GRAY, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 189, "* Cellular VoLTE / SIM Stack Active", COLOR_GREEN, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 207, "* Awaiting Framebuffer Presentation", COLOR_YELLOW, COLOR_DARK_NAVY, 1);
                self.draw_text(16, 225, sub_status, COLOR_WHITE, COLOR_DARK_NAVY, 1);
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
            let dial_scale = if dial_str.len() <= 10 { 3 } else { 2 };
            let dial_y = if dial_scale == 3 { 206 } else { 210 };
            self.draw_text(20, dial_y, dial_str, COLOR_WHITE, COLOR_DARK_NAVY, dial_scale);
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
    pub fn render_cover_screen(&mut self, _stage: Option<BootStage>, _is_open: bool) {
        // Procedural mock UI eliminated: outer cover display strictly displays authentic firmware output
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

    /// Transfers active firmware display output from primary screen (GC9307 240x320)
    /// to outer cover screen (ST7735 120x160) using a smooth 2:1 RGB565 box-filter.
    pub fn transfer_main_to_sub(&mut self) {
        for sy in 0..ST7735_HEIGHT {
            let my0 = sy * 2;
            let my1 = (my0 + 1).min(GC9307_HEIGHT - 1);
            for sx in 0..ST7735_WIDTH {
                let mx0 = sx * 2;
                let mx1 = (mx0 + 1).min(GC9307_WIDTH - 1);

                let p00 = self.gc9307.framebuffer[my0][mx0];
                let p01 = self.gc9307.framebuffer[my0][mx1];
                let p10 = self.gc9307.framebuffer[my1][mx0];
                let p11 = self.gc9307.framebuffer[my1][mx1];

                let r = (((p00 >> 11) & 0x1F) + ((p01 >> 11) & 0x1F) + ((p10 >> 11) & 0x1F) + ((p11 >> 11) & 0x1F) + 2) / 4;
                let g = (((p00 >> 5) & 0x3F) + ((p01 >> 5) & 0x3F) + ((p10 >> 5) & 0x3F) + ((p11 >> 5) & 0x3F) + 2) / 4;
                let b = ((p00 & 0x1F) + (p01 & 0x1F) + (p10 & 0x1F) + (p11 & 0x1F) + 2) / 4;

                self.st7735.framebuffer[sy][sx] = ((r & 0x1F) << 11) | ((g & 0x3F) << 5) | (b & 0x1F);
            }
        }
        self.st7735.has_custom_framebuffer = true;
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

/// Decodes Spreadtrum RLE16 compressed bitmaps (`compression = 0x00001003`) or standard uncompressed 16-bit BMPs.
///
/// In Spreadtrum RLE16:
/// - Commands consist of pairs of bytes (A, B).
/// - If A == 0 and B == 0: end of line / record separator marker.
/// - 'A' uncompressed literal 16-bit RGB565 pixels follow in the byte stream.
/// - Next is 1 pixel value (2 bytes), repeated 'B' times.
/// - Total pixels decoded per command = A + B.
pub fn decode_sprd_rle16(data: &[u8], width: usize, height: usize) -> Option<Vec<u16>> {
    let total = width * height;
    if data.len() < 14 {
        return None;
    }

    let (data_offset, compression, bpp, bmp_w, bmp_h) = if &data[0..2] == b"BM" {
        let off = if data.len() >= 14 {
            u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize
        } else {
            0x42
        };
        let (comp, bpp, bw, bh) = if data.len() >= 34 {
            let bw = (i32::from_le_bytes([data[18], data[19], data[20], data[21]])).unsigned_abs() as usize;
            let bh = (i32::from_le_bytes([data[22], data[23], data[24], data[25]])).unsigned_abs() as usize;
            let bpp = u16::from_le_bytes([data[28], data[29]]);
            let comp = u32::from_le_bytes([data[30], data[31], data[32], data[33]]);
            (comp, bpp, bw, bh)
        } else {
            (0x1003, 16, width, height)
        };
        (off, comp, bpp, bw, bh)
    } else {
        (0, 0x1003, 16, width, height)
    };

    if data_offset > data.len() {
        return None;
    }
    let body = &data[data_offset..];
    let expected_pixels = if bmp_w > 0 && bmp_h > 0 {
        bmp_w * bmp_h
    } else {
        total
    };

    if compression == 0x1003 {
        let mut pixels = Vec::with_capacity(expected_pixels);
        let mut i = 0;

        while i + 1 < body.len() && pixels.len() < expected_pixels {
            let a = body[i] as usize;
            let b = body[i + 1] as usize;
            i += 2;

            if a == 0 && b == 0 {
                continue;
            }

            // 'a' literal 16-bit pixels follow
            for _ in 0..a {
                if i + 1 < body.len() && pixels.len() < expected_pixels {
                    let px = u16::from_le_bytes([body[i], body[i + 1]]);
                    pixels.push(px);
                    i += 2;
                }
            }

            // Next is 1 pixel value repeated 'b' times
            if b > 0 && i + 1 < body.len() {
                let px = u16::from_le_bytes([body[i], body[i + 1]]);
                i += 2;
                for _ in 0..b {
                    if pixels.len() < expected_pixels {
                        pixels.push(px);
                    }
                }
            }
        }

        if pixels.len() < expected_pixels {
            pixels.resize(expected_pixels, 0);
        }
        Some(pixels)
    } else if (compression == 0 || compression == 3) && bpp == 16 {
        let mut pixels = Vec::with_capacity(expected_pixels);
        for chunk in body.chunks_exact(2).take(expected_pixels) {
            pixels.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        if pixels.len() < expected_pixels {
            pixels.resize(expected_pixels, 0);
        }
        Some(pixels)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprd_rle16_synthetic_decode() {
        // Build a synthetic BMP with RLE16 stream
        let mut bmp = Vec::new();
        bmp.extend_from_slice(b"BM");
        let file_size: u32 = 0x42 + 10;
        bmp.extend_from_slice(&file_size.to_le_bytes()); // size
        bmp.extend_from_slice(&[0u8; 4]); // reserved
        bmp.extend_from_slice(&0x42u32.to_le_bytes()); // data offset = 0x42 (66)
        // DIB header (BITMAPINFOHEADER + bitmasks)
        bmp.extend_from_slice(&40u32.to_le_bytes()); // header size
        bmp.extend_from_slice(&4i32.to_le_bytes()); // width = 4
        bmp.extend_from_slice(&2i32.to_le_bytes()); // height = 2
        bmp.extend_from_slice(&1u16.to_le_bytes()); // planes = 1
        bmp.extend_from_slice(&16u16.to_le_bytes()); // bpp = 16
        bmp.extend_from_slice(&0x1003u32.to_le_bytes()); // compression = 0x1003
        bmp.extend_from_slice(&[0u8; 20]); // image size, etc.
        bmp.extend_from_slice(&[0u8; 12]); // RGB565 masks (pad to 66 bytes)
        assert_eq!(bmp.len(), 0x42);

        // Body: 2 literal pixels (0x1234, 0x5678), then 2 repeats of 0xAAAA
        // Command 1: a = 2, b = 2
        bmp.push(2); // a
        bmp.push(2); // b
        bmp.extend_from_slice(&0x1234u16.to_le_bytes());
        bmp.extend_from_slice(&0x5678u16.to_le_bytes());
        bmp.extend_from_slice(&0xAAAAu16.to_le_bytes());
        // Command 2: a = 0, b = 4 of 0xFFFF
        bmp.push(0); // a
        bmp.push(4); // b
        bmp.extend_from_slice(&0xFFFFu16.to_le_bytes());

        let decoded = decode_sprd_rle16(&bmp, 4, 2).expect("decode failed");
        assert_eq!(decoded.len(), 8);
        assert_eq!(decoded[0], 0x1234);
        assert_eq!(decoded[1], 0x5678);
        assert_eq!(decoded[2], 0xAAAA);
        assert_eq!(decoded[3], 0xAAAA);
        assert_eq!(decoded[4], 0xFFFF);
        assert_eq!(decoded[5], 0xFFFF);
        assert_eq!(decoded[6], 0xFFFF);
        assert_eq!(decoded[7], 0xFFFF);

        let mut controller = Gc9307Controller::new();
        assert!(!controller.has_custom_framebuffer);
        let res = controller.load_splash(&bmp);
        assert!(res);
        assert!(controller.has_custom_framebuffer);
        let x_off = (GC9307_WIDTH - 4) / 2;
        let y_off = (GC9307_HEIGHT - 2) / 2;
        // With bottom-up BMPs, stream row 1 (0xFFFF) is top (y_off), stream row 0 (0x1234) is bottom (y_off + 1)
        assert_eq!(controller.framebuffer[y_off][x_off], 0xFFFF);
        assert_eq!(controller.framebuffer[y_off + 1][x_off], 0x1234);
        assert_eq!(controller.framebuffer[y_off + 1][x_off + 1], 0x5678);
        assert_eq!(controller.framebuffer[y_off + 1][x_off + 2], 0xAAAA);
        assert_eq!(controller.framebuffer[y_off + 1][x_off + 3], 0xAAAA);
    }
}
