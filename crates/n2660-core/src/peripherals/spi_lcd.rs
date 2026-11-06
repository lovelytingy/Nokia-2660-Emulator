// SPDX-License-Identifier: MIT
//! GalaxyCore GC9307 (SPI0) and Sitronix ST7735 (SPI1) TFT LCD controllers

use crate::types::{GC9307_WIDTH, GC9307_HEIGHT, ST7735_WIDTH, ST7735_HEIGHT};

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
}

pub type Gc9307Controller = LcdController<GC9307_WIDTH, GC9307_HEIGHT>;
pub type St7735Controller = LcdController<ST7735_WIDTH, ST7735_HEIGHT>;

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
                // SPI_TXD_REG: Transmit byte based on D/C flag
                let byte = (val & 0xFF) as u8;
                if !self.gc9307.dc_mode {
                    self.gc9307.write_cmd(byte);
                } else {
                    self.gc9307.write_data(byte);
                }
            }
            0x04 => {
                // Dedicated data register fallback
                self.gc9307.write_data((val & 0xFF) as u8);
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
                // SPI_TXD_REG: Transmit byte based on D/C flag
                let byte = (val & 0xFF) as u8;
                if !self.st7735.dc_mode {
                    self.st7735.write_cmd(byte);
                } else {
                    self.st7735.write_data(byte);
                }
            }
            0x04 => {
                // Dedicated data register fallback
                self.st7735.write_data((val & 0xFF) as u8);
            }
            _ => {}
        }
    }
}
