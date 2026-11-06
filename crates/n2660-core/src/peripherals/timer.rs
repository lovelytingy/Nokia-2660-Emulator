// SPDX-License-Identifier: MIT
//! Unisoc SPRD System Timer Controller model

#[derive(Debug, Clone)]
pub struct SprdTimer {
    pub load: u32,
    pub value: u32,
    pub ctl: u32,
    pub int_raw: u32,
}

impl Default for SprdTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl SprdTimer {
    pub fn new() -> Self {
        Self {
            load: 0x0000_FFFF,
            value: 0x0000_FFFF,
            ctl: 0,
            int_raw: 0,
        }
    }

    pub fn read(&mut self, offset: u32) -> u32 {
        match offset {
            0x00 => self.load,
            0x04 => {
                let current = self.value;
                // Decrement counter on read to simulate hardware down-counter clock
                if self.value > 0x4E0 {
                    self.value -= 0x4E0;
                } else {
                    self.value = self.load;
                    self.int_raw = 1;
                }
                current
            }
            0x08 => self.ctl,
            0x0C => self.int_raw,
            0x10 => 0,
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u32, val: u32) {
        match offset {
            0x00 => {
                self.load = val;
                self.value = val;
            }
            0x08 => self.ctl = val,
            0x10 => self.int_raw = 0,
            _ => {}
        }
    }
}
