// SPDX-License-Identifier: MIT
//! Unisoc SPRD GPIO Controller model (Pin 0 = Hall effect flip sensor)

#[derive(Debug, Clone)]
pub struct SprdGpio {
    pub data_out: u32,
    pub dir: u32,
    pub data_in: u32,
    pub inen: u32,
}

impl Default for SprdGpio {
    fn default() -> Self {
        Self::new(true)
    }
}

impl SprdGpio {
    pub fn new(flip_open: bool) -> Self {
        Self {
            data_out: 0,
            dir: 0,
            data_in: if flip_open { 1 } else { 0 },
            inen: 0,
        }
    }

    pub fn set_flip_state(&mut self, is_open: bool) {
        if is_open {
            self.data_in |= 0x1;
        } else {
            self.data_in &= !0x1;
        }
    }

    pub fn is_flip_open(&self) -> bool {
        (self.data_in & 0x1) != 0
    }

    pub fn read(&self, offset: u32) -> u32 {
        match offset {
            0x00 => (self.data_out & self.dir) | (self.data_in & !self.dir),
            0x04 | 0x08 => self.dir,
            0x28 => self.inen,
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u32, val: u32) {
        match offset {
            0x00 => self.data_out = val,
            0x04 | 0x08 => self.dir = val,
            0x28 => self.inen = val,
            _ => {}
        }
    }
}
