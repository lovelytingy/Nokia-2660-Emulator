// SPDX-License-Identifier: MIT
//! Unisoc SPRD BOOT0 Header & BootROM Reset Vector Loader

use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;

pub const SPRD_MAGIC: u32 = 0x5350_5244; // "SPRD"
pub const SPRD_MAGIC_BIMG: u32 = 0x4249_4D47; // "BIMG"
pub const SPRD_BOOT0_VERSION: u32 = 1;
pub const DEFAULT_LOAD_ADDR: u32 = 0x0000_6200;
pub const HEADER_SIZE: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprdHeader {
    pub magic: u32,
    pub version: u32,
    pub payload_size: u32,
    pub load_addr: u32,
    pub entry_point: u32,
    pub checksum: u32,
    pub header_checksum: u32,
    pub flags: u32,
}

impl SprdHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < HEADER_SIZE {
            return Err("SPRD header too short");
        }
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != SPRD_MAGIC && magic != SPRD_MAGIC_BIMG {
            return Err("Invalid SPRD magic");
        }
        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let payload_size = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let load_addr = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let entry_point = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let checksum = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        let header_checksum = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let flags = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);

        Ok(Self {
            magic,
            version,
            payload_size,
            load_addr,
            entry_point,
            checksum,
            header_checksum,
            flags,
        })
    }

    pub fn calculate_checksum(data: &[u8]) -> u32 {
        let mut csum: u32 = 0;
        let mut chunks = data.chunks_exact(4);
        for chunk in &mut chunks {
            let w = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            csum = csum.wrapping_add(w);
        }
        let rem = chunks.remainder();
        if !rem.is_empty() {
            let mut buf = [0u8; 4];
            buf[..rem.len()].copy_from_slice(rem);
            csum = csum.wrapping_add(u32::from_le_bytes(buf));
        }
        csum
    }

    pub fn create(payload: &[u8], load_addr: u32, entry_point: u32) -> Vec<u8> {
        let mut out = vec![0u8; HEADER_SIZE + payload.len()];
        let magic = SPRD_MAGIC;
        let version = SPRD_BOOT0_VERSION;
        let payload_size = payload.len() as u32;
        let checksum = Self::calculate_checksum(payload);

        let words = [magic, version, payload_size, load_addr, entry_point, checksum];
        let mut header_checksum: u32 = 0;
        for (i, &w) in words.iter().enumerate() {
            out[i * 4..(i + 1) * 4].copy_from_slice(&w.to_le_bytes());
            header_checksum = header_checksum.wrapping_add(w);
        }
        out[24..28].copy_from_slice(&header_checksum.to_le_bytes());
        out[28..32].fill(0); // flags / reserved
        out[32..].copy_from_slice(payload);
        out
    }

    pub fn inject_into_flash<P: AsRef<Path>>(
        boot0_data: &[u8],
        flash_path: P,
        block_index: usize,
        flash_size: usize,
    ) -> io::Result<()> {
        let block_offset = block_index * 128 * 1024;
        let path = flash_path.as_ref();
        let mut file = if path.exists() {
            OpenOptions::new().read(true).write(true).open(path)?
        } else {
            let f = File::create(path)?;
            f.set_len(flash_size as u64)?;
            OpenOptions::new().read(true).write(true).open(path)?
        };

        file.seek(SeekFrom::Start(block_offset as u64))?;
        file.write_all(boot0_data)?;
        Ok(())
    }
}
