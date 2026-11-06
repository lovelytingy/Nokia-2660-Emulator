// SPDX-License-Identifier: MIT
//! T107 Virtual Machine powered by Unicorn ARM Cortex-A7 engine

use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use unicorn_engine::{
    Unicorn, RegisterARM,
    unicorn_const::{Arch, Mode, HookType, Prot, MemType},
};

use crate::types::*;
use crate::peripherals::*;
use crate::atags::AtagsBuilder;
use crate::bootrom::SprdHeader;
use crate::pac::PacArchive;

pub struct VmContext {
    pub uart: SprdUart,
    pub timer: SprdTimer,
    pub gpio: SprdGpio,
    pub intc: SprdIntc,
    pub kpd: SprdKpd,
    pub display: SprdDisplaySubsystem,
    pub sfc: SprdSfc,
    pub audio: SprdAudio,
    pub misc: SprdMisc,
    pub logged_traps: HashSet<u32>,
    pub instruction_count: u64,
    pub trace: bool,
    pub timer_start_insn: u64,
}

pub struct T107Vm {
    pub uc: Unicorn<'static, VmContext>,
    pub entry_point: u64,
}

impl T107Vm {
    pub fn new(flip_open: bool, print_stdout: bool, trace: bool) -> Result<Self, String> {
        let ctx = VmContext {
            uart: SprdUart::new(print_stdout),
            timer: SprdTimer::new(),
            gpio: SprdGpio::new(flip_open),
            intc: SprdIntc::new(),
            kpd: SprdKpd::new(),
            display: SprdDisplaySubsystem::new(),
            sfc: SprdSfc::new(),
            audio: SprdAudio::new(),
            misc: SprdMisc::new(),
            logged_traps: HashSet::new(),
            instruction_count: 0,
            trace,
            timer_start_insn: 0,
        };

        let mut uc = Unicorn::new_with_data(Arch::ARM, Mode::ARM, ctx)
            .map_err(|e| format!("Failed to create ARM Unicorn instance: {:?}", e))?;

        // 1. Map Internal SRAM (64 KB at 0x0000_0000)
        uc.mem_map(SRAM_BASE, SRAM_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map SRAM at 0x{:X}: {:?}", SRAM_BASE, e))?;

        // 2. Map System High DRAM (48 MB at 0x8000_0000)
        uc.mem_map(DRAM_BASE, DRAM_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map DRAM at 0x{:X}: {:?}", DRAM_BASE, e))?;

        // 3. Map Direct Flash Window (128 MB at 0x6000_0000)
        uc.mem_map(FLASH_DIRECT_BASE, FLASH_DIRECT_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map Flash XIP at 0x{:X}: {:?}", FLASH_DIRECT_BASE, e))?;

        // Fill Flash direct window with default 0xFF
        let blank_chunk = vec![0xFFu8; 64 * 1024];
        for off in (0..FLASH_DIRECT_SIZE).step_by(blank_chunk.len()) {
            let _ = uc.mem_write(FLASH_DIRECT_BASE + off as u64, &blank_chunk);
        }

        // 4. Map MMIO Address Space (1 MB at 0x5000_0000)
        uc.mem_map(MMIO_BASE, MMIO_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map MMIO at 0x{:X}: {:?}", MMIO_BASE, e))?;

        // 4b. Map PL011 UART Fallback (4 KB at 0x0900_0000 for QEMU virt payload compatibility)
        uc.mem_map(PL011_UART_BASE, PL011_UART_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map PL011 UART at 0x{:X}: {:?}", PL011_UART_BASE, e))?;

        // 5. Map Unmapped Peripheral Trap Window (64 KB at 0x4000_0000)
        uc.mem_map(TRAP_BASE, TRAP_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map Trap window at 0x{:X}: {:?}", TRAP_BASE, e))?;

        // Setup Instruction Hook & IRQ Dispatcher
        uc.add_code_hook(0, 0xFFFF_FFFF, |uc, addr, _size| {
            let ctx = uc.get_data_mut();
            ctx.instruction_count += 1;
            if ctx.trace {
                println!("[TRACE] PC = 0x{:08X} (insn #{})", addr, ctx.instruction_count);
            }

            // Check for pending unmasked IRQ to dispatch to CPU exception vector
            let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
            if (cpsr & 0x80) == 0 {
                let pending = uc.get_data().intc.get_irq_masked_status();
                if pending != 0 {
                    let vector_base = 0x8000_0000u64;
                    let svc_cpsr = cpsr;
                    let irq_cpsr = (svc_cpsr & !0x1F) | 0x12 | 0x80;
                    let _ = uc.reg_write(RegisterARM::CPSR, irq_cpsr);
                    let _ = uc.reg_write(RegisterARM::SPSR, svc_cpsr);
                    let _ = uc.reg_write(RegisterARM::LR, addr + 4);
                    let _ = uc.reg_write(RegisterARM::PC, vector_base + 0x18);
                }
            }
        }).map_err(|e| format!("Failed to add code hook: {:?}", e))?;

        // Setup MMIO Read Hook (0x5000_0000..0x5010_0000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            MMIO_BASE,
            MMIO_BASE + MMIO_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let base = (addr & 0xFFFF_F000) as u32;
                let offset = (addr & 0x0FFF) as u32;

                let (val, known) = {
                    let ctx = uc.get_data_mut();
                    match base {
                        SPRD_UART0_BASE => (ctx.uart.read(offset), true),
                        SPRD_TIMER_BASE => (ctx.timer.read(offset), true),
                        SPRD_GPIO_BASE  => (ctx.gpio.read(offset), true),
                        SPRD_INTC_BASE  => (ctx.intc.read(offset), true),
                        SPRD_KPD_BASE   => (ctx.kpd.read(offset), true),
                        SPRD_SFC_BASE   => (ctx.sfc.read(offset), true),
                        SPRD_AUDIO_BASE => (ctx.audio.read(offset), true),
                        SPRD_SPI0_BASE  => (ctx.display.read_spi0(offset), true),
                        SPRD_SPI1_BASE  => (ctx.display.read_spi1(offset), true),
                        SPRD_ADI_BASE | SPRD_CLK_BASE | SPRD_PIN_BASE | SPRD_EIC_BASE | SPRD_I2C_BASE | SPRD_UART1_BASE | SPRD_UART2_BASE => {
                            (ctx.misc.read(base, offset), true)
                        }
                        _ => (0, false),
                    }
                };

                if known {
                    let bytes = val.to_le_bytes();
                    let slice = &bytes[..size.min(4)];
                    let _ = uc.mem_write(addr, slice);
                } else {
                    let pc = uc.reg_read(RegisterARM::PC).unwrap_or(0);
                    let is_new = uc.get_data_mut().logged_traps.insert(addr as u32);
                    if is_new {
                        println!("\n[N2660Emu MMIO TRAP] Unknown peripheral READ at 0x{:08X} (size {}) from PC 0x{:08X}", addr, size, pc);
                        println!("[INFO] Unmapped MMIO read returned: 0x00000000\n");
                    }
                    let _ = uc.mem_write(addr, &vec![0u8; size]);
                }
                true
            },
        ).map_err(|e| format!("Failed to add MMIO read hook: {:?}", e))?;

        // Setup MMIO Write Hook (0x5000_0000..0x5010_0000)
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            MMIO_BASE,
            MMIO_BASE + MMIO_SIZE as u64,
            |uc, _mem_type, addr, size, value| {
                let base = (addr & 0xFFFF_F000) as u32;
                let offset = (addr & 0x0FFF) as u32;
                let val = (value as u64 & 0xFFFF_FFFF) as u32;
                let mut sync_data: Option<(usize, Vec<u8>)> = None;

                let known = {
                    let ctx = uc.get_data_mut();
                    match base {
                        SPRD_UART0_BASE => {
                            ctx.uart.write(offset, val);
                            true
                        }
                        SPRD_TIMER_BASE => {
                            ctx.timer.write(offset, val);
                            true
                        }
                        SPRD_GPIO_BASE => {
                            ctx.gpio.write(offset, val);
                            true
                        }
                        SPRD_INTC_BASE => {
                            ctx.intc.write(offset, val);
                            true
                        }
                        SPRD_KPD_BASE => {
                            ctx.kpd.write(offset, val);
                            true
                        }
                        SPRD_SPI0_BASE => {
                            ctx.display.write_spi0(offset, val);
                            true
                        }
                        SPRD_SPI1_BASE => {
                            ctx.display.write_spi1(offset, val);
                            true
                        }
                        SPRD_SFC_BASE => {
                            ctx.sfc.write(offset, val);
                            if let Some((flash_offset, data)) = ctx.sfc.pending_flash_sync.take() {
                                sync_data = Some((flash_offset, data));
                            }
                            true
                        }
                        SPRD_AUDIO_BASE => {
                            ctx.audio.write(offset, val);
                            true
                        }
                        SPRD_ADI_BASE | SPRD_CLK_BASE | SPRD_PIN_BASE | SPRD_EIC_BASE | SPRD_I2C_BASE | SPRD_UART1_BASE | SPRD_UART2_BASE => {
                            ctx.misc.write(base, offset, val);
                            true
                        }
                        _ => false,
                    }
                };

                if let Some((flash_offset, data)) = sync_data {
                    let _ = uc.mem_write(FLASH_DIRECT_BASE + flash_offset as u64, &data);
                }

                if !known {
                    let pc = uc.reg_read(RegisterARM::PC).unwrap_or(0);
                    let is_new = uc.get_data_mut().logged_traps.insert(addr as u32);
                    if is_new {
                        println!("\n[N2660Emu MMIO TRAP] Unknown peripheral WRITE at 0x{:08X} (val 0x{:08X}, size {}) from PC 0x{:08X}\n", addr, val, size, pc);
                    }
                }
                true
            },
        ).map_err(|e| format!("Failed to add MMIO write hook: {:?}", e))?;

        // Setup Unmapped Trap Read/Write Hook for 0x4000_0000 range
        uc.add_mem_hook(
            HookType::MEM_READ | HookType::MEM_WRITE,
            TRAP_BASE,
            TRAP_BASE + TRAP_SIZE as u64,
            |uc, mem_type, addr, size, value| {
                let pc = uc.reg_read(RegisterARM::PC).unwrap_or(0);
                let is_new = uc.get_data_mut().logged_traps.insert(addr as u32);
                if mem_type == MemType::READ {
                    if is_new {
                        println!("\n[N2660Emu MMIO TRAP] Unknown peripheral READ at 0x{:08X} (size {}) from PC 0x{:08X}", addr, size, pc);
                        println!("[INFO] Unmapped MMIO read returned: 0x00000000\n");
                    }
                    let _ = uc.mem_write(addr, &vec![0u8; size]);
                } else {
                    if is_new {
                        println!("\n[N2660Emu MMIO TRAP] Unknown peripheral WRITE at 0x{:08X} (val 0x{:08X}, size {}) from PC 0x{:08X}\n", addr, value, size, pc);
                    }
                }
                true
            },
        ).map_err(|e| format!("Failed to add TRAP hook: {:?}", e))?;

        Ok(Self {
            uc,
            entry_point: 0x8000_0000,
        })
    }

    pub fn load_binary(&mut self, path: &str, addr: u64) -> Result<usize, String> {
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to open binary file '{}': {}", path, e))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| format!("Failed to read binary file '{}': {}", path, e))?;

        println!("[VM] Loading binary '{}' ({} bytes) to address 0x{:08X}...", path, buf.len(), addr);
        self.load_bytes(&buf, addr)?;
        Ok(buf.len())
    }

    pub fn load_bytes(&mut self, bytes: &[u8], addr: u64) -> Result<(), String> {
        self.uc.mem_write(addr, bytes)
            .map_err(|e| format!("Failed to write {} bytes to 0x{:08X}: {:?}", bytes.len(), addr, e))?;
        Ok(())
    }

    pub fn read_bytes(&self, addr: u64, size: usize) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; size];
        self.uc.mem_read(addr, &mut buf)
            .map_err(|e| format!("Failed to read {} bytes from 0x{:08X}: {:?}", size, addr, e))?;
        Ok(buf)
    }

    pub fn init_cpu(&mut self, entry_point: u64) -> Result<(), String> {
        self.entry_point = entry_point;
        // Stack pointer: SRAM (0x0000_FF00) for BootROM/FDL1, High DRAM (0x8003_FF00) for OS/Kernel
        let sp = if entry_point < (SRAM_BASE + SRAM_SIZE as u64) {
            0x0000_FF00
        } else {
            0x8003_FF00
        };
        self.uc.reg_write(RegisterARM::SP, sp)
            .map_err(|e| format!("Failed to write SP: {:?}", e))?;
        // ARM Supervisor (SVC) mode, IRQ/FIQ masked: 0x600000D3
        self.uc.reg_write(RegisterARM::CPSR, 0x6000_00D3)
            .map_err(|e| format!("Failed to write CPSR: {:?}", e))?;
        // Enable VFP / NEON via CP15 CPACR and FPEXC
        let _ = self.uc.reg_write(RegisterARM::C1_C0_2, 0x00F0_0000);
        let _ = self.uc.reg_write(RegisterARM::FPEXC, 0x4000_0000);

        self.uc.reg_write(RegisterARM::PC, entry_point)
            .map_err(|e| format!("Failed to write PC: {:?}", e))?;
        Ok(())
    }

    pub fn set_entry_point(&mut self, entry_point: u64) -> Result<(), String> {
        self.init_cpu(entry_point)
    }

    pub fn load_pac(
        &mut self,
        pac: &PacArchive,
        fdl1_addr: u64,
        fdl2_addr: u64,
        default_entry: u64,
    ) -> Result<Option<u64>, String> {
        let mut pac_boot_entry: Option<u64> = None;
        for f in &pac.files {
            let id_lower = f.id.to_lowercase();
            if id_lower.contains("fdl1") {
                println!("[PAC] Staging FDL1 partition ({} bytes) at 0x{:08X}", f.data.len(), fdl1_addr);
                self.load_bytes(&f.data, fdl1_addr)?;
            } else if id_lower.contains("fdl2") {
                println!("[PAC] Staging FDL2 partition ({} bytes) at 0x{:08X}", f.data.len(), fdl2_addr);
                self.load_bytes(&f.data, fdl2_addr)?;
            } else if id_lower.contains("boot0") || id_lower.contains("spl") {
                println!("[PAC] Staging BOOT0/SPL partition ({} bytes) into SPI NAND Block 0", f.data.len());
                let chunk_len = f.data.len().min(2048);
                self.uc.get_data_mut().sfc.write_page(0, 0, &f.data[..chunk_len]);
                let _ = self.load_bytes(&f.data, FLASH_DIRECT_BASE);
                if let Ok(hdr) = SprdHeader::parse(&f.data) {
                    if f.data.len() > 32 {
                        let _ = self.load_bytes(&f.data[32..], hdr.load_addr as u64);
                    }
                    pac_boot_entry = Some(hdr.entry_point as u64);
                }
            } else if id_lower.contains("boot") || id_lower.contains("kernel") || id_lower.contains("firmware") {
                let load_addr = if f.load_addr != 0 { f.load_addr as u64 } else { default_entry };
                println!("[PAC] Staging Kernel/Boot partition '{}' ({} bytes) at 0x{:08X}", f.id, f.data.len(), load_addr);
                self.load_bytes(&f.data, load_addr)?;
                if pac_boot_entry.is_none() {
                    pac_boot_entry = Some(load_addr);
                }
            } else if id_lower.contains("system") || id_lower.contains("rootfs") {
                println!("[PAC] Staging System partition ({} bytes) into SPI NAND Flash", f.data.len());
                self.uc.get_data_mut().sfc.load_image(&f.data);
            } else if f.load_addr != 0 {
                // Secondary partitions with non-zero load address
                println!("[PAC] Staging partition '{}' ({} bytes) at 0x{:08X}", f.id, f.data.len(), f.load_addr);
                let _ = self.load_bytes(&f.data, f.load_addr as u64);
            }
        }
        Ok(pac_boot_entry)
    }

    pub fn setup_atags(&mut self, atags_addr: u64, board_id: u32, cmdline: Option<String>) -> Result<(), String> {
        let builder = AtagsBuilder::new().with_cmdline(cmdline);
        let tags_bytes = builder.build();
        self.load_bytes(&tags_bytes, atags_addr)?;

        // ARM kernel handoff registers: r0 = 0, r1 = board_id, r2 = atags_addr
        self.uc.reg_write(RegisterARM::R0, 0).map_err(|e| format!("{:?}", e))?;
        self.uc.reg_write(RegisterARM::R1, board_id as u64).map_err(|e| format!("{:?}", e))?;
        self.uc.reg_write(RegisterARM::R2, atags_addr).map_err(|e| format!("{:?}", e))?;

        println!(
            "[VM ATAGS] Staged ARM Boot Tags ({} bytes) at 0x{:08X} (r0=0, r1=0x{:X}, r2=0x{:08X})",
            tags_bytes.len(), atags_addr, board_id, atags_addr
        );
        Ok(())
    }

    pub fn step(&mut self, count: usize, timeout_us: u64) -> Result<bool, String> {
        let pc = self.uc.reg_read(RegisterARM::PC).unwrap_or(self.entry_point);
        match self.uc.emu_start(pc, 0xFFFF_FFFF, timeout_us, count) {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("TIMEOUT") || err_str.contains("OK") {
                    Ok(true)
                } else {
                    Err(format!("Emulation error at PC 0x{:08X}: {:?}", pc, e))
                }
            }
        }
    }

    pub fn inject_key(&mut self, key: &str, pressed: bool) -> bool {
        let ctx = self.uc.get_data_mut();
        let changed = ctx.kpd.inject_key(key, pressed);
        if changed && pressed && (ctx.kpd.int_en & 1) != 0 {
            ctx.intc.assert_irq(SPRD_IRQ_KPD);
        }
        changed
    }

    pub fn press_key(&mut self, key: &str) -> bool {
        self.inject_key(key, true)
    }

    pub fn release_key(&mut self, key: &str) -> bool {
        self.inject_key(key, false)
    }

    pub fn set_flip_state(&mut self, is_open: bool) {
        self.uc.get_data_mut().gpio.set_flip_state(is_open);
    }

    pub fn is_flip_open(&self) -> bool {
        self.uc.get_data().gpio.is_flip_open()
    }

    pub fn get_gc9307_framebuffer(&self) -> &[[u16; GC9307_WIDTH]; GC9307_HEIGHT] {
        &self.uc.get_data().display.gc9307.framebuffer
    }

    pub fn get_st7735_framebuffer(&self) -> &[[u16; ST7735_WIDTH]; ST7735_HEIGHT] {
        &self.uc.get_data().display.st7735.framebuffer
    }

    pub fn get_pc(&self) -> u32 {
        self.uc.reg_read(RegisterARM::PC).unwrap_or(0) as u32
    }

    pub fn get_sp(&self) -> u32 {
        self.uc.reg_read(RegisterARM::SP).unwrap_or(0) as u32
    }

    pub fn get_lr(&self) -> u32 {
        self.uc.reg_read(RegisterARM::LR).unwrap_or(0) as u32
    }

    pub fn get_cpsr(&self) -> u32 {
        self.uc.reg_read(RegisterARM::CPSR).unwrap_or(0) as u32
    }

    pub fn get_instruction_count(&self) -> u64 {
        self.uc.get_data().instruction_count
    }

    pub fn get_uart_tx_buffer(&self) -> Vec<u8> {
        self.uc.get_data().uart.tx_buffer.clone()
    }

    pub fn export_ppm(&self, path: &str) -> Result<(), String> {
        let mut f = File::create(path).map_err(|e| format!("Failed to create PPM: {}", e))?;
        writeln!(f, "P6\n{} {}\n255", GC9307_WIDTH, GC9307_HEIGHT)
            .map_err(|e| format!("Failed to write PPM header: {}", e))?;

        let fb = &self.uc.get_data().display.gc9307.framebuffer;
        let mut pixels = Vec::with_capacity(GC9307_WIDTH * GC9307_HEIGHT * 3);
        for row in fb.iter() {
            for &pixel in row.iter() {
                let r = (((pixel >> 11) & 0x1F) * 255 / 31) as u8;
                let g = (((pixel >> 5) & 0x3F) * 255 / 63) as u8;
                let b = ((pixel & 0x1F) * 255 / 31) as u8;
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
            }
        }
        f.write_all(&pixels).map_err(|e| format!("Failed to write PPM pixels: {}", e))?;
        Ok(())
    }

    pub fn export_sub_ppm(&self, path: &str) -> Result<(), String> {
        let mut f = File::create(path).map_err(|e| format!("Failed to create PPM: {}", e))?;
        writeln!(f, "P6\n{} {}\n255", ST7735_WIDTH, ST7735_HEIGHT)
            .map_err(|e| format!("Failed to write PPM header: {}", e))?;

        let fb = &self.uc.get_data().display.st7735.framebuffer;
        let mut pixels = Vec::with_capacity(ST7735_WIDTH * ST7735_HEIGHT * 3);
        for row in fb.iter() {
            for &pixel in row.iter() {
                let r = (((pixel >> 11) & 0x1F) * 255 / 31) as u8;
                let g = (((pixel >> 5) & 0x3F) * 255 / 63) as u8;
                let b = ((pixel & 0x1F) * 255 / 31) as u8;
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
            }
        }
        f.write_all(&pixels).map_err(|e| format!("Failed to write PPM pixels: {}", e))?;
        Ok(())
    }
}
