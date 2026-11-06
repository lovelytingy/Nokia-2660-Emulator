// SPDX-License-Identifier: MIT
//! Unisoc Spreadtrum .PAC Firmware Extractor & Packer (Rust Edition)

use clap::{Parser, Subcommand};
use n2660_core::pac::PacArchive;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pac_extractor", about = "Spreadtrum .PAC Firmware Container Utility")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List partitions in a .PAC archive
    List {
        pac_file: PathBuf,
    },
    /// Extract partitions from a .PAC archive
    Extract {
        pac_file: PathBuf,
        #[arg(short = 'o', long, default_value = "extracted_fw")]
        out_dir: PathBuf,
        #[arg(long)]
        id: Option<String>,
    },
    /// Dump embedded partition XML from a .PAC archive
    DumpXml {
        pac_file: PathBuf,
    },
    /// Pack individual partition files into a .PAC archive
    Pack {
        #[arg(short = 'o', long)]
        output: PathBuf,
        #[arg(long, default_value = "Nokia 2660 Flip")]
        name: String,
        #[arg(long, default_value = "TA-1480_v20.00")]
        version: String,
        #[arg(short = 'f', long)]
        file: Vec<String>, // format: ID:path:load_addr
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
        Commands::List { pac_file } => {
            let data = fs::read(&pac_file)?;
            let pac = PacArchive::parse(&data).map_err(|e| e.to_string())?;
            println!("PAC Archive: {}", pac_file.display());
            println!("  Product Name    : {}", pac.product_name);
            println!("  Product Version : {}", pac.product_version);
            println!("  PAC Version     : {}", pac.pac_version);
            println!("  Flash Type      : {}", pac.flash_type);
            println!("  Partitions      : {}", pac.files.len());
            println!("{:-<78}", "");
            println!("{:<4} {:<16} {:>10}  {:<12}  {}", "#", "ID", "SIZE", "LOAD ADDR", "FILENAME");
            println!("{:-<78}", "");
            for f in &pac.files {
                println!("{:<4} {:<16} {:>10}  0x{:08X}    {}", f.index, f.id, f.size, f.load_addr, f.filename);
            }
        }
        Commands::Extract { pac_file, out_dir, id } => {
            let data = fs::read(&pac_file)?;
            let pac = PacArchive::parse(&data).map_err(|e| e.to_string())?;
            if let Some(target_id) = id {
                let out_name = format!("{}.bin", target_id.to_lowercase());
                let target = out_dir.join(out_name);
                if !pac.extract_single(&target_id, &target)? {
                    eprintln!("[PAC] Warning: Partition ID '{}' not found in PAC file.", target_id);
                }
            } else {
                pac.extract_all(&out_dir)?;
            }
        }
        Commands::DumpXml { pac_file } => {
            let data = fs::read(&pac_file)?;
            let pac = PacArchive::parse(&data).map_err(|e| e.to_string())?;
            let xml = pac.dump_xml();
            if xml.is_empty() {
                println!("No XML partition map found in '{}'.", pac_file.display());
            } else {
                println!("{}", xml);
            }
        }
        Commands::Pack { output, name, version, file } => {
            let mut pac = PacArchive::new(&name, &version);
            for item in file {
                let parts: Vec<&str> = item.split(':').collect();
                if parts.len() >= 2 {
                    let id = parts[0];
                    let path = parts[1];
                    let load_addr = if parts.len() >= 3 { parse_u32(parts[2]) } else { 0 };
                    let data = fs::read(path)?;
                    let filename = std::path::Path::new(path).file_name().unwrap().to_str().unwrap();
                    pac.add_file(id, filename, load_addr, data);
                }
            }
            let packed = pac.pack();
            fs::write(&output, &packed)?;
            println!("[PacPacker] Successfully generated PAC archive '{}':", output.display());
            println!("  Product     : {} ({})", name, version);
            println!("  Partitions  : {}", pac.files.len());
            println!("  Total Size  : {} bytes", packed.len());
        }
    }

    Ok(())
}
