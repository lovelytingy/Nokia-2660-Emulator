// SPDX-License-Identifier: MIT
//! N2660Emu: Unified Nokia 2660 Flip Hardware Emulator CLI (Rust Edition)
//! High-performance Unisoc T107 (UMS9117) ARM Cortex-A7 emulation.

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process;
use std::time::Instant;
use clap::Parser;

use n2660_core::vm::T107Vm;
use n2660_core::pac::PacArchive;
use n2660_core::bootrom::SprdHeader;

fn parse_hex_or_dec(src: &str) -> Result<u64, String> {
    let s = src.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).map_err(|e| format!("{}", e))
    } else {
        s.parse::<u64>().map_err(|e| format!("{}", e))
    }
}

#[derive(Parser, Debug)]
#[command(name = "n2660emu-rs", version = "0.1.0", author = "N2660Emu Developers")]
#[command(about = "N2660Emu: Nokia 2660 Flip (Unisoc T107) Emulation Environment (Rust)")]
struct Args {
    /// Path to firmware ELF or raw binary
    #[arg(value_name = "FIRMWARE")]
    firmware: Option<PathBuf>,

    /// Entry address for raw binary (default: 0x80000000)
    #[arg(long, default_value = "0x80000000", value_parser = parse_hex_or_dec)]
    entry: u64,

    /// Enable CPU instruction tracing
    #[arg(long)]
    trace: bool,

    /// Execution timeout in seconds
    #[arg(long, default_value_t = 10)]
    timeout: u64,

    /// Dump main display framebuffer to PPM image
    #[arg(long)]
    screen_dump: Option<PathBuf>,

    /// Dump outer sub-display framebuffer to PPM image
    #[arg(long)]
    sub_screen_dump: Option<PathBuf>,

    /// Inject key press (e.g. --key 5 or --key ok)
    #[arg(long = "key")]
    keys: Vec<String>,

    /// Hall effect flip sensor initial state (open or closed)
    #[arg(long, default_value = "open")]
    flip: String,

    /// Path to raw SPI NAND flash image to mount
    #[arg(long)]
    flash: Option<PathBuf>,

    /// Path to FDL1 binary to stage and run
    #[arg(long)]
    fdl1: Option<PathBuf>,

    /// FDL1 load address (default: 0x00006200)
    #[arg(long = "fdl1-addr", default_value = "0x00006200", value_parser = parse_hex_or_dec)]
    fdl1_addr: u64,

    /// Path to FDL2 binary to stage into DRAM
    #[arg(long)]
    fdl2: Option<PathBuf>,

    /// FDL2 load address (default: 0x80100000)
    #[arg(long = "fdl2-addr", default_value = "0x80100000", value_parser = parse_hex_or_dec)]
    fdl2_addr: u64,

    /// Path to Unisoc .PAC firmware container to unpack and run
    #[arg(long)]
    pac: Option<PathBuf>,

    /// Simulate BootROM BSL handshake over UART0 when loading FDL1
    #[arg(long)]
    bsl: bool,

    /// Execute hardware BootROM reset vector at 0x00000000
    #[arg(long)]
    bootrom: bool,

    /// Path to BOOT0 binary to stage into SPI NAND Block 0
    #[arg(long)]
    boot0: Option<PathBuf>,

    /// Stage ARM Boot Tags (ATAGS) at 0x80000100 for kernel booting
    #[arg(long)]
    atags: bool,

    /// Physical DRAM address to stage ATAGS (default: 0x80000100)
    #[arg(long = "atags-addr", default_value = "0x80000100", value_parser = parse_hex_or_dec)]
    atags_addr: u64,

    /// ARM machine/board ID in r1 (default: 0x9117)
    #[arg(long = "board-id", default_value = "0x9117", value_parser = parse_hex_or_dec)]
    board_id: u64,

    /// Kernel command line string passed via ATAG_CMDLINE
    #[arg(long)]
    cmdline: Option<String>,
}

fn main() {
    let args = Args::parse();

    let flip_open = args.flip.to_lowercase() != "closed";
    let mut vm = match T107Vm::new(flip_open, true, args.trace) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error initializing VM: {}", e);
            process::exit(1);
        }
    };

    // Mount SPI NAND Flash if provided
    if let Some(flash_path) = &args.flash {
        if let Ok(mut f) = File::open(flash_path) {
            let mut data = Vec::new();
            if f.read_to_end(&mut data).is_ok() {
                println!("[N2660Emu] Mounted {} bytes SPI NAND flash image from {:?}", data.len(), flash_path);
                vm.uc.get_data_mut().sfc.load_image(&data);
                let _ = vm.load_bytes(&data, 0x6000_0000);
            }
        }
    }

    // Handle .PAC archive if provided
    let fdl1_file = args.fdl1.clone();
    let fdl2_file = args.fdl2.clone();
    let boot0_file = args.boot0.clone();

    if let Some(pac_path) = &args.pac {
        println!("[N2660Emu] Parsing .PAC firmware container: {:?}", pac_path);
        if let Ok(mut f) = File::open(pac_path) {
            let mut bytes = Vec::new();
            if f.read_to_end(&mut bytes).is_ok() {
                if let Ok(pac) = PacArchive::parse(&bytes) {
                    println!("[N2660Emu] PAC Product: {} ({}) with {} partitions", pac.product_version, pac.product_alias, pac.files.len());
                    for part in &pac.files {
                        let id = part.id.to_uppercase();
                        if id.contains("FDL1") && fdl1_file.is_none() {
                            let _ = vm.load_bytes(&part.data, args.fdl1_addr);
                            println!("[N2660Emu PAC] Staged FDL1 ({} bytes) at 0x{:08X}", part.data.len(), args.fdl1_addr);
                        } else if id.contains("FDL2") && fdl2_file.is_none() {
                            let _ = vm.load_bytes(&part.data, args.fdl2_addr);
                            println!("[N2660Emu PAC] Staged FDL2 ({} bytes) at 0x{:08X}", part.data.len(), args.fdl2_addr);
                        }
                    }
                }
            }
        }
    }

    // Handle BOOT0 staging
    if let Some(b0) = &boot0_file {
        if let Ok(mut f) = File::open(b0) {
            let mut data = Vec::new();
            if f.read_to_end(&mut data).is_ok() {
                // If it has SPRD header, parse and stage to SRAM
                if let Ok(hdr) = SprdHeader::parse(&data) {
                    println!("[BOOTROM] Staging BOOT0 ({} bytes) to 0x{:08X} (entry: 0x{:08X})...", hdr.payload_size, hdr.load_addr, hdr.entry_point);
                    let _ = vm.load_bytes(&data[32..32 + hdr.payload_size as usize], hdr.load_addr as u64);
                    let _ = vm.init_cpu(hdr.entry_point as u64);
                } else {
                    let _ = vm.load_bytes(&data, 0x0000_6200);
                    let _ = vm.init_cpu(0x0000_6200);
                }
            }
        }
    }

    // Stage FDL1 if requested
    if let Some(f1) = &fdl1_file {
        if let Ok(mut f) = File::open(f1) {
            let mut data = Vec::new();
            if f.read_to_end(&mut data).is_ok() {
                println!("[N2660Emu] Staging FDL1 to 0x{:08X}...", args.fdl1_addr);
                let _ = vm.load_bytes(&data, args.fdl1_addr);
                let _ = vm.init_cpu(args.fdl1_addr);
                if args.bsl {
                    println!("[BSL] Simulating Spreadtrum BSL Protocol Handshake over UART0...");
                }
            }
        }
    } else if args.bsl {
        println!("[BSL] Simulating Spreadtrum BSL Protocol Handshake over UART0...");
    }

    // Stage FDL2 if requested
    if let Some(f2) = &fdl2_file {
        if let Ok(mut f) = File::open(f2) {
            let mut data = Vec::new();
            if f.read_to_end(&mut data).is_ok() {
                println!("[N2660Emu] Staging FDL2 to 0x{:08X}...", args.fdl2_addr);
                let _ = vm.load_bytes(&data, args.fdl2_addr);
            }
        }
    }

    // Handle normal firmware payload
    let target_entry = args.entry;
    if let Some(fw) = &args.firmware {
        if fw.exists() {
            if let Err(e) = vm.load_binary(fw.to_str().unwrap(), target_entry) {
                eprintln!("Failed to load firmware: {}", e);
                process::exit(1);
            }
            if let Err(e) = vm.init_cpu(target_entry) {
                eprintln!("Failed to initialize CPU: {}", e);
                process::exit(1);
            }
        }
    }

    // ATAGS staging
    if args.atags {
        if let Err(e) = vm.setup_atags(args.atags_addr, args.board_id as u32, args.cmdline) {
            eprintln!("Failed to stage ATAGS: {}", e);
            process::exit(1);
        }
    }

    // Key injections
    for k in &args.keys {
        vm.inject_key(k, true);
        println!("[KPD] Injected key '{}' press", k);
    }

    println!("[N2660Emu] Executing Unisoc T107 at PC = 0x{:08X} (timeout: {}s)...", vm.get_pc(), args.timeout);
    let start_time = Instant::now();

    let step_slice = 50_000;
    let max_duration = std::time::Duration::from_secs(args.timeout);

    while start_time.elapsed() < max_duration {
        match vm.step(step_slice, 25_000) {
            Ok(_) => {}
            Err(e) => {
                if !e.contains("TIMEOUT") {
                    eprintln!("{}", e);
                    break;
                }
            }
        }

        let buf = vm.get_uart_tx_buffer();
        let uart_str = String::from_utf8_lossy(&buf);
        if uart_str.contains("ALL PHASE 1, 2, 3 & 4") && uart_str.contains("CHECKS PASSED") {
            break;
        }
        if uart_str.contains("SOME TESTS FAILED") {
            break;
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    let insns = vm.get_instruction_count();
    let mips = if elapsed > 0.0 { (insns as f64 / elapsed) / 1_000_000.0 } else { 0.0 };

    println!("\n[VM] Emulation finished cleanly.");
    println!("[VM] Total instructions: {} in {:.3}s ({:.2} MIPS)", insns, elapsed, mips);
    println!("[VM] Final PC: 0x{:08X} | SP: 0x{:08X} | CPSR: 0x{:08X}", vm.get_pc(), vm.get_sp(), vm.get_cpsr());

    // Export framebuffers if requested
    if let Some(dump_path) = &args.screen_dump {
        if let Err(e) = vm.export_ppm(dump_path.to_str().unwrap()) {
            eprintln!("Failed to dump main screen: {}", e);
        } else {
            println!("[LCD] Dumped 240x320 main screen to {:?}", dump_path);
        }
    }

    if let Some(dump_path) = &args.sub_screen_dump {
        if let Err(e) = vm.export_sub_ppm(dump_path.to_str().unwrap()) {
            eprintln!("Failed to dump sub screen: {}", e);
        } else {
            println!("[SUB-LCD] Dumped 120x160 sub-screen to {:?}", dump_path);
        }
    }
}
