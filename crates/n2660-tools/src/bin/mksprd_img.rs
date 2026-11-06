// SPDX-License-Identifier: MIT
//! Unisoc SPRD BOOT0 Image Packaging & NAND Flash Utility (Rust Edition)

use clap::{Parser, Subcommand};
use n2660_core::bootrom::SprdHeader;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mksprd_img", about = "SPRD BOOT0 Image Packaging & Flash Utility")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a BOOT0 image with a 32-byte SPRD header
    Create {
        #[arg(short = 'i', long)]
        input: PathBuf,
        #[arg(short = 'o', long)]
        output: PathBuf,
        #[arg(long, default_value = "0x00006200")]
        load_addr: String,
        #[arg(long, default_value = "0x00006200")]
        entry: String,
    },
    /// Verify an existing BOOT0 image and print its header metadata
    Verify {
        file: PathBuf,
    },
    /// Inject a BOOT0 image into an SPI NAND flash binary
    Inject {
        #[arg(short = 'i', long)]
        boot0: PathBuf,
        #[arg(short = 'f', long)]
        flash: PathBuf,
        #[arg(short = 'b', long, default_value_t = 0)]
        block: usize,
    },
}

fn parse_u32(s: &str) -> u32 {
    let clean = s.trim();
    if clean.starts_with("0x") || clean.starts_with("0X") {
        u32::from_str_radix(&clean[2..], 16).unwrap_or(0)
    } else {
        clean.parse::<u32>().unwrap_or(0)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { input, output, load_addr, entry } => {
            let payload = fs::read(&input)?;
            let l_addr = parse_u32(&load_addr);
            let e_point = parse_u32(&entry);
            let image = SprdHeader::create(&payload, l_addr, e_point);
            fs::write(&output, image)?;
            println!("[mksprd_img] Created BOOT0 image '{}' ({} bytes, entry 0x{:08X})", output.display(), payload.len() + 32, e_point);
        }
        Commands::Verify { file } => {
            let data = fs::read(&file)?;
            let hdr = SprdHeader::parse(&data).map_err(|e| e.to_string())?;
            println!("SPRD BOOT0 Header for '{}':", file.display());
            println!("  Magic        : 0x{:08X}", hdr.magic);
            println!("  Version      : {}", hdr.version);
            println!("  Load Addr    : 0x{:08X}", hdr.load_addr);
            println!("  Payload Size : {} bytes", hdr.payload_size);
            println!("  Entry Point  : 0x{:08X}", hdr.entry_point);
            println!("  Checksum     : 0x{:08X}", hdr.checksum);
            println!("  Header Csum  : 0x{:08X}", hdr.header_checksum);
        }
        Commands::Inject { boot0, flash, block } => {
            let data = fs::read(&boot0)?;
            SprdHeader::inject_into_flash(&data, &flash, block, 128 * 1024 * 1024)?;
            println!("[mksprd_img] Injected '{}' into flash '{}' at block {}", boot0.display(), flash.display(), block);
        }
    }

    Ok(())
}
