// SPDX-License-Identifier: MIT
//! Unisoc SPRD Miscellaneous Peripherals (ADI/PMIC, Clock Controller, PIN/IOMUX, Secondary UARTs)

use std::collections::HashMap;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct SprdMisc {
    pub registers: HashMap<(u32, u32), u32>,
    pub battery_mv: u32,
}

impl Default for SprdMisc {
    fn default() -> Self {
        Self::new()
    }
}

impl SprdMisc {
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
            battery_mv: 3900, // 3.9V nominal healthy Li-ion battery (~85% capacity)
        }
    }

    pub fn read(&mut self, base: u32, offset: u32) -> u32 {
        match base {
            SPRD_ADI_BASE => {
                match offset {
                    0x04 => self.battery_mv, // ADI read data: return battery voltage
                    0x08 => 0x0000_0003,     // ADI status: Bit 0 (FIFO empty) | Bit 1 (RD_DONE)
                    _ => *self.registers.get(&(base, offset)).unwrap_or(&0),
                }
            }
            SPRD_CLK_BASE => {
                // Return 0xFFFFFFFF (enabled) for clock gating registers if unwritten
                *self.registers.get(&(base, offset)).unwrap_or(&0xFFFF_FFFF)
            }
            _ => *self.registers.get(&(base, offset)).unwrap_or(&0),
        }
    }

    pub fn write(&mut self, base: u32, offset: u32, val: u32) {
        self.registers.insert((base, offset), val);
    }
}
