// SPDX-License-Identifier: MIT
//! ARM Boot Tags (ATAGS) Generator & Inspector Utility (Rust Edition)

use clap::{Parser, Subcommand};
use n2660_core::atags::{AtagsBuilder, AtagsParser};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "atags", about = "ARM Boot Tags (ATAGS) Generator & Inspector")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build binary ATAGS list for kernel boot handoff
    Build {
        #[arg(short = 'o', long, default_value = "atags.bin")]
        output: PathBuf,
        #[arg(long)]
        cmdline: Option<String>,
        #[arg(long, default_value = "0x80000000")]
        ram_base: String,
        #[arg(long, default_value_t = 48)]
        ram_size_mb: u32,
    },
    /// Inspect and decode existing binary ATAGS file
    Inspect {
        file: PathBuf,
        #[arg(long, default_value = "0x80000100")]
        addr: String,
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
        Commands::Build { output, cmdline, ram_base, ram_size_mb } => {
            let base = parse_u32(&ram_base);
            let size = ram_size_mb * 1024 * 1024;
            let mut builder = AtagsBuilder::new().with_ram(base, size);
            if let Some(c) = cmdline {
                builder = builder.with_cmdline(Some(c));
            }
            let data = builder.build();
            fs::write(&output, &data)?;
            println!("[ATAGS] Generated {} bytes atags list saved to '{}'.", data.len(), output.display());
        }
        Commands::Inspect { file, addr } => {
            let data = fs::read(&file)?;
            let base_addr = parse_u32(&addr);
            let tags = AtagsParser::parse(&data, base_addr).map_err(|e| e.to_string())?;
            println!("ATAGS list for '{}' (staged at 0x{:08X}):", file.display(), base_addr);
            for (i, t) in tags.iter().enumerate() {
                println!("  [{}] {:<14} (tag=0x{:08X}, size={} words): {}", i, t.name, t.tag, t.size_words, t.details);
            }
        }
    }

    Ok(())
}
