// SPDX-License-Identifier: MIT
//! Unisoc Spreadtrum .PAC Firmware Container parser, extractor, and packer

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

pub const PAC_HEADER_SIZE: usize = 2124;
pub const PAC_RECORD_SIZE: usize = 2124;
pub const PAC_MAGIC: u32 = 0xFFFA_FFFA;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacFileRecord {
    pub index: usize,
    pub id: String,
    pub filename: String,
    pub version: String,
    pub size: usize,
    pub offset: usize,
    pub load_addr: u32,
    pub active: bool,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacArchive {
    pub pac_version: String,
    pub product_name: String,
    pub product_version: String,
    pub flash_type: u32,
    pub total_size: usize,
    pub files: Vec<PacFileRecord>,
}

impl Default for PacArchive {
    fn default() -> Self {
        Self::new("Nokia 2660 Flip", "TA-1480")
    }
}

impl PacArchive {
    pub fn new(product_name: &str, product_version: &str) -> Self {
        Self {
            pac_version: "BP_R1.0.0".to_string(),
            product_name: product_name.to_string(),
            product_version: product_version.to_string(),
            flash_type: 3, // SPI NAND
            total_size: 0,
            files: Vec::new(),
        }
    }

    pub fn add_file(&mut self, id: &str, filename: &str, load_addr: u32, data: Vec<u8>) {
        let size = data.len();
        let index = self.files.len();
        self.files.push(PacFileRecord {
            index,
            id: id.to_string(),
            filename: filename.to_string(),
            version: "".to_string(),
            size,
            offset: 0,
            load_addr,
            active: true,
            data,
        });
    }

    pub fn pack(&self) -> Vec<u8> {
        let file_count = self.files.len();
        let file_table_offset = PAC_HEADER_SIZE;
        let file_table_size = file_count * PAC_RECORD_SIZE;
        let payload_start_offset = file_table_offset + file_table_size;

        let mut current_offset = payload_start_offset;
        let mut prepared_files: Vec<(usize, usize)> = Vec::with_capacity(file_count); // (offset, aligned_size)
        for f in &self.files {
            let sz = f.data.len();
            let off = if sz > 0 { current_offset } else { 0 };
            let aligned = (sz + 3) & !3;
            prepared_files.push((off, aligned));
            if sz > 0 {
                current_offset += aligned;
            }
        }
        let total_pac_size = current_offset;

        let mut out = vec![0u8; total_pac_size];

        // 1. Build Main PAC Header
        encode_utf16le_into(&self.pac_version, &mut out[0..96]);
        out[96..100].copy_from_slice(&(total_pac_size as u32).to_le_bytes());
        encode_utf16le_into(&self.product_name, &mut out[100..612]);
        encode_utf16le_into(&self.product_version, &mut out[612..1124]);
        out[1124..1128].copy_from_slice(&(file_count as u32).to_le_bytes());
        out[1128..1132].copy_from_slice(&(file_table_offset as u32).to_le_bytes());
        out[1132..1136].copy_from_slice(&0u32.to_le_bytes()); // dwMode
        out[1136..1140].copy_from_slice(&self.flash_type.to_le_bytes());
        out[1952..1956].copy_from_slice(&PAC_MAGIC.to_le_bytes());

        // 2. Build Records
        for (i, f) in self.files.iter().enumerate() {
            let rec_start = file_table_offset + i * PAC_RECORD_SIZE;
            let rec_end = rec_start + PAC_RECORD_SIZE;
            let rec = &mut out[rec_start..rec_end];

            rec[0..4].copy_from_slice(&(PAC_RECORD_SIZE as u32).to_le_bytes());
            encode_utf16le_into(&f.id, &mut rec[4..516]);
            encode_utf16le_into(&f.filename, &mut rec[516..1028]);
            encode_utf16le_into(&f.version, &mut rec[1028..1540]);

            let sz64 = f.data.len() as u64;
            let lo_size = (sz64 & 0xFFFF_FFFF) as u32;
            let hi_size = (sz64 >> 32) as u32;
            rec[1540..1544].copy_from_slice(&lo_size.to_le_bytes());
            rec[1544..1548].copy_from_slice(&hi_size.to_le_bytes());

            let flag: u32 = if f.data.is_empty() { 0 } else { 1 };
            rec[1548..1552].copy_from_slice(&flag.to_le_bytes()); // nFileFlag
            rec[1552..1556].copy_from_slice(&0u32.to_le_bytes()); // nCheckFlag

            let (off, _) = prepared_files[i];
            rec[1556..1560].copy_from_slice(&(off as u32).to_le_bytes()); // dwDataOffset
            rec[1560..1564].copy_from_slice(&0u32.to_le_bytes()); // dwCanOmitFlag

            let addr_num: u32 = if f.load_addr != 0 { 1 } else { 0 };
            rec[1564..1568].copy_from_slice(&addr_num.to_le_bytes());
            rec[1568..1572].copy_from_slice(&f.load_addr.to_le_bytes());

            // 3. Copy Payload
            if !f.data.is_empty() && off > 0 {
                out[off..off + f.data.len()].copy_from_slice(&f.data);
            }
        }

        out
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 1024 {
            return Err("File too small for PAC archive");
        }

        // Detect header format (Standard 48-byte vs 96-byte version field)
        let (pac_version, total_size, product_name, product_version, file_count, file_offset, flash_type) = {
            let std_file_count = if bytes.len() >= 1080 {
                u32::from_le_bytes([bytes[1076], bytes[1077], bytes[1078], bytes[1079]]) as usize
            } else { 0 };
            let std_file_offset = if bytes.len() >= 1084 {
                u32::from_le_bytes([bytes[1080], bytes[1081], bytes[1082], bytes[1083]]) as usize
            } else { 0 };

            if std_file_count > 0 && std_file_count <= 256 && std_file_offset < bytes.len() {
                let v = decode_utf16le(&bytes[0..48]);
                let sz = u32::from_le_bytes([bytes[48], bytes[49], bytes[50], bytes[51]]) as usize;
                let prd = decode_utf16le(&bytes[52..564]);
                let prd_v = decode_utf16le(&bytes[564..1076]);
                let fl = if bytes.len() >= 1092 { u32::from_le_bytes([bytes[1088], bytes[1089], bytes[1090], bytes[1091]]) } else { 0 };
                (v, sz, prd, prd_v, std_file_count, std_file_offset, fl)
            } else {
                let leg_file_count = if bytes.len() >= 1128 {
                    u32::from_le_bytes([bytes[1124], bytes[1125], bytes[1126], bytes[1127]]) as usize
                } else { 0 };
                let leg_file_offset = if bytes.len() >= 1132 {
                    u32::from_le_bytes([bytes[1128], bytes[1129], bytes[1130], bytes[1131]]) as usize
                } else { 0 };
                let v = decode_utf16le(&bytes[0..96.min(bytes.len())]);
                let sz = if bytes.len() >= 100 { u32::from_le_bytes([bytes[96], bytes[97], bytes[98], bytes[99]]) as usize } else { bytes.len() };
                let prd = decode_utf16le(&bytes[100..612.min(bytes.len())]);
                let prd_v = decode_utf16le(&bytes[612..1124.min(bytes.len())]);
                let fl = if bytes.len() >= 1140 { u32::from_le_bytes([bytes[1136], bytes[1137], bytes[1138], bytes[1139]]) } else { 0 };
                (v, sz, prd, prd_v, leg_file_count, leg_file_offset, fl)
            }
        };

        let mut files = Vec::with_capacity(file_count);
        let mut cur_rec_ptr = file_offset;

        for i in 0..file_count {
            if cur_rec_ptr + 4 > bytes.len() {
                break;
            }

            let rec_size = u32::from_le_bytes([bytes[cur_rec_ptr], bytes[cur_rec_ptr + 1], bytes[cur_rec_ptr + 2], bytes[cur_rec_ptr + 3]]) as usize;
            let effective_size = if rec_size >= 516 && rec_size <= 65536 { rec_size } else { PAC_RECORD_SIZE };
            if cur_rec_ptr + effective_size > bytes.len() {
                break;
            }
            let rec = &bytes[cur_rec_ptr..cur_rec_ptr + effective_size];

            let id = decode_utf16le(&rec[4..516.min(rec.len())]);
            let filename = decode_utf16le(&rec[516.min(rec.len())..1028.min(rec.len())]);
            let version = decode_utf16le(&rec[1028.min(rec.len())..1540.min(rec.len())]);

            // Check if 32-bit offset layout (offset at 1552) or 64-bit size layout (offset at 1556)
            let (data_size, file_flag, offset, load_addr) = {
                let off_1552 = if rec.len() >= 1556 {
                    u32::from_le_bytes([rec[1552], rec[1553], rec[1554], rec[1555]]) as usize
                } else { 0 };
                let off_1556 = if rec.len() >= 1560 {
                    u32::from_le_bytes([rec[1556], rec[1557], rec[1558], rec[1559]]) as usize
                } else { 0 };

                // Data payload starts after the header and record table (at least 2048 bytes)
                let min_valid_offset = 2048;

                if off_1552 >= min_valid_offset && off_1552 < bytes.len() {
                    let sz = u32::from_le_bytes([rec[1540], rec[1541], rec[1542], rec[1543]]) as usize;
                    let flag = u32::from_le_bytes([rec[1544], rec[1545], rec[1546], rec[1547]]);
                    let addr_num = if rec.len() >= 1564 { u32::from_le_bytes([rec[1560], rec[1561], rec[1562], rec[1563]]) } else { 0 };
                    let laddr = if rec.len() >= 1568 && addr_num > 0 { u32::from_le_bytes([rec[1564], rec[1565], rec[1566], rec[1567]]) } else { 0 };
                    (sz, flag, off_1552, laddr)
                } else if off_1556 >= min_valid_offset && off_1556 < bytes.len() {
                    let lo_size = u32::from_le_bytes([rec[1540], rec[1541], rec[1542], rec[1543]]) as u64;
                    let hi_size = u32::from_le_bytes([rec[1544], rec[1545], rec[1546], rec[1547]]) as u64;
                    let sz = (lo_size | (hi_size << 32)) as usize;
                    let flag = if rec.len() >= 1552 { u32::from_le_bytes([rec[1548], rec[1549], rec[1550], rec[1551]]) } else { 0 };
                    let addr_num = if rec.len() >= 1568 { u32::from_le_bytes([rec[1564], rec[1565], rec[1566], rec[1567]]) } else { 0 };
                    let laddr = if rec.len() >= 1572 && addr_num > 0 { u32::from_le_bytes([rec[1568], rec[1569], rec[1570], rec[1571]]) } else { 0 };
                    (sz, flag, off_1556, laddr)
                } else {
                    (0, 0, 0, 0)
                }
            };

            let mut data = Vec::new();
            if offset > 0 && offset < bytes.len() && data_size > 0 {
                let end = (offset + data_size).min(bytes.len());
                data = bytes[offset..end].to_vec();
            }

            files.push(PacFileRecord {
                index: i,
                id: if id.is_empty() { format!("PART_{i}") } else { id },
                filename: if filename.is_empty() { format!("part_{i}.bin") } else { filename },
                version,
                size: data_size,
                offset,
                load_addr,
                active: file_flag == 1 || data_size > 0,
                data,
            });

            cur_rec_ptr += effective_size;
        }

        Ok(Self {
            pac_version,
            product_name,
            product_version,
            flash_type,
            total_size,
            files,
        })
    }

    pub fn extract_all<P: AsRef<Path>>(&self, out_dir: P) -> io::Result<usize> {
        fs::create_dir_all(&out_dir)?;
        let mut count = 0;
        for f in &self.files {
            let out_name = Path::new(&f.filename).file_name().unwrap_or_default();
            let target = out_dir.as_ref().join(out_name);
            let mut file = File::create(target)?;
            file.write_all(&f.data)?;
            println!("[PAC] Extracted {:<16} ({:>10} bytes) -> '{}'", f.id, f.size, f.filename);
            count += 1;
        }
        println!("[PAC] Extraction complete: {} partition files saved to '{}'.", count, out_dir.as_ref().display());
        Ok(count)
    }

    pub fn extract_single<P: AsRef<Path>>(&self, target_id_or_name: &str, out_path: P) -> io::Result<bool> {
        let rec = self.files.iter().find(|f| {
            f.id.eq_ignore_ascii_case(target_id_or_name) ||
            f.filename.eq_ignore_ascii_case(target_id_or_name)
        });

        if let Some(f) = rec {
            if let Some(parent) = out_path.as_ref().parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = File::create(&out_path)?;
            file.write_all(&f.data)?;
            println!("[PAC] Extracted '{}' ({} bytes) -> '{}'", f.id, f.size, out_path.as_ref().display());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn dump_xml(&self) -> String {
        for f in &self.files {
            if f.id.to_lowercase().contains("xml") || f.filename.to_lowercase().ends_with(".xml") {
                if let Ok(s) = String::from_utf8(f.data.clone()) {
                    return s;
                }
            }
        }
        String::new()
    }
}

pub fn encode_pac_str(s: &str, byte_len: usize) -> Vec<u8> {
    let mut out = vec![0u8; byte_len];
    encode_utf16le_into(s, &mut out);
    out
}

pub fn decode_pac_str(raw: &[u8]) -> String {
    decode_utf16le(raw)
}

fn encode_utf16le_into(s: &str, dest: &mut [u8]) {
    let max_u16 = dest.len() / 2;
    let u16_chars: Vec<u16> = s.encode_utf16().take(max_u16).collect();
    for (i, &c) in u16_chars.iter().enumerate() {
        dest[i * 2..i * 2 + 2].copy_from_slice(&c.to_le_bytes());
    }
}

fn decode_utf16le(raw: &[u8]) -> String {
    let mut u16_chars = Vec::new();
    for chunk in raw.chunks_exact(2) {
        let val = u16::from_le_bytes([chunk[0], chunk[1]]);
        if val == 0 {
            break;
        }
        u16_chars.push(val);
    }
    String::from_utf16_lossy(&u16_chars).trim().to_string()
}
