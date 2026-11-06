// SPDX-License-Identifier: MIT
//! Unisoc SPRD 32-Line Interrupt Controller (INTC) model

#[derive(Debug, Clone, Default)]
pub struct SprdIntc {
    pub irq_raw_sts: u32,
    pub irq_en: u32,
    pub irq_soft: u32,
    pub fiq_raw_sts: u32,
    pub fiq_en: u32,
    pub fiq_soft: u32,
    pub int_type: u32,
}

impl SprdIntc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_irq_masked_status(&self) -> u32 {
        let lines = (self.irq_raw_sts | self.irq_soft) & !self.int_type;
        lines & self.irq_en
    }

    pub fn get_fiq_masked_status(&self) -> u32 {
        let lines = (self.fiq_raw_sts | self.fiq_soft) & self.int_type;
        lines & self.fiq_en
    }

    pub fn assert_irq(&mut self, irq: u32) {
        if irq < 32 {
            self.irq_raw_sts |= 1 << irq;
        }
    }

    pub fn deassert_irq(&mut self, irq: u32) {
        if irq < 32 {
            self.irq_raw_sts &= !(1 << irq);
        }
    }

    pub fn read(&self, offset: u32) -> u32 {
        match offset {
            0x00 => self.irq_raw_sts,
            0x04 => self.get_irq_masked_status(),
            0x08 | 0x0C => self.irq_en,
            0x10 | 0x14 => self.irq_soft,
            0x1C => self.fiq_raw_sts,
            0x20 => self.get_fiq_masked_status(),
            0x24 | 0x28 => self.fiq_en,
            0x2C | 0x30 => self.fiq_soft,
            0x34 => self.int_type,
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u32, val: u32) {
        match offset {
            0x08 => self.irq_en |= val,
            0x0C => self.irq_en &= !val,
            0x10 => {
                self.irq_soft |= val;
                self.irq_raw_sts |= val;
            }
            0x14 => {
                self.irq_soft &= !val;
                self.irq_raw_sts &= !val;
            }
            0x18 => {
                self.irq_raw_sts &= !val;
            }
            0x24 => self.fiq_en |= val,
            0x28 => self.fiq_en &= !val,
            0x2C => {
                self.fiq_soft |= val;
                self.fiq_raw_sts |= val;
            }
            0x30 => {
                self.fiq_soft &= !val;
                self.fiq_raw_sts &= !val;
            }
            0x34 => self.int_type = val,
            _ => {}
        }
    }
}
