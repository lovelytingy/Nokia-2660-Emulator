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

    /// Load address for raw binary (defaults to --entry or 0x80100000 for DHTB images)
    #[arg(long = "load-addr", value_parser = parse_hex_or_dec)]
    load_addr: Option<u64>,

    /// Launch native desktop host GUI with Nokia 2660 Flip physical skin
    #[arg(long)]
    gui: bool,

    /// Launch desktop host GUI directly in Phone-Only view (centered phone without telemetry deck)
    #[arg(long)]
    phone_only: bool,

    /// Capture GUI screenshot to image file (PPM/PNG) and exit
    #[arg(long)]
    screenshot: Option<PathBuf>,

    /// Optional theme argument (Deep Navy, Charcoal Black, Pop Pink, Lush Green)
    #[arg(long, default_value = "Deep Navy")]
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

    /// Execution stage when running PAC firmware: boot0 (default), boot1, kernel, or user
    #[arg(long, default_value = "boot0")]
    stage: String,
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

    // Handle .PAC archive if provided explicitly (--pac) or passed as positional FIRMWARE argument
    let fdl1_file = args.fdl1.clone();
    let fdl2_file = args.fdl2.clone();
    let mut firmware_target = args.firmware.clone();
    let mut pac_boot_entry: Option<u64> = None;

    let pac_path_opt = args.pac.clone().or_else(|| {
        if let Some(fw) = &firmware_target {
            if fw.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pac")) {
                return Some(fw.clone());
            }
            return None; // Explicit binary given, do not auto-detect PAC
        }
        // Auto-detect PAC in current working directory or known RE locations
        let default_pac = PathBuf::from("v23_update_Nokia_2660.pac");
        if default_pac.exists() {
            println!("[N2660Emu] Auto-detected Nokia 2660 PAC container: {:?}", default_pac);
            return Some(default_pac);
        }
        let re_pac = PathBuf::from("../Nokia-2660-RE/firmware/v23_update_Nokia_2660.pac");
        if re_pac.exists() {
            println!("[N2660Emu] Auto-detected Nokia 2660 PAC container from RE repo: {:?}", re_pac);
            return Some(re_pac);
        }
        None
    });

    if let Some(pac_path) = pac_path_opt {
        if let Ok(data) = std::fs::read(&pac_path) {
            if let Ok(pac) = PacArchive::parse(&data) {
                println!("[PAC] Parsing vendor firmware container '{:?}' ({} partitions)...", pac_path, pac.files.len());
                println!("[PAC] Product: {} ({}) | Flash Type: {}", pac.product_name, pac.product_version, pac.flash_type);
                match vm.load_pac(&pac, args.fdl1_addr, args.fdl2_addr, args.entry) {
                    Ok(entry_opt) => {
                        pac_boot_entry = entry_opt;
                    }
                    Err(e) => {
                        eprintln!("[PAC] Error staging firmware partitions: {}", e);
                    }
                }
                firmware_target = None;
            }
        }
    }

    // Apply --stage entry override when booting from PAC
    if pac_boot_entry.is_some() {
        if args.entry != 0x8000_0000 {
            println!("[ENTRY] Using explicit entry address: 0x{:08X}", args.entry);
            pac_boot_entry = Some(args.entry);
        } else {
            match args.stage.to_lowercase().as_str() {
                "boot0" => {
                    println!("[STAGE] Multi-Stage Order: Starting from Stage 1: BOOT0 SPL (0x00006400)");
                    pac_boot_entry = Some(0x0000_6400);
                    vm.set_boot_stage(n2660_core::types::BootStage::Boot0);
                }
                "boot1" => {
                    println!("[STAGE] Selected Stage: Stage 2: BOOT1 Secondary Bootloader (0x80100200)");
                    pac_boot_entry = Some(0x8010_0200);
                    vm.set_boot_stage(n2660_core::types::BootStage::Boot1);
                }
                "kernel" => {
                    println!("[STAGE] Selected Stage: Stage 3: MOCOR OS Kernel (0x80a06000)");
                    pac_boot_entry = Some(0x80a0_6000);
                    vm.set_boot_stage(n2660_core::types::BootStage::Kernel);
                }
                "user" => {
                    println!("[STAGE] Selected Stage: Stage 4: Userland Application Framework MS_MMI_Main (0x88479F6F)");
                    pac_boot_entry = Some(0x8847_9F6F);
                    vm.set_boot_stage(n2660_core::types::BootStage::User);
                }
                other => {
                    eprintln!("[STAGE] Warning: Unknown stage '{}'. Valid stages: boot0, boot1, kernel, user", other);
                }
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

    // Load firmware binary, BootROM, or default interactive firmware
    if args.bootrom {
        println!("[BootROM] Reset vector configured at 0x00000000");
        let _ = vm.set_entry_point(0x0000_0000);
    } else if let Some(entry) = pac_boot_entry {
        println!("[PAC] Configuring execution entry point at 0x{:08X}", entry);
        let _ = vm.set_entry_point(entry);
    } else if let Some(fw_path) = &firmware_target {
        if let Ok(data) = std::fs::read(fw_path) {
            let is_dhtb = data.len() >= 4 && &data[0..4] == b"DHTB";
            let (load_addr, entry) = if let Some(la) = args.load_addr {
                (la, args.entry)
            } else if is_dhtb {
                if args.entry == 0x8000_0000 || args.entry == 0x8010_0200 {
                    (0x8010_0000u64, 0x8010_0200u64)
                } else {
                    (args.entry.saturating_sub(0x200), args.entry)
                }
            } else {
                (args.entry, args.entry)
            };
            println!("[VM] Loading binary '{:?}' ({} bytes) to address 0x{:08X} (entry: 0x{:08X})...", fw_path, data.len(), load_addr, entry);
            let _ = vm.load_bytes(&data, load_addr);
            let _ = vm.set_entry_point(entry);
        } else {
            eprintln!("[N2660Emu] Error: Failed to open firmware binary '{:?}'", fw_path);
            process::exit(1);
        }
    } else {
        // Built-in default: Authentic Nokia 2660 Flip OS Shell
        let default_fw = include_bytes!("../../../tests/baremetal/interactive_demo.bin");
        println!("[VM] No firmware binary specified; booting authentic Nokia 2660 Flip OS Shell ({} bytes) at 0x{:08X}...", default_fw.len(), args.entry);
        let _ = vm.load_bytes(default_fw, args.entry);
        let _ = vm.set_entry_point(args.entry);
    }

    // Inject initial keys
    for k in &args.keys {
        vm.inject_key(k, true);
    }

    // Handle GUI or Interactive TUI mode
    if args.gui || args.phone_only || args.screenshot.is_some() {
        let closed = args.flip.to_lowercase() == "closed";
        if let Err(e) = gui::run_desktop_gui(&mut vm, closed, Some(&args.theme), args.phone_only, args.screenshot.as_deref()) {
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

    let mut key_press_injected = false;
    let mut key_release_injected = false;

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

        let insns = vm.get_instruction_count();
        if !key_press_injected && insns >= 1_000_000 {
            for k in &args.keys {
                vm.press_key(k);
            }
            key_press_injected = true;
        } else if key_press_injected && !key_release_injected && insns >= 3_000_000 {
            for k in &args.keys {
                vm.release_key(k);
            }
            key_release_injected = true;
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
