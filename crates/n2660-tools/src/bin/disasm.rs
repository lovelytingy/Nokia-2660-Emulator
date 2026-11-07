// SPDX-License-Identifier: MIT
//! Capstone Disassembler with Unisoc T107 Symbol Annotations (Rust Edition)

use capstone::prelude::*;
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "disasm", about = "ARMv7-A / Thumb Disassembler with Unisoc T107 Annotations")]
struct Cli {
    binary: PathBuf,
    #[arg(short = 'b', long, default_value = "0x80000000")]
    base: String,
    #[arg(short = 'o', long, default_value = "0x0")]
    offset: String,
    #[arg(short = 'n', long, default_value_t = 40)]
    count: usize,
    #[arg(short = 't', long)]
    thumb: bool,
}

fn parse_u64(s: &str) -> u64 {
    let clean = s.trim();
    if clean.starts_with("0x") || clean.starts_with("0X") {
        u64::from_str_radix(&clean[2..], 16).unwrap_or(0)
    } else {
        clean.parse::<u64>().unwrap_or(0)
    }
}

fn get_t107_symbols() -> HashMap<u64, &'static str> {
    let mut m = HashMap::new();
    m.insert(0x0000_0000, "IRAM_BASE (Internal SRAM)");
    m.insert(0x0000_6200, "FDL1_ENTRY_POINT");
    m.insert(0x4000_0000, "PERIPH_AON_BASE");
    m.insert(0x5000_0000, "SPRD_UART0_TXD");
    m.insert(0x5000_0004, "SPRD_UART0_RXD");
    m.insert(0x5000_0008, "SPRD_UART0_STS0");
    m.insert(0x5000_000C, "SPRD_UART0_STS1 (FIFO Status)");
    m.insert(0x5000_0010, "SPRD_UART0_IEN");
    m.insert(0x5001_0000, "SPRD_UART1_BASE");
    m.insert(0x5002_0000, "SPRD_UART2_BASE");
    m.insert(0x5003_0000, "SPRD_TIMERS_BASE");
    m.insert(0x5004_0000, "SPRD_GPIO_BASE");
    m.insert(0x5005_0000, "SPRD_INTC_BASE");
    m.insert(0x5006_0000, "SPRD_KPD_BASE");
    m.insert(0x5008_0000, "SPRD_SPI0_LCD_BASE");
    m.insert(0x5009_0000, "SPRD_SPI1_SUBLCD_BASE");
    m.insert(0x500A_0000, "SPRD_SFC_BASE");
    m.insert(0x500C_0000, "SPRD_AUDIO_BASE");
    m.insert(0x6000_0000, "FLASH_DIRECT_BASE (XIP)");
    m.insert(0x8000_0000, "DRAM_BASE (LPDDR2)");
    m.insert(0x8002_0000, "BOOT1_ENTRY_POINT");
    m.insert(0x8004_0000, "MOCOR_OS_ENTRY");
    m.insert(0x8010_0000, "FDL2_ENTRY_POINT");
    m
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let code_all = fs::read(&cli.binary)?;
    let base_addr = parse_u64(&cli.base);
    let offset = parse_u64(&cli.offset) as usize;
    let code = if offset < code_all.len() { &code_all[offset..] } else { &code_all[..] };

    let mode = if cli.thumb {
        arch::arm::ArchMode::Thumb
    } else {
        arch::arm::ArchMode::Arm
    };

    let cs = Capstone::new()
        .arm()
        .mode(mode)
        .detail(true)
        .build()
        .map_err(|e| e.to_string())?;

    let syms = get_t107_symbols();

    let start_addr = base_addr + offset as u64;
    println!(
        "Disassembling '{}' at 0x{:08X} (base: 0x{:08X}, offset: 0x{:X}) ({} mode):",
        cli.binary.display(),
        start_addr,
        base_addr,
        offset,
        if cli.thumb { "Thumb" } else { "ARM" }
    );
    println!("{:-<72}", "");

    let insns = cs.disasm_all(&code, start_addr).map_err(|e| e.to_string())?;
    for (i, insn) in insns.iter().enumerate() {
        if cli.count > 0 && i >= cli.count {
            break;
        }

        let addr = insn.address();
        let mnemonic = insn.mnemonic().unwrap_or("???");
        let op_str = insn.op_str().unwrap_or("");
        let bytes_hex: String = insn.bytes().iter().map(|b| format!("{:02x} ", b)).collect();

        let mut annotation = "";
        for (sym_addr, name) in &syms {
            let hex_target = format!("{:x}", sym_addr);
            if op_str.to_lowercase().contains(&hex_target) {
                annotation = name;
                break;
            }
        }

        if !annotation.is_empty() {
            println!("0x{:08X}:  {:<12}  {:<6} {:<24} ; {}", addr, bytes_hex, mnemonic, op_str, annotation);
        } else {
            println!("0x{:08X}:  {:<12}  {:<6} {}", addr, bytes_hex, mnemonic, op_str);
        }
    }

    Ok(())
}
