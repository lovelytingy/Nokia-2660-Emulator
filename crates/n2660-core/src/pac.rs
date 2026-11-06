// SPDX-License-Identifier: MIT
//! Unisoc Spreadtrum .PAC Firmware Container parser and packer

pub const PAC_HEADER_SIZE: usize = 2112; // 0x840
pub const PAC_FILE_ENTRY_SIZE: usize = 2560; // Standard PAC entry size

#[derive(Debug, Clone)]
pub struct PacFileRecord {
    pub id: String,
    pub filename: String,
    pub size: usize,
    pub load_addr: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PacArchive {
    pub product_version: String,
    pub product_alias: String,
    pub files: Vec<PacFileRecord>,
}

impl Default for PacArchive {
    fn default() -> Self {
        Self::new("Nokia 2660 Flip (TA-1480)", "Nokia_2660")
    }
}

impl PacArchive {
    pub fn new(product_version: &str, product_alias: &str) -> Self {
        Self {
            product_version: product_version.to_string(),
            product_alias: product_alias.to_string(),
            files: Vec::new(),
        }
    }

    pub fn add_file(&mut self, id: &str, filename: &str, load_addr: u32, data: Vec<u8>) {
        let size = data.len();
        self.files.push(PacFileRecord {
            id: id.to_string(),
            filename: filename.to_string(),
            size,
            load_addr,
            data,
        });
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut out = Vec::new();
        // 1. Pack Header (2112 bytes)
        let mut hdr = vec![0u8; PAC_HEADER_SIZE];

        // szPrdVers (UTF-16LE, up to 24 chars / 48 bytes)
        let vers_u16: Vec<u16> = self.product_version.encode_utf16().take(24).collect();
        for (i, &c) in vers_u16.iter().enumerate() {
            hdr[i * 2..i * 2 + 2].copy_from_slice(&c.to_le_bytes());
        }

        // szPrdAlias (UTF-16LE, up to 24 chars / 48 bytes at offset 48)
        let alias_u16: Vec<u16> = self.product_alias.encode_utf16().take(24).collect();
        for (i, &c) in alias_u16.iter().enumerate() {
            let off = 48 + i * 2;
            hdr[off..off + 2].copy_from_slice(&c.to_le_bytes());
        }

        // dwFileCount at offset 96 (0x60)
        let count = self.files.len() as u32;
        hdr[96..100].copy_from_slice(&count.to_le_bytes());

        // dwFileOffset at offset 100 (0x64)
        let file_offset = PAC_HEADER_SIZE as u32;
        hdr[100..104].copy_from_slice(&file_offset.to_le_bytes());

        out.extend_from_slice(&hdr);

        // 2. Compute payload offsets
        let file_table_size = self.files.len() * PAC_FILE_ENTRY_SIZE;
        let mut cur_payload_offset = PAC_HEADER_SIZE + file_table_size;

        // 3. Write File Entries
        let mut payload_blobs = Vec::with_capacity(self.files.len());
        for f in &self.files {
            let mut entry = vec![0u8; PAC_FILE_ENTRY_SIZE];
            // szFileID at 0 (UTF-16LE up to 50 chars / 100 bytes)
            let id_u16: Vec<u16> = f.id.encode_utf16().take(50).collect();
            for (i, &c) in id_u16.iter().enumerate() {
                entry[i * 2..i * 2 + 2].copy_from_slice(&c.to_le_bytes());
            }

            // szFileName at 512 (UTF-16LE up to 256 chars / 512 bytes)
            let name_u16: Vec<u16> = f.filename.encode_utf16().take(256).collect();
            for (i, &c) in name_u16.iter().enumerate() {
                let off = 512 + i * 2;
                entry[off..off + 2].copy_from_slice(&c.to_le_bytes());
            }

            // nFileSize (64-bit at offset 1024)
            let sz64 = f.size as u64;
            entry[1024..1032].copy_from_slice(&sz64.to_le_bytes());

            // dwDataOffset (64-bit at offset 1040)
            let off64 = cur_payload_offset as u64;
            entry[1040..1048].copy_from_slice(&off64.to_le_bytes());

            // dwLoadAddr at offset 1056
            entry[1056..1060].copy_from_slice(&f.load_addr.to_le_bytes());

            out.extend_from_slice(&entry);
            cur_payload_offset += f.data.len();
            payload_blobs.push(&f.data);
        }

        // 4. Append payloads
        for p in payload_blobs {
            out.extend_from_slice(p);
        }

        out
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < PAC_HEADER_SIZE {
            return Err("Data too small for PAC header");
        }

        let vers = decode_utf16le(&bytes[0..48]);
        let alias = decode_utf16le(&bytes[48..96]);
        let file_count = u32::from_le_bytes([bytes[96], bytes[97], bytes[98], bytes[99]]) as usize;
        let file_offset = u32::from_le_bytes([bytes[100], bytes[101], bytes[102], bytes[103]]) as usize;

        let mut files = Vec::with_capacity(file_count);
        for i in 0..file_count {
            let entry_start = file_offset + i * PAC_FILE_ENTRY_SIZE;
            if entry_start + PAC_FILE_ENTRY_SIZE > bytes.len() {
                break;
            }
            let entry = &bytes[entry_start..entry_start + PAC_FILE_ENTRY_SIZE];
            let id = decode_utf16le(&entry[0..100]);
            let filename = decode_utf16le(&entry[512..1024]);
            let size = u64::from_le_bytes([
                entry[1024], entry[1025], entry[1026], entry[1027],
                entry[1028], entry[1029], entry[1030], entry[1031],
            ]) as usize;
            let offset = u64::from_le_bytes([
                entry[1040], entry[1041], entry[1042], entry[1043],
                entry[1044], entry[1045], entry[1046], entry[1047],
            ]) as usize;
            let load_addr = u32::from_le_bytes([entry[1056], entry[1057], entry[1058], entry[1059]]);

            let mut data = Vec::new();
            if offset < bytes.len() {
                let end = (offset + size).min(bytes.len());
                data = bytes[offset..end].to_vec();
            }

            files.push(PacFileRecord {
                id,
                filename,
                size,
                load_addr,
                data,
            });
        }

        Ok(Self {
            product_version: vers,
            product_alias: alias,
            files,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pac_roundtrip() {
        let mut pac = PacArchive::new("Nokia 2660 Flip", "TA-1480");
        pac.add_file("FDL1", "fdl1.bin", 0x00006200, vec![1, 2, 3, 4]);
        pac.add_file("BOOT0", "boot0.bin", 0x00006200, vec![5, 6, 7, 8]);

        let packed = pac.pack();
        let parsed = PacArchive::parse(&packed).expect("Failed to parse PAC");
        assert_eq!(parsed.product_version, "Nokia 2660 Flip");
        assert_eq!(parsed.files.len(), 2);
        assert_eq!(parsed.files[0].id, "FDL1");
        assert_eq!(parsed.files[0].data, vec![1, 2, 3, 4]);
        assert_eq!(parsed.files[1].id, "BOOT0");
        assert_eq!(parsed.files[1].data, vec![5, 6, 7, 8]);
    }
}
