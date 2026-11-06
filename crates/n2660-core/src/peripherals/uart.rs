// SPDX-License-Identifier: MIT
//! Unisoc SPRD UART0 Controller model

use std::collections::VecDeque;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct SprdUart {
    pub tx_buffer: Vec<u8>,
    pub rx_fifo: VecDeque<u8>,
    pub ier: u32,
    pub clkd0: u32,
    pub clkd1: u32,
    pub print_stdout: bool,
}

impl Default for SprdUart {
    fn default() -> Self {
        Self::new(false)
    }
}

impl SprdUart {
    pub fn new(print_stdout: bool) -> Self {
        Self {
            tx_buffer: Vec::with_capacity(1024),
            rx_fifo: VecDeque::new(),
            ier: 0,
            clkd0: 0,
            clkd1: 0,
            print_stdout,
        }
    }

    pub fn read(&mut self, offset: u32) -> u32 {
        match offset {
            0x00 => 0, // TXD
            0x04 => {  // RXD
                self.rx_fifo.pop_front().map(|b| b as u32).unwrap_or(0)
            }
            0x08 => {  // STS0
                let mut sts = 0x08; // TX FIFO empty
                if !self.rx_fifo.is_empty() {
                    sts |= 0x01; // RX FIFO not empty
                }
                sts
            }
            0x0C => {  // STS1
                (self.rx_fifo.len() as u32) & 0xFF
            }
            0x10 => self.ier,
            0x14 => 0, // ICR
            0x18 => self.clkd0,
            0x1C => self.clkd1,
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u32, val: u32) {
        match offset {
            0x00 => { // TXD
                let ch = (val & 0xFF) as u8;
                self.tx_buffer.push(ch);
                if self.print_stdout {
                    let _ = io::stdout().write_all(&[ch]);
                    let _ = io::stdout().flush();
                }
            }
            0x10 => self.ier = val,
            0x18 => self.clkd0 = val,
            0x1C => self.clkd1 = val,
            _ => {}
        }
    }

    pub fn inject_str(&mut self, s: &str) {
        for b in s.bytes() {
            self.rx_fifo.push_back(b);
        }
    }
}
