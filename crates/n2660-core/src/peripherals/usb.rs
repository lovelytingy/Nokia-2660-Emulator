// SPDX-License-Identifier: MIT
//! Unisoc T107 (UMS9117) USB OTG Hardware Controller & Libc Server Protocol Model
//!
//! Emulates the Unisoc USB 2.0 OTG hardware DMA endpoints (EP5 TX, EP6 RX)
//! and implements the host-side libc server protocol used by bare-metal RAM payloads
//! such as `keypad_test.bin`, `baremetal_lcd.bin`, `hello_world.bin`, and fpdoom.

use std::collections::VecDeque;

pub const USB_CHECKSUM_INIT: u32 = 0x5a5a;

pub const HOST_CONNECT: u8 = 0x00;
pub const CMD_MESSAGE: u8 = 0x80;
pub const CMD_FOPEN: u8 = 0x81;
pub const CMD_FREAD: u8 = 0x82;
pub const CMD_FWRITE: u8 = 0x83;
pub const CMD_FCLOSE: u8 = 0x84;
pub const CMD_FSEEK: u8 = 0x85;
pub const CMD_FTELL: u8 = 0x86;
pub const CMD_GETARGS: u8 = 0x87;
pub const CMD_RESETIO: u8 = 0x88;
pub const CMD_USBBENCH: u8 = 0x89;

pub const ARGV_GET_ARGC: u8 = 1;
pub const ARGV_GET_SIZE: u8 = 2;
pub const ARGV_GET_ARGV: u8 = 4;

/// Computes the authentic 16-bit one's complement addition checksum
pub fn fastchk16(mut crc: u32, src: &[u8]) -> u16 {
    let mut i = 0;
    while i + 1 < src.len() {
        crc += ((src[i + 1] as u32) << 8) | (src[i] as u32);
        i += 2;
    }
    if i < src.len() {
        crc += src[i] as u32;
    }
    crc = (crc >> 16) + (crc & 0xffff);
    crc += crc >> 16;
    (crc & 0xffff) as u16
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsbState {
    WaitingFdlAck,
    Connected,
}

#[derive(Debug, Clone)]
pub struct VirtualFile {
    pub name: String,
    pub data: Vec<u8>,
    pub pos: usize,
}

pub struct SprdUsb {
    pub state: UsbState,
    pub tx_status: u32,
    pub rx_status: u32,
    pub tx_tr_ptr: u32,
    pub rx_tr_ptr: u32,
    pub rx_rem_len: u32,
    pub tx_buffer: Vec<u8>,
    pub pending_rx: VecDeque<u8>,
    pub open_files: Vec<Option<VirtualFile>>,
    pub pinmap_data: Vec<u8>,
    pub keymap_data: Vec<u8>,
    pub stdout_log: Vec<u8>,
    pub print_stdout: bool,
    pub args: Vec<String>,
    pub handshake_drained: bool,
}

impl SprdUsb {
    pub fn new(print_stdout: bool) -> Self {
        // Embedded authentic pinmap and keymap for TA-1480 / TA-1711
        let pinmap = include_bytes!("data/pinmap.bin").to_vec();
        let keymap = include_bytes!("data/keymap.bin").to_vec();

        let mut files = Vec::new();
        // Index 0: stdin, 1: stdout, 2: stderr
        files.push(Some(VirtualFile { name: "stdin".into(), data: Vec::new(), pos: 0 }));
        files.push(Some(VirtualFile { name: "stdout".into(), data: Vec::new(), pos: 0 }));
        files.push(Some(VirtualFile { name: "stderr".into(), data: Vec::new(), pos: 0 }));

        Self {
            state: UsbState::WaitingFdlAck,
            tx_status: 0,
            rx_status: 0,
            tx_tr_ptr: 0,
            rx_tr_ptr: 0,
            rx_rem_len: 12800 << 16,
            tx_buffer: Vec::new(),
            pending_rx: VecDeque::new(),
            open_files: files,
            pinmap_data: pinmap,
            keymap_data: keymap,
            stdout_log: Vec::new(),
            print_stdout,
            args: vec!["fptest".to_string(), "display".to_string()],
            handshake_drained: false,
        }
    }

    /// Read USB OTG controller registers
    pub fn read_reg(&self, addr: u32) -> u32 {
        match addr {
            0x2020_0000 => 0,
            0x2020_0002 => 0, // No unhandled USB control setup events
            0x2020_000a => 0,
            0x2020_0001 => 0x10, // High Speed (not full speed 64-byte mode)
            0x2020_0012 => 0,
            0x2020_1c80 => 0,
            0x2020_1c84 => 0,
            0x2020_1c88 => self.tx_status,
            0x2020_1c90 => 0,
            0x2020_1c94 => self.tx_tr_ptr,
            0x2020_1e80 => 0,
            0x2020_1e84 => 0,
            0x2020_1e88 => self.rx_status,
            0x2020_1e90 => self.rx_rem_len,
            0x2020_1e94 => self.rx_tr_ptr,
            _ => 0,
        }
    }

    /// Write USB OTG controller registers
    pub fn write_reg(&mut self, addr: u32, val: u32) {
        match addr {
            0x2020_1c84 => {
                // EP5 TX Trigger: starts TX DMA
                if (val & 1) != 0 {
                    // Set TX DMA complete bit (bit 18)
                    self.tx_status |= 1 << 18;
                }
            }
            0x2020_1c88 => {
                self.tx_status = val;
            }
            0x2020_1c94 => {
                self.tx_tr_ptr = val;
            }
            0x2020_1e84 => {
                // EP6 RX Trigger: guest is ready to receive
                if (val & 1) != 0 {
                    // If we have pending RX bytes, signal bit 18
                    if !self.pending_rx.is_empty() {
                        self.rx_status |= 1 << 18;
                    }
                }
            }
            0x2020_1e88 => {
                if (val & 0x1400_0000) != 0 {
                    // Guest acknowledged RX packet
                    self.rx_status &= !(1 << 18);
                }
                if (val & 0x14) != 0 {
                    self.rx_status |= 0x14;
                }
            }
            0x2020_1e90 => {
                self.rx_rem_len = val;
            }
            0x2020_1e94 => {
                self.rx_tr_ptr = val;
            }
            _ => {}
        }
    }

    /// Ingest newly transmitted data from guest EP5 TX DMA
    pub fn ingest_tx_data(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        self.tx_buffer.extend_from_slice(data);
        self.process_tx_queue()
    }

    pub fn process_tx_queue(&mut self) -> Option<Vec<u8>> {
        if self.tx_buffer.is_empty() {
            return None;
        }

        // State 1: Awaiting FDL ACK [8 bytes: 0x7e, 0x00, 0x80, 0x00, 0x00, 0xff, 0x7f, 0x7e]
        if self.state == UsbState::WaitingFdlAck {
            if let Some(pos) = self.tx_buffer.windows(8).position(|w| {
                w == [0x7e, 0x00, 0x80, 0x00, 0x00, 0xff, 0x7f, 0x7e]
            }) {
                self.tx_buffer.drain(0..pos + 8);
                self.state = UsbState::Connected;
                println!("[USB] Received FDL ACK handshake from payload. Established libc server link.");
                // Deliver HOST_CONNECT (0x00)
                return Some(vec![HOST_CONNECT]);
            }
            return None;
        }

        // State 2: Libc Server Protocol Request Loop
        self.handshake_drained = true;
        let cmd = self.tx_buffer[0];
        match cmd {
            CMD_MESSAGE => {
                // Header: 4 bytes [cmd, len, chk_lo, chk_hi] + len bytes string
                if self.tx_buffer.len() < 4 {
                    return None;
                }
                let len = self.tx_buffer[1] as usize;
                if self.tx_buffer.len() < 4 + len {
                    return None;
                }
                let msg_bytes = self.tx_buffer[4..4 + len].to_vec();
                self.tx_buffer.drain(0..4 + len);
                let msg = String::from_utf8_lossy(&msg_bytes);
                println!("[PAYLOAD DEBUG] {}", msg);
                None
            }

            CMD_GETARGS => {
                // Header: 6 bytes [cmd, flags, skip_lo, skip_hi, chk_lo, chk_hi]
                if self.tx_buffer.len() < 6 {
                    return None;
                }
                let flags = self.tx_buffer[1];
                let _skip = u16::from_le_bytes([self.tx_buffer[2], self.tx_buffer[3]]);
                self.tx_buffer.drain(0..6);

                let mut total_size = 0usize;
                let mut argv_bytes = Vec::new();
                for a in &self.args {
                    argv_bytes.extend_from_slice(a.as_bytes());
                    argv_bytes.push(0);
                    total_size += a.len() + 1;
                }

                if (flags & (ARGV_GET_ARGC | ARGV_GET_SIZE)) != 0 {
                    // Phase 1 response: 6 bytes [argc: u16, total_size: u16, chk: u16]
                    let mut resp = Vec::new();
                    resp.extend_from_slice(&(self.args.len() as u16).to_le_bytes());
                    resp.extend_from_slice(&(total_size as u16).to_le_bytes());
                    let chk = fastchk16(USB_CHECKSUM_INIT, &resp);
                    resp.extend_from_slice(&chk.to_le_bytes());
                    Some(resp)
                } else if (flags & ARGV_GET_ARGV) != 0 {
                    // Phase 2 response: 2 bytes checksum + argv data
                    let chk = fastchk16(USB_CHECKSUM_INIT, &argv_bytes);
                    let mut resp = Vec::new();
                    resp.extend_from_slice(&chk.to_le_bytes());
                    resp.extend_from_slice(&argv_bytes);
                    Some(resp)
                } else {
                    None
                }
            }

            CMD_FOPEN => {
                // Header: 6 bytes [cmd, flags, len_lo, len_hi, chk_lo, chk_hi] + len bytes filename
                if self.tx_buffer.len() < 6 {
                    return None;
                }
                let _flags = self.tx_buffer[1];
                let len = u16::from_le_bytes([self.tx_buffer[2], self.tx_buffer[3]]) as usize;
                if self.tx_buffer.len() < 6 + len {
                    return None;
                }
                let filename_bytes = self.tx_buffer[6..6 + len].to_vec();
                self.tx_buffer.drain(0..6 + len);

                let filename = String::from_utf8_lossy(&filename_bytes).trim_matches('\0').to_string();
                let clean_name = filename.split('/').last().unwrap_or(&filename).to_string();

                let file_data = if clean_name == "pinmap.bin" {
                    self.pinmap_data.clone()
                } else if clean_name == "keymap.bin" {
                    self.keymap_data.clone()
                } else {
                    std::fs::read(&clean_name).unwrap_or_default()
                };

                let handle = self.open_files.len();
                self.open_files.push(Some(VirtualFile {
                    name: clean_name.clone(),
                    data: file_data,
                    pos: 0,
                }));

                println!("[USB LIBC] fopen(\"{}\") -> handle {}", clean_name, handle);
                Some(vec![handle as u8])
            }

            CMD_FSEEK => {
                // Header: 10 bytes [cmd, handle, origin_lo, origin_hi, off0..3, chk_lo, chk_hi]
                if self.tx_buffer.len() < 10 {
                    return None;
                }
                let handle = self.tx_buffer[1] as usize;
                let origin = u16::from_le_bytes([self.tx_buffer[2], self.tx_buffer[3]]);
                let offset = i32::from_le_bytes([
                    self.tx_buffer[4], self.tx_buffer[5], self.tx_buffer[6], self.tx_buffer[7]
                ]);
                self.tx_buffer.drain(0..10);

                let mut ret = 0u8;
                if let Some(Some(f)) = self.open_files.get_mut(handle) {
                    let new_pos = match origin {
                        0 => offset as i64,                    // SEEK_SET
                        1 => f.pos as i64 + offset as i64,     // SEEK_CUR
                        2 => f.data.len() as i64 + offset as i64, // SEEK_END
                        _ => -1,
                    };
                    if new_pos >= 0 && (new_pos as usize) <= f.data.len() {
                        f.pos = new_pos as usize;
                    } else {
                        ret = 0xFF; // Error
                    }
                } else {
                    ret = 0xFF;
                }
                Some(vec![ret])
            }

            CMD_FTELL => {
                // Header: 4 bytes [cmd, handle, chk_lo, chk_hi]
                if self.tx_buffer.len() < 4 {
                    return None;
                }
                let handle = self.tx_buffer[1] as usize;
                self.tx_buffer.drain(0..4);

                let pos = if let Some(Some(f)) = self.open_files.get(handle) {
                    f.pos as u32
                } else {
                    0xFFFF_FFFF
                };
                Some(pos.to_le_bytes().to_vec())
            }

            CMD_FREAD => {
                // Header: 6 bytes [cmd, handle, size_lo, size_hi, chk_lo, chk_hi]
                if self.tx_buffer.len() < 6 {
                    return None;
                }
                let handle = self.tx_buffer[1] as usize;
                let req_size = u16::from_le_bytes([self.tx_buffer[2], self.tx_buffer[3]]) as usize;
                self.tx_buffer.drain(0..6);

                let mut read_bytes = Vec::new();
                if let Some(Some(f)) = self.open_files.get_mut(handle) {
                    let avail = f.data.len().saturating_sub(f.pos);
                    let to_read = req_size.min(avail);
                    read_bytes.extend_from_slice(&f.data[f.pos..f.pos + to_read]);
                    f.pos += to_read;
                }

                let nread = read_bytes.len() as u16;
                let mut chk = (USB_CHECKSUM_INIT as u32) + (nread as u32);
                chk = (chk + (chk >> 16)) & 0xFFFF;
                if nread > 0 {
                    chk = fastchk16(chk, &read_bytes) as u32;
                }

                let mut resp = Vec::new();
                resp.extend_from_slice(&nread.to_le_bytes());
                resp.extend_from_slice(&(chk as u16).to_le_bytes());
                resp.extend_from_slice(&read_bytes);
                Some(resp)
            }

            CMD_FCLOSE => {
                // Header: 4 bytes [cmd, handle, chk_lo, chk_hi]
                if self.tx_buffer.len() < 4 {
                    return None;
                }
                let handle = self.tx_buffer[1] as usize;
                self.tx_buffer.drain(0..4);

                if handle < self.open_files.len() {
                    self.open_files[handle] = None;
                }
                Some(vec![0])
            }

            CMD_FWRITE => {
                // Header: 6 bytes [cmd, handle, size_lo, size_hi, chk_lo, chk_hi] + size bytes data
                if self.tx_buffer.len() < 6 {
                    return None;
                }
                let handle = self.tx_buffer[1] as usize;
                let size = u16::from_le_bytes([self.tx_buffer[2], self.tx_buffer[3]]) as usize;
                if self.tx_buffer.len() < 6 + size {
                    return None;
                }
                let content = self.tx_buffer[6..6 + size].to_vec();
                self.tx_buffer.drain(0..6 + size);

                if handle == 1 || handle == 2 {
                    self.stdout_log.extend_from_slice(&content);
                    if self.print_stdout {
                        let text = String::from_utf8_lossy(&content);
                        print!("{}", text);
                    }
                }

                // Return written size (2 bytes)
                Some((size as u16).to_le_bytes().to_vec())
            }

            CMD_RESETIO => {
                if self.tx_buffer.len() < 4 {
                    return None;
                }
                self.tx_buffer.drain(0..4);
                self.state = UsbState::WaitingFdlAck;
                None
            }

            _ => {
                // Unknown command, discard first byte to avoid infinite lock
                self.tx_buffer.remove(0);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fastchk16_calculation() {
        assert_eq!(fastchk16(0x5a5a, &[]), 0x5a5a);
        let sample = b"hello world";
        let chk = fastchk16(0x5a5a, sample);
        assert_ne!(chk, 0);
    }

    #[test]
    fn test_fdl_ack_handshake() {
        let mut usb = SprdUsb::new(false);
        assert_eq!(usb.state, UsbState::WaitingFdlAck);

        // Send authentic FDL ACK packet
        let fdl_ack = [0x7e, 0x00, 0x80, 0x00, 0x00, 0xff, 0x7f, 0x7e];
        usb.tx_buffer.extend_from_slice(&fdl_ack);
        let resp = usb.process_tx_queue();

        assert_eq!(usb.state, UsbState::Connected);
        assert_eq!(resp, Some(vec![HOST_CONNECT]));
    }

    #[test]
    fn test_pinmap_and_keymap_embedded() {
        let usb = SprdUsb::new(false);
        assert_eq!(usb.pinmap_data.len(), 2416, "Authentic Nokia 2660 pinmap.bin must be 2416 bytes");
        assert_eq!(usb.keymap_data.len(), 128, "Authentic Nokia 2660 keymap.bin must be 128 bytes");
    }

    #[test]
    fn test_cmd_fwrite_logging() {
        let mut usb = SprdUsb::new(false);
        usb.state = UsbState::Connected;

        // FWRITE header: [CMD_FWRITE, handle=1, size_lo=5, size_hi=0, chk_lo=0, chk_hi=0] + "HELLO"
        let mut packet = vec![CMD_FWRITE, 1, 5, 0, 0, 0];
        packet.extend_from_slice(b"HELLO");
        usb.tx_buffer.extend_from_slice(&packet);

        let resp = usb.process_tx_queue();
        assert_eq!(resp, Some(5u16.to_le_bytes().to_vec()));
        assert_eq!(&usb.stdout_log, b"HELLO");
    }
}
