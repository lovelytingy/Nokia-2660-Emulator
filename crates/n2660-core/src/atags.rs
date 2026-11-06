// SPDX-License-Identifier: MIT
//! ARM Boot Tags (ATAGS) generation, parsing, and DRAM staging for ARMv7-A Linux / MOCOR OS handoff

pub const ATAG_NONE: u32     = 0x0000_0000;
pub const ATAG_CORE: u32     = 0x5441_0001;
pub const ATAG_MEM: u32      = 0x5441_0002;
pub const ATAG_VIDEOTEXT: u32 = 0x5441_0003;
pub const ATAG_RAMDISK: u32   = 0x5441_0004;
pub const ATAG_INITRD2: u32   = 0x5442_0005;
pub const ATAG_SERIAL: u32   = 0x5441_0006;
pub const ATAG_REVISION: u32 = 0x5441_0007;
pub const ATAG_VIDEOLFB: u32 = 0x5441_0008;
pub const ATAG_CMDLINE: u32  = 0x5441_0009;

pub const DEFAULT_BOARD_ID: u32 = 0x9117;
pub const DEFAULT_ATAGS_ADDR: u32 = 0x8000_0100;
pub const DEFAULT_DRAM_BASE: u32 = 0x8000_0000;
pub const DEFAULT_DRAM_SIZE: u32 = 48 * 1024 * 1024;
pub const DEFAULT_CMDLINE: &str = "console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M";

#[derive(Debug, Clone)]
pub struct ParsedTag {
    pub tag: u32,
    pub name: String,
    pub size_words: u32,
    pub data: Vec<u8>,
    pub details: String,
}

#[derive(Debug, Clone)]
pub struct AtagsBuilder {
    pub ram_start: u32,
    pub ram_size: u32,
    pub cmdline: Option<String>,
    pub serial: Option<(u32, u32)>,
    pub revision: Option<u32>,
    pub initrd2: Option<(u32, u32)>, // (start, size)
    pub page_size: u32,
    pub flags: u32,
    pub root_dev: u32,
}

impl Default for AtagsBuilder {
    fn default() -> Self {
        Self {
            ram_start: DEFAULT_DRAM_BASE,
            ram_size: DEFAULT_DRAM_SIZE,
            cmdline: Some(DEFAULT_CMDLINE.to_string()),
            serial: Some((0x1480_0001, 0x9117_2660)),
            revision: Some(0x0000_0001),
            initrd2: None,
            page_size: 4096,
            flags: 0,
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

    pub fn with_serial(mut self, low: u32, high: u32) -> Self {
        self.serial = Some((low, high));
        self
    }

    pub fn with_revision(mut self, rev: u32) -> Self {
        self.revision = Some(rev);
        self
    }

    pub fn with_initrd2(mut self, start: u32, size: u32) -> Self {
        self.initrd2 = Some((start, size));
        self
    }

    pub fn with_core(mut self, flags: u32, page_size: u32, root_dev: u32) -> Self {
        self.flags = flags;
        self.page_size = page_size;
        self.root_dev = root_dev;
        self
    }

    pub fn build(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);

        // 1. ATAG_CORE (5 words)
        out.extend_from_slice(&5u32.to_le_bytes());
        out.extend_from_slice(&ATAG_CORE.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&self.page_size.to_le_bytes());
        out.extend_from_slice(&self.root_dev.to_le_bytes());

        // 2. ATAG_MEM (4 words)
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&ATAG_MEM.to_le_bytes());
        out.extend_from_slice(&self.ram_size.to_le_bytes());
        out.extend_from_slice(&self.ram_start.to_le_bytes());

        // 3. ATAG_REVISION
        if let Some(rev) = self.revision {
            out.extend_from_slice(&3u32.to_le_bytes());
            out.extend_from_slice(&ATAG_REVISION.to_le_bytes());
            out.extend_from_slice(&rev.to_le_bytes());
        }

        // 4. ATAG_SERIAL
        if let Some((low, high)) = self.serial {
            out.extend_from_slice(&4u32.to_le_bytes());
            out.extend_from_slice(&ATAG_SERIAL.to_le_bytes());
            out.extend_from_slice(&low.to_le_bytes());
            out.extend_from_slice(&high.to_le_bytes());
        }

        // 5. ATAG_INITRD2
        if let Some((start, size)) = self.initrd2 {
            out.extend_from_slice(&4u32.to_le_bytes());
            out.extend_from_slice(&ATAG_INITRD2.to_le_bytes());
            out.extend_from_slice(&start.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
        }

        // 6. ATAG_CMDLINE
        if let Some(ref cmd) = self.cmdline {
            let mut cmd_bytes = cmd.as_bytes().to_vec();
            cmd_bytes.push(0); // null terminator
            let len = cmd_bytes.len();
            let words = (len + 3) / 4;
            let tag_size = 2 + words as u32;

            out.extend_from_slice(&tag_size.to_le_bytes());
            out.extend_from_slice(&ATAG_CMDLINE.to_le_bytes());
            out.extend_from_slice(&cmd_bytes);
            let pad = words * 4 - len;
            out.extend(std::iter::repeat(0).take(pad));
        }

        // 7. ATAG_NONE (2 words)
        out.extend_from_slice(&0u32.to_le_bytes()); // size = 0
        out.extend_from_slice(&ATAG_NONE.to_le_bytes());

        out
    }
}

pub struct AtagsParser;

impl AtagsParser {
    pub fn parse(bytes: &[u8], _base_addr: u32) -> Result<Vec<ParsedTag>, &'static str> {
        let mut tags = Vec::new();
        let mut offset = 0;

        while offset + 8 <= bytes.len() {
            let size = u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
            let tag = u32::from_le_bytes([bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]]);

            if tag == ATAG_NONE {
                tags.push(ParsedTag {
                    tag: ATAG_NONE,
                    name: "ATAG_NONE".to_string(),
                    size_words: size,
                    data: Vec::new(),
                    details: "End of tag list".to_string(),
                });
                break;
            }

            if size < 2 {
                return Err("Invalid tag size < 2");
            }

            let byte_len = (size as usize) * 4;
            if offset + byte_len > bytes.len() {
                return Err("Tag exceeds buffer boundary");
            }

            let payload = &bytes[offset + 8..offset + byte_len];
            let (name, details) = match tag {
                ATAG_CORE => {
                    let flags = if payload.len() >= 4 { u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) } else { 0 };
                    let pagesize = if payload.len() >= 8 { u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]) } else { 0 };
                    let rootdev = if payload.len() >= 12 { u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]) } else { 0 };
                    ("ATAG_CORE".to_string(), format!("flags=0x{:X}, pagesize={}, rootdev=0x{:X}", flags, pagesize, rootdev))
                }
                ATAG_MEM => {
                    let sz = if payload.len() >= 4 { u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) } else { 0 };
                    let start = if payload.len() >= 8 { u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]) } else { 0 };
                    ("ATAG_MEM".to_string(), format!("size={} MB (0x{:X}), start=0x{:08X}", sz / (1024 * 1024), sz, start))
                }
                ATAG_SERIAL => {
                    let lo = if payload.len() >= 4 { u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) } else { 0 };
                    let hi = if payload.len() >= 8 { u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]) } else { 0 };
                    ("ATAG_SERIAL".to_string(), format!("serial=0x{:08X}{:08X}", hi, lo))
                }
                ATAG_REVISION => {
                    let rev = if payload.len() >= 4 { u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) } else { 0 };
                    ("ATAG_REVISION".to_string(), format!("rev=0x{:08X}", rev))
                }
                ATAG_INITRD2 => {
                    let start = if payload.len() >= 4 { u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) } else { 0 };
                    let sz = if payload.len() >= 8 { u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]) } else { 0 };
                    ("ATAG_INITRD2".to_string(), format!("start=0x{:08X}, size=0x{:X}", start, sz))
                }
                ATAG_CMDLINE => {
                    let s = String::from_utf8_lossy(payload).trim_end_matches('\0').to_string();
                    ("ATAG_CMDLINE".to_string(), format!("cmdline=\"{}\"", s))
                }
                _ => (format!("ATAG_0x{:08X}", tag), format!("payload_bytes={}", payload.len())),
            };

            tags.push(ParsedTag {
                tag,
                name,
                size_words: size,
                data: payload.to_vec(),
                details,
            });

            offset += byte_len;
        }

        Ok(tags)
    }
}
