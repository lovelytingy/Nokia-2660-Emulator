// SPDX-License-Identifier: MIT
//! Integration & unit tests for Spreadtrum .PAC firmware container in Rust

use n2660_core::pac::PacArchive;
use std::fs;

#[test]
fn test_pac_pack_and_parse_header() {
    let mut pac = PacArchive::new("Nokia 2660 Flip", "TA-1480_v20.00");
    pac.add_file("FDL1", "fdl1.bin", 0x0000_6200, vec![1, 2, 3, 4]);
    pac.add_file("BOOT0", "boot0.bin", 0x0000_6200, vec![5, 6, 7, 8]);

    let packed = pac.pack();
    let parsed = PacArchive::parse(&packed).expect("Failed to parse PAC");

    assert_eq!(parsed.product_name, "Nokia 2660 Flip");
    assert_eq!(parsed.product_version, "TA-1480_v20.00");
    assert_eq!(parsed.files.len(), 2);
    assert_eq!(parsed.files[0].id, "FDL1");
    assert_eq!(parsed.files[0].data, vec![1, 2, 3, 4]);
    assert_eq!(parsed.files[1].id, "BOOT0");
    assert_eq!(parsed.files[1].data, vec![5, 6, 7, 8]);
}

#[test]
fn test_pac_extract_all_and_single() {
    let mut pac = PacArchive::new("Nokia 2660 Flip", "TA-1480");
    pac.add_file("FDL1", "fdl1.bin", 0x0000_6200, vec![10, 20, 30, 40]);
    pac.add_file("PARTITION_XML", "partition.xml", 0, b"<Partitions/>".to_vec());

    let td = std::env::temp_dir().join("test_pac_extract");
    let _ = fs::remove_dir_all(&td);

    let count = pac.extract_all(&td).expect("Extract all failed");
    assert_eq!(count, 2);

    let fdl1_content = fs::read(td.join("fdl1.bin")).expect("Read fdl1 failed");
    assert_eq!(fdl1_content, vec![10, 20, 30, 40]);

    let xml = pac.dump_xml();
    assert_eq!(xml, "<Partitions/>");

    let _ = fs::remove_dir_all(td);
}
