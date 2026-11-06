// SPDX-License-Identifier: MIT
//! Spreadtrum BSL Protocol Engine CLI Utility (Rust Edition)

use clap::Parser;
use n2660_core::bsl::{build_bsl_packet, crc16_ccitt, hdlc_unstuff, parse_bsl_packet};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bsl_engine", about = "Spreadtrum BSL Protocol & CRC16 Engine")]
struct Cli {
    #[arg(short = 'c', long)]
    crc16: Option<String>,
    #[arg(short = 'f', long)]
    frame: Option<String>,
    #[arg(short = 'u', long)]
    unframe: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(text) = cli.crc16 {
        let crc = crc16_ccitt(text.as_bytes(), 0);
        println!("CRC-16 for '{}' = 0x{:04X}", text, crc);
    } else if let Some(msg) = cli.frame {
        let framed = build_bsl_packet(0, msg.as_bytes());
        println!("Framed {} bytes into {} bytes (HDLC 0x7E)", msg.len(), framed.len());
    } else if let Some(path) = cli.unframe {
        let data = fs::read(&path)?;
        let raw = if data.first() == Some(&0x7E) && data.last() == Some(&0x7E) && data.len() >= 2 {
            &data[1..data.len() - 1]
        } else {
            &data[..]
        };
        let unstuffed = hdlc_unstuff(raw);
        let pkt = parse_bsl_packet(&unstuffed).map_err(|e| e.to_string())?;
        println!("Unframed BSL packet: type=0x{:04X}, payload_len={}", pkt.pkt_type, pkt.payload.len());
    } else {
        println!("Spreadtrum BSL Protocol Engine (Rust Edition)");
        println!("Use --crc16, --frame, or --unframe to interact with protocol frames.");
    }

    Ok(())
}
