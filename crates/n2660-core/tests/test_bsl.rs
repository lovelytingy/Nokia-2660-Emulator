// SPDX-License-Identifier: MIT
//! Integration & unit tests for Spreadtrum BSL Protocol in Rust

use n2660_core::bsl::*;

#[test]
fn test_bsl_crc16_calculation() {
    let data = b"SPRD3\x00\x01\x02\x03Hello";
    let crc = crc16_ccitt(data, 0);
    assert_eq!(crc, 0xEBB5);

    assert_eq!(crc16_ccitt(b"", 0), 0);
    assert_eq!(crc16_ccitt(b"\x00\x00\x00\x00", 0), 0);
}

#[test]
fn test_hdlc_stuff_and_unstuff() {
    let raw = vec![HDLC_FLAG, 0x10, HDLC_ESCAPE, 0x20, 0x00, HDLC_FLAG, HDLC_ESCAPE];
    let stuffed = hdlc_stuff(&raw);

    // Stuffed frame begins and ends with HDLC_FLAG (0x7E)
    assert_eq!(stuffed[0], HDLC_FLAG);
    assert_eq!(*stuffed.last().unwrap(), HDLC_FLAG);

    let unstuffed = hdlc_unstuff(&stuffed[1..stuffed.len() - 1]);
    assert_eq!(unstuffed, raw);
}

#[test]
fn test_encode_and_decode_bsl_packet() {
    let payload = vec![0x00, 0x00, 0x62, 0x00, 0x00, 0x00, 0x10, 0x00];
    let framed = build_bsl_packet(BSL_CMD_START_DATA, &payload);

    assert_eq!(framed[0], HDLC_FLAG);
    assert_eq!(*framed.last().unwrap(), HDLC_FLAG);

    let unstuffed = hdlc_unstuff(&framed[1..framed.len() - 1]);
    let parsed = parse_bsl_packet(&unstuffed).expect("Failed to parse BSL packet");
    assert_eq!(parsed.pkt_type, BSL_CMD_START_DATA);
    assert_eq!(parsed.payload, payload);
}
