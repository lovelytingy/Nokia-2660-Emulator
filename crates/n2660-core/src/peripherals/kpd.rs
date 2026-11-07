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
    pub key_status: u32,
    pub last_key_down: bool,
    pub power_button_down: bool,
}

impl SprdKpd {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inject_key(&mut self, key: &str, pressed: bool) -> bool {
        // Handle dedicated PMIC EIC Power / End key
        if matches!(key.to_lowercase().as_str(), "end" | "power" | "clear" | "x") {
            self.power_button_down = pressed;
        }

        // Authentic Nokia 2660 Flip 5x5 Keypad Matrix (col, row)
        let matrix_coords = match key.to_lowercase().as_str() {
            "0" => Some((1, 0)),
            "1" => Some((0, 2)),
            "2" => Some((1, 2)),
            "3" => Some((1, 1)),
            "4" => Some((0, 4)),
            "5" => Some((1, 4)),
            "6" => Some((3, 2)),
            "7" => Some((0, 1)),
            "8" => Some((2, 2)),
            "9" => Some((3, 1)),
            "*" | "star" => Some((0, 0)),
            "#" | "hash" => Some((3, 0)),
            "up"   => Some((2, 4)),
            "down" => Some((1, 3)),
            "left" => Some((2, 1)),
            "right"=> Some((3, 3)),
            "ok" | "enter" | "return" | "select" => Some((2, 3)),
            "lsk" | "soft1" | "menu"             => Some((4, 0)),
            "rsk" | "soft2" | "names"            => Some((4, 1)),
            "shortcut" | "apps" | "app"          => Some((2, 0)),
            "back"                               => Some((3, 4)),
            "call" | "c" | "dial"                => Some((0, 3)),
            "sos"                                => Some((4, 2)),
            "vol_up" | "volup" | "+"             => Some((4, 3)),
            "vol_down" | "voldown" | "-"         => Some((4, 4)),
            _ => None,
        };

        if let Some((col, row)) = matrix_coords {
            self.key_status = ((row as u32) << 4) | (col as u32);
            if pressed {
                self.int_raw = 0x01; // Bit 0 = key down
                self.last_key_down = true;
            } else {
                self.int_raw = 0x10; // Bit 4 = key up
                self.last_key_down = false;
            }
        }

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
            "ok" | "enter" | "return" | "select"   => (false, 0),
            "lsk" | "soft1" | "menu"               => (false, 1),
            "rsk" | "soft2" | "names"              => (false, 2),
            "shortcut" | "apps" | "app"            => (false, 8),
            "back"                                 => (false, 9),
            "call" | "c" | "dial"                  => (false, 3),
            "end" | "x" | "power" | "clear"        => (false, 4),
            "sos"                                  => (false, 5),
            "vol_up" | "volup" | "+"               => (false, 6),
            "vol_down" | "voldown" | "-"           => (false, 7),
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
            0x002C => self.key_status,
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
