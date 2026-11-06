// SPDX-License-Identifier: MIT
//! Unisoc SPRD Keypad Matrix Controller model

#[derive(Debug, Clone, Default)]
pub struct SprdKpd {
    pub ctrl: u32,
    pub int_en: u32,
    pub int_raw: u32,
    pub polarity: u32,
    pub clk_div: u32,
    pub status0: u32,
    pub status1: u32,
}

impl SprdKpd {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inject_key(&mut self, key: &str, pressed: bool) -> bool {
        let (is_s0, bit) = match key.to_lowercase().as_str() {
            "0" => (true, 0),
            "1" => (true, 1),
            "2" => (true, 2),
            "3" => (true, 3),
            "4" => (true, 4),
            "5" => (true, 5),
            "6" => (true, 6),
            "7" => (true, 7),
            "8" => (true, 8),
            "9" => (true, 9),
            "*" | "star" => (true, 10),
            "#" | "hash" => (true, 11),
            "up"   => (true, 12),
            "down" => (true, 13),
            "left" => (true, 14),
            "right"=> (true, 15),
            "ok"   => (false, 0),
            "lsk"  => (false, 1),
            "rsk"  => (false, 2),
            "call" => (false, 3),
            "end"  => (false, 4),
            "sos"  => (false, 5),
            "vol_up"   => (false, 6),
            "vol_down" => (false, 7),
            _ => return false,
        };

        if is_s0 {
            if pressed {
                self.status0 |= 1 << bit;
            } else {
                self.status0 &= !(1 << bit);
            }
        } else if pressed {
            self.status1 |= 1 << bit;
        } else {
            self.status1 &= !(1 << bit);
        }

        if pressed {
            self.int_raw |= 0x1;
        }
        true
    }

    pub fn read(&self, offset: u32) -> u32 {
        match offset {
            0x0000 => self.ctrl,
            0x0004 => self.int_en,
            0x0008 => self.int_raw,
            0x000C => self.int_raw & self.int_en,
            0x0010 => 0,
            0x0014 => self.polarity,
            0x0018 => self.clk_div,
            0x001C => self.status0,
            0x0020 => self.status1,
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u32, val: u32) {
        match offset {
            0x0000 => self.ctrl = val,
            0x0004 => self.int_en = val,
            0x0010 => self.int_raw &= !val,
            0x0014 => self.polarity = val,
            0x0018 => self.clk_div = val,
            _ => {}
        }
    }
}
