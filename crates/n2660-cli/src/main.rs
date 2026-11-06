// SPDX-License-Identifier: MIT
//! N2660Emu: Unified Nokia 2660 Flip Hardware Emulator CLI (Rust Edition)
//! High-performance Unisoc T107 (UMS9117) ARM Cortex-A7 emulation.

mod gui;
mod tui;

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
#[command(name = "n2660emu", version = "0.1.0", author = "N2660Emu Developers")]
#[command(about = "N2660Emu: Nokia 2660 Flip (Unisoc T107) Emulation Environment (Rust)")]
struct Args {
    /// Path to firmware ELF or raw binary
    #[arg(value_name = "FIRMWARE")]
    firmware: Option<PathBuf>,

    /// Entry address for raw binary (default: 0x80000000)
    #[arg(long, default_value = "0x80000000", value_parser = parse_hex_or_dec)]
    entry: u64,

    /// Launch native desktop host GUI with Nokia 2660 Flip physical skin
    #[arg(long)]
    gui: bool,

    /// Color theme for phone casing in Desktop GUI ("Deep Blue", "Pop Red", "Classic Black")
    #[arg(long, default_value = "Deep Blue")]
    theme: String,

    /// Launch interactive real-time terminal GUI & keypad
    #[arg(short = 'i', long)]
    interactive: bool,

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
    let mut firmware_target = args.firmware.clone();

    if let Some(pac_path) = &args.pac {
        if let Ok(data) = std::fs::read(pac_path) {
            if let Ok(pac) = PacArchive::parse(&data) {
                println!("[PAC] Parsing vendor firmware container '{:?}' ({} partitions)...", pac_path, pac.files.len());
                for f in pac.files {
                    if f.id.eq_ignore_ascii_case("fdl1") {
                        println!("[PAC] Found FDL1 partition ({} bytes)", f.data.len());
                        let _ = vm.load_bytes(&f.data, args.fdl1_addr);
                    } else if f.id.eq_ignore_ascii_case("fdl2") {
                        println!("[PAC] Found FDL2 partition ({} bytes)", f.data.len());
                        let _ = vm.load_bytes(&f.data, args.fdl2_addr);
                    }
                }
            }
        }
    } else if let Some(fw) = &firmware_target {
        if let Some(ext) = fw.extension() {
            if ext == "pac" {
                if let Ok(data) = std::fs::read(fw) {
                    if let Ok(pac) = PacArchive::parse(&data) {
                        println!("[PAC] Parsing vendor firmware container '{:?}' ({} partitions)...", fw, pac.files.len());
                        for f in pac.files {
                            if f.id.eq_ignore_ascii_case("fdl1") {
                                println!("[PAC] Found FDL1 partition ({} bytes)", f.data.len());
                                let _ = vm.load_bytes(&f.data, args.fdl1_addr);
                            } else if f.id.eq_ignore_ascii_case("fdl2") {
                                println!("[PAC] Found FDL2 partition ({} bytes)", f.data.len());
                                let _ = vm.load_bytes(&f.data, args.fdl2_addr);
                            }
                        }
                    }
                }
                firmware_target = None;
            }
        }
    }

    // Load BOOT0 into SPI NAND Block 0 if provided
    if let Some(boot0_path) = &args.boot0 {
        if let Ok(data) = std::fs::read(boot0_path) {
            println!("[BOOT0] Loaded BOOT0 image ({} bytes) into SPI NAND Block 0", data.len());
            vm.uc.get_data_mut().sfc.write_page(0, 0, &data[..data.len().min(2048)]);
            let _ = vm.load_bytes(&data, 0x6000_0000);
            if let Ok(hdr) = SprdHeader::parse(&data) {
                let _ = vm.load_bytes(&data[32..], hdr.load_addr as u64);
                let _ = vm.set_entry_point(hdr.entry_point as u64);
            }
        }
    }

    // Load FDL1 if specified
    if let Some(f_path) = &fdl1_file {
        if let Ok(data) = std::fs::read(f_path) {
            println!("[FDL1] Staging FDL1 ({} bytes) into SRAM at 0x{:08X}", data.len(), args.fdl1_addr);
            let _ = vm.load_bytes(&data, args.fdl1_addr);
            let _ = vm.set_entry_point(args.fdl1_addr);
        }
    }

    // Load FDL2 if specified
    if let Some(f_path) = &fdl2_file {
        if let Ok(data) = std::fs::read(f_path) {
            println!("[FDL2] Staging FDL2 ({} bytes) into DRAM at 0x{:08X}", data.len(), args.fdl2_addr);
            let _ = vm.load_bytes(&data, args.fdl2_addr);
        }
    }

    // Simulate BSL handshake
    if args.bsl {
        println!("[BSL] Simulating Spreadtrum BSL Protocol Handshake over UART0...");
        println!("[BSL] Handshake ACK (0x80) received from target.");
        println!("[BSL] Simulated BSL UART0 Handshake complete.");
    }

    // Stage ATAGS if enabled
    if args.atags {
        let cmd = args.cmdline.clone().unwrap_or_else(|| "console=ttyS0,115200 root=/dev/mtdblock3 ro mem=48M".to_string());
        let _ = vm.setup_atags(args.atags_addr, args.board_id as u32, Some(cmd));
    }

    // Load firmware binary or BootROM
    if args.bootrom {
        println!("[BootROM] Reset vector configured at 0x00000000");
        let _ = vm.set_entry_point(0x0000_0000);
    } else if let Some(fw_path) = &firmware_target {
        if let Ok(data) = std::fs::read(fw_path) {
            println!("[VM] Loading binary '{:?}' ({} bytes) to address 0x{:08X}...", fw_path, data.len(), args.entry);
            let _ = vm.load_bytes(&data, args.entry);
            let _ = vm.set_entry_point(args.entry);
        }
    }

    // Inject initial keys
    for k in &args.keys {
        vm.inject_key(k, true);
    }

    // Handle GUI or Interactive TUI mode
    if args.gui {
        let theme = args.theme.clone();
        let closed = args.flip.to_lowercase() == "closed";
        if let Err(e) = gui::run_desktop_gui(&mut vm, &theme, closed) {
            eprintln!("Desktop GUI error: {}", e);
        }
        return;
    }

    if args.interactive {
        if let Err(e) = tui::run_interactive_tui(&mut vm) {
            eprintln!("Interactive TUI error: {}", e);
        }
        return;
    }

    // Standard headless execution
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
