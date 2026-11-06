// SPDX-License-Identifier: MIT
//! ARM Boot Tags (ATAGS) generation and DRAM staging for ARMv7-A Linux / MOCOR OS handoff

pub const ATAG_NONE: u32    = 0x0000_0000;
pub const ATAG_CORE: u32    = 0x5441_0001;
pub const ATAG_MEM: u32     = 0x5441_0002;
pub const ATAG_SERIAL: u32  = 0x5441_0006;
pub const ATAG_CMDLINE: u32 = 0x5441_0009;

#[derive(Debug, Clone)]
pub struct AtagsBuilder {
    pub ram_start: u32,
    pub ram_size: u32,
    pub cmdline: Option<String>,
    pub serial: Option<(u32, u32)>,
    pub page_size: u32,
    pub root_dev: u32,
}

impl Default for AtagsBuilder {
    fn default() -> Self {
        Self {
            ram_start: 0x8000_0000,
            ram_size: 48 * 1024 * 1024, // 48 MB
            cmdline: Some("console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M".to_string()),
            serial: Some((0x1480_0001, 0x9117_2660)),
            page_size: 4096,
            root_dev: 0,
        }
    }
}

impl AtagsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ram(mut self, start: u32, size: u32) -> Self {
        self.ram_start = start;
        self.ram_size = size;
        self
    }

    pub fn with_cmdline(mut self, cmdline: Option<String>) -> Self {
        self.cmdline = cmdline;
        self
    }

    pub fn build(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);

        // 1. ATAG_CORE (5 words: size=5, tag, flags=0, pagesize, rootdev)
        out.extend_from_slice(&5u32.to_le_bytes());
        out.extend_from_slice(&ATAG_CORE.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // flags
        out.extend_from_slice(&self.page_size.to_le_bytes());
        out.extend_from_slice(&self.root_dev.to_le_bytes());

        // 2. ATAG_MEM (4 words: size=4, tag, size_bytes, start_addr)
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&ATAG_MEM.to_le_bytes());
        out.extend_from_slice(&self.ram_size.to_le_bytes());
        out.extend_from_slice(&self.ram_start.to_le_bytes());

        // 3. ATAG_CMDLINE (optional)
        if let Some(cmd) = &self.cmdline {
            let mut cmd_bytes = cmd.as_bytes().to_vec();
            cmd_bytes.push(0); // null terminator
            while cmd_bytes.len() % 4 != 0 {
                cmd_bytes.push(0); // 4-byte align
            }
            let word_count = 2 + (cmd_bytes.len() / 4);
            out.extend_from_slice(&(word_count as u32).to_le_bytes());
            out.extend_from_slice(&ATAG_CMDLINE.to_le_bytes());
            out.extend_from_slice(&cmd_bytes);
        }

        // 4. ATAG_SERIAL (optional)
        if let Some((low, high)) = self.serial {
            out.extend_from_slice(&4u32.to_le_bytes());
            out.extend_from_slice(&ATAG_SERIAL.to_le_bytes());
            out.extend_from_slice(&low.to_le_bytes());
            out.extend_from_slice(&high.to_le_bytes());
        }

        // 5. ATAG_NONE (2 words: size=0, tag=ATAG_NONE)
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&ATAG_NONE.to_le_bytes());

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atags_builder() {
        let builder = AtagsBuilder::new();
        let bytes = builder.build();
        assert!(bytes.len() >= 64);
        assert_eq!(bytes.len() % 4, 0);

        // First tag must be ATAG_CORE (size 5, tag 0x54410001)
        let size = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let tag = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(size, 5);
        assert_eq!(tag, ATAG_CORE);
    }
}
