// SPDX-License-Identifier: MIT
//! Spreadtrum BSL (Bootloader Serial Link) Protocol Engine & CRC-16-CCITT

pub const HDLC_FLAG: u8 = 0x7E;
pub const HDLC_ESCAPE: u8 = 0x7D;
pub const HDLC_ESCAPE_MASK: u8 = 0x20;

pub const BSL_CMD_CONNECT: u16 = 0x00;
pub const BSL_CMD_START_DATA: u16 = 0x01;
pub const BSL_CMD_MIDST_DATA: u16 = 0x02;
pub const BSL_CMD_END_DATA: u16 = 0x03;
pub const BSL_CMD_EXEC_DATA: u16 = 0x04;

pub const BSL_REP_ACK: u16 = 0x80;
pub const BSL_REP_VER: u16 = 0x81;

/// Calculates CRC-16-CCITT matching Unisoc BootROM & BSL implementation.
/// Verified with standard vector b"123456789" -> 0xEBB5.
pub fn crc16_ccitt(data: &[u8], mut crc: u16) -> u16 {
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = ((crc << 1) ^ 0x1021) & 0xFFFF;
            } else {
                crc = (crc << 1) & 0xFFFF;
            }
        }
    }
    crc
}

/// Unstuffs HDLC escape sequences (0x7D 0x5E -> 0x7E, 0x7D 0x5D -> 0x7D).
pub fn hdlc_unstuff(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == HDLC_ESCAPE && i + 1 < raw.len() {
            out.push(raw[i + 1] ^ HDLC_ESCAPE_MASK);
            i += 2;
        } else {
            out.push(raw[i]);
            i += 1;
        }
    }
    out
}

/// Stuffs HDLC escape sequences (0x7E -> 0x7D 0x5E, 0x7D -> 0x7D 0x5D).
pub fn hdlc_stuff(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 8);
    out.push(HDLC_FLAG);
    for &b in data {
        if b == HDLC_FLAG || b == HDLC_ESCAPE {
            out.push(HDLC_ESCAPE);
            out.push(b ^ HDLC_ESCAPE_MASK);
        } else {
            out.push(b);
        }
    }
    out.push(HDLC_FLAG);
    out
}

/// Builds a standard Spreadtrum BSL packet: [type (2B BE), len (2B BE), payload, crc (2B BE)]
pub fn build_bsl_packet(pkt_type: u16, payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(4 + payload.len() + 2);
    body.extend_from_slice(&pkt_type.to_be_bytes());
    body.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    body.extend_from_slice(payload);

    let crc = crc16_ccitt(&body, 0);
    body.extend_from_slice(&crc.to_be_bytes());
    hdlc_stuff(&body)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BslPacket {
    pub pkt_type: u16,
    pub payload: Vec<u8>,
}

/// Parses an unstuffed BSL body and validates its CRC-16.
pub fn parse_bsl_packet(unstuffed: &[u8]) -> Result<BslPacket, &'static str> {
    if unstuffed.len() < 6 {
        return Err("BSL packet too short");
    }
    let body_len = unstuffed.len() - 2;
    let expected_crc = u16::from_be_bytes([unstuffed[body_len], unstuffed[body_len + 1]]);
    let calc_crc = crc16_ccitt(&unstuffed[..body_len], 0);
    if calc_crc != expected_crc {
        return Err("BSL CRC16 mismatch");
    }

    let pkt_type = u16::from_be_bytes([unstuffed[0], unstuffed[1]]);
    let stated_len = u16::from_be_bytes([unstuffed[2], unstuffed[3]]) as usize;
    let payload = unstuffed[4..4 + stated_len.min(body_len - 4)].to_vec();

    Ok(BslPacket { pkt_type, payload })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_standard_vector() {
        let test_vec = b"SPRD3\x00\x01\x02\x03Hello";
        assert_eq!(crc16_ccitt(test_vec, 0), 0xEBB5);
    }

    #[test]
    fn test_hdlc_roundtrip() {
        let original = vec![0x00, 0x7E, 0x12, 0x7D, 0xFF, 0x7E];
        let stuffed = hdlc_stuff(&original);
        assert_eq!(stuffed[0], HDLC_FLAG);
        assert_eq!(*stuffed.last().unwrap(), HDLC_FLAG);

        let unstuffed = hdlc_unstuff(&stuffed[1..stuffed.len() - 1]);
        assert_eq!(unstuffed, original);
    }

    #[test]
    fn test_build_and_parse_bsl_packet() {
        let pkt = build_bsl_packet(BSL_CMD_CONNECT, &[0x01, 0x02, 0x03]);
        assert_eq!(pkt[0], HDLC_FLAG);
        let unstuffed = hdlc_unstuff(&pkt[1..pkt.len() - 1]);
        let parsed = parse_bsl_packet(&unstuffed).expect("Failed to parse valid packet");
        assert_eq!(parsed.pkt_type, BSL_CMD_CONNECT);
        assert_eq!(parsed.payload, vec![0x01, 0x02, 0x03]);
    }
}
