// SPDX-License-Identifier: MIT
//! Unisoc SPRD BOOT0 Header & BootROM Reset Vector Loader

pub const SPRD_MAGIC: u32 = 0x5350_5244; // "SPRD"

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprdHeader {
    pub magic: u32,
    pub version: u32,
    pub payload_size: u32,
    pub load_addr: u32,
    pub entry_point: u32,
    pub checksum: u32,
    pub header_checksum: u32,
}

impl SprdHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 32 {
            return Err("SPRD header too short");
        }
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != SPRD_MAGIC {
            return Err("Invalid SPRD magic");
        }
        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let payload_size = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let load_addr = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let entry_point = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let checksum = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        let header_checksum = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);

        // Validate header checksum (sum of first 6 words)
        let mut calc_hdr: u32 = 0;
        for i in 0..6 {
            let offset = i * 4;
            let w = u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
            calc_hdr = calc_hdr.wrapping_add(w);
        }
        if calc_hdr != header_checksum {
            return Err("SPRD header checksum failure");
        }

        Ok(Self {
            magic,
            version,
            payload_size,
            load_addr,
            entry_point,
            checksum,
            header_checksum,
        })
    }

    pub fn create(payload: &[u8], load_addr: u32, entry_point: u32) -> Vec<u8> {
        let mut out = vec![0u8; 32 + payload.len()];
        let magic = SPRD_MAGIC;
        let version = 1u32;
        let payload_size = payload.len() as u32;

        let mut checksum: u32 = 0;
        let mut chunks = payload.chunks_exact(4);
        for chunk in &mut chunks {
            let w = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            checksum = checksum.wrapping_add(w);
        }
        let rem = chunks.remainder();
        if !rem.is_empty() {
            let mut buf = [0u8; 4];
            buf[..rem.len()].copy_from_slice(rem);
            checksum = checksum.wrapping_add(u32::from_le_bytes(buf));
        }

        let words = [magic, version, payload_size, load_addr, entry_point, checksum];
        let mut header_checksum: u32 = 0;
        for (i, &w) in words.iter().enumerate() {
            out[i * 4..(i + 1) * 4].copy_from_slice(&w.to_le_bytes());
            header_checksum = header_checksum.wrapping_add(w);
        }
        out[24..28].copy_from_slice(&header_checksum.to_le_bytes());
        out[28..32].fill(0); // reserved
        out[32..].copy_from_slice(payload);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprd_header_roundtrip() {
        let payload = vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let img = SprdHeader::create(&payload, 0x00006200, 0x00006200);
        assert_eq!(img.len(), 32 + payload.len());

        let hdr = SprdHeader::parse(&img).expect("Failed to parse SPRD header");
        assert_eq!(hdr.magic, SPRD_MAGIC);
        assert_eq!(hdr.load_addr, 0x00006200);
        assert_eq!(hdr.entry_point, 0x00006200);
        assert_eq!(hdr.payload_size, 8);
    }
}
