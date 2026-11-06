// SPDX-License-Identifier: MIT
//! Unisoc SFC (Serial Flash Controller) and 128 MB SPI NAND model (Winbond W25N01GV)

pub const NAND_PAGE_SIZE: usize = 2048;
pub const NAND_SPARE_SIZE: usize = 64;
pub const NAND_PAGE_TOTAL_SIZE: usize = NAND_PAGE_SIZE + NAND_SPARE_SIZE; // 2112
pub const NAND_PAGES_PER_BLOCK: usize = 64;
pub const NAND_BLOCK_SIZE: usize = NAND_PAGE_SIZE * NAND_PAGES_PER_BLOCK; // 128 KB
pub const NAND_BLOCK_COUNT: usize = 1024;
pub const NAND_TOTAL_SIZE: usize = NAND_BLOCK_SIZE * NAND_BLOCK_COUNT; // 128 MB

#[derive(Clone)]
pub struct SprdSfc {
    pub cmd: u32,
    pub addr: u32,
    pub ctl: u32,
    pub sts: u32,
    pub buf_ptr: usize,
    pub page_buffer: Vec<u8>,
    pub storage: Vec<u8>,
    pub pending_flash_sync: Option<(usize, Vec<u8>)>,
}

impl Default for SprdSfc {
    fn default() -> Self {
        Self::new()
    }
}

impl SprdSfc {
    pub fn new() -> Self {
        Self {
            cmd: 0,
            addr: 0,
            ctl: 0,
            sts: 0,
            buf_ptr: 0,
            page_buffer: vec![0xFF; NAND_PAGE_TOTAL_SIZE],
            storage: vec![0xFF; NAND_TOTAL_SIZE],
            pending_flash_sync: None,
        }
    }

    pub fn load_image(&mut self, image_bytes: &[u8]) {
        let copy_len = image_bytes.len().min(self.storage.len());
        self.storage[..copy_len].copy_from_slice(&image_bytes[..copy_len]);
    }

    pub fn write_page(&mut self, block: usize, page: usize, data: &[u8]) {
        let flash_offset = (block * 64 + page) * NAND_PAGE_SIZE;
        if flash_offset < self.storage.len() {
            let end = (flash_offset + data.len()).min(self.storage.len());
            self.storage[flash_offset..end].copy_from_slice(&data[..end - flash_offset]);
        }
    }

    pub fn read(&mut self, offset: u32) -> u32 {
        match offset {
            0x0000 => self.cmd,
            0x0004 => self.addr,
            0x0008 => {
                // SFC_DATA (32-bit word read)
                if self.buf_ptr + 4 <= self.page_buffer.len() {
                    let val = u32::from_le_bytes([
                        self.page_buffer[self.buf_ptr],
                        self.page_buffer[self.buf_ptr + 1],
                        self.page_buffer[self.buf_ptr + 2],
                        self.page_buffer[self.buf_ptr + 3],
                    ]);
                    self.buf_ptr += 4;
                    val
                } else if self.buf_ptr < self.page_buffer.len() {
                    let val = self.page_buffer[self.buf_ptr] as u32;
                    self.buf_ptr += 1;
                    val
                } else {
                    0xFFFF_FFFF
                }
            }
            0x000C => self.ctl,
            0x0010 => self.sts,
            0x0014 => self.buf_ptr as u32,
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u32, val: u32) {
        match offset {
            0x0000 => {
                self.cmd = val & 0xFF;
                self.execute_cmd(self.cmd);
            }
            0x0004 => self.addr = val,
            0x0008 => {
                // SFC_DATA (32-bit word write)
                if self.buf_ptr + 4 <= self.page_buffer.len() {
                    let bytes = val.to_le_bytes();
                    self.page_buffer[self.buf_ptr..self.buf_ptr + 4].copy_from_slice(&bytes);
                    self.buf_ptr += 4;
                }
            }
            0x000C => self.ctl = val,
            0x0014 => self.buf_ptr = (val as usize) % NAND_PAGE_TOTAL_SIZE,
            _ => {}
        }
    }

    fn execute_cmd(&mut self, cmd: u32) {
        match cmd & 0xFF {
            0xFF => {
                // NAND_CMD_RESET
                self.buf_ptr = 0;
                self.page_buffer.fill(0xFF);
            }
            0x9F => {
                // NAND_CMD_READ_ID: Winbond W25N01GV (0x00EFAA21)
                self.buf_ptr = 0;
                self.page_buffer[0] = 0xEF; // Mfr Winbond
                self.page_buffer[1] = 0xAA; // Dev Hi
                self.page_buffer[2] = 0x21; // Dev Lo
                self.page_buffer[3] = 0x00;
            }
            0x13 => {
                // NAND_CMD_PAGE_READ
                let page_idx = self.addr as usize;
                let flash_offset = page_idx * NAND_PAGE_SIZE;
                self.buf_ptr = 0;
                if flash_offset + NAND_PAGE_SIZE <= self.storage.len() {
                    self.page_buffer[..NAND_PAGE_SIZE]
                        .copy_from_slice(&self.storage[flash_offset..flash_offset + NAND_PAGE_SIZE]);
                } else {
                    self.page_buffer[..NAND_PAGE_SIZE].fill(0xFF);
                }
            }
            0x03 | 0x0B => {
                // NAND_CMD_READ_CACHE
                self.buf_ptr = (self.addr as usize) % NAND_PAGE_TOTAL_SIZE;
            }
            0x02 | 0x84 => {
                // NAND_CMD_PROGRAM_LOAD
                self.buf_ptr = (self.addr as usize) % NAND_PAGE_TOTAL_SIZE;
            }
            0x10 => {
                // NAND_CMD_PROGRAM_EXECUTE
                let page_idx = self.addr as usize;
                let flash_offset = page_idx * NAND_PAGE_SIZE;
                if flash_offset + NAND_PAGE_SIZE <= self.storage.len() {
                    self.storage[flash_offset..flash_offset + NAND_PAGE_SIZE]
                        .copy_from_slice(&self.page_buffer[..NAND_PAGE_SIZE]);
                    self.pending_flash_sync = Some((
                        flash_offset,
                        self.page_buffer[..NAND_PAGE_SIZE].to_vec(),
                    ));
                }
            }
            0xD8 => {
                // NAND_CMD_BLOCK_ERASE
                let block_idx = self.addr as usize;
                let flash_offset = block_idx * NAND_BLOCK_SIZE;
                if flash_offset + NAND_BLOCK_SIZE <= self.storage.len() {
                    self.storage[flash_offset..flash_offset + NAND_BLOCK_SIZE].fill(0xFF);
                    self.pending_flash_sync = Some((
                        flash_offset,
                        vec![0xFF; NAND_BLOCK_SIZE],
                    ));
                }
            }
            _ => {}
        }
    }
}
