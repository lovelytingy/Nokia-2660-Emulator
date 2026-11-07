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
    pub usb: SprdUsb,
    pub logged_traps: HashSet<u32>,
    pub instruction_count: u64,
    pub trace: bool,
    pub timer_start_insn: u64,
    pub current_stage: Option<BootStage>,
    pub dial_buffer: String,
    pub in_call: bool,
    pub kernel_insn_count: u64,
    pub pending_unmapped_page: Option<u64>,
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
            usb: SprdUsb::new(print_stdout),
            logged_traps: HashSet::new(),
            instruction_count: 0,
            trace,
            timer_start_insn: 0,
            current_stage: None,
            dial_buffer: String::new(),
            in_call: false,
            kernel_insn_count: 0,
            pending_unmapped_page: None,
        };

        let mut uc = Unicorn::new_with_data(Arch::ARM, Mode::ARM, ctx)
            .map_err(|e| format!("Failed to create ARM Unicorn instance: {:?}", e))?;

        // 1. Map Internal SRAM (64 KB at 0x0000_0000)
        uc.mem_map(SRAM_BASE, SRAM_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map SRAM at 0x{:X}: {:?}", SRAM_BASE, e))?;

        // FDL1 Ack stub at 0x00006C22 (Thumb "bx lr" = 0x4770) for bare-metal payload clean return
        let _ = uc.mem_write(0x0000_6C22, &[0x70, 0x47]);

        // 2. Map System High DRAM (48 MB at 0x8000_0000)
        uc.mem_map(DRAM_BASE, DRAM_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map DRAM at 0x{:X}: {:?}", DRAM_BASE, e))?;

        // 2b. Map System Extended DRAM (16 MB at 0x8300_0000 to bring DRAM to full 64 MB)
        uc.mem_map(DRAM_EXT_BASE, DRAM_EXT_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map Extended DRAM at 0x{:X}: {:?}", DRAM_EXT_BASE, e))?;

        // 2c. Map Userland Application DRAM (32 MB at 0x8800_0000)
        uc.mem_map(USER_DRAM_BASE, USER_DRAM_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map User DRAM at 0x{:X}: {:?}", USER_DRAM_BASE, e))?;

        // 2d. Map ThreadX Task Stack & Dynamic Heap Window (32 MB at 0x7E00_0000)
        uc.mem_map(SYS_HEAP_BASE, SYS_HEAP_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map SYS HEAP at 0x{:X}: {:?}", SYS_HEAP_BASE, e))?;

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

        // 5. Map SPRD AHB System Space (32 MB at 0x2000_0000: AHB, LCDC, HW SFC and System Registers)
        uc.mem_map(SPRD_AHB_SYS_BASE, SPRD_AHB_SYS_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map AHB SYS at 0x{:X}: {:?}", SPRD_AHB_SYS_BASE, e))?;

        // 6. Map SPRD DMC / DDR PHY (2 MB at 0x3000_0000)
        uc.mem_map(SPRD_DMC_BASE, SPRD_DMC_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map DMC at 0x{:X}: {:?}", SPRD_DMC_BASE, e))?;

        // 7. Map Unisoc T107 AON / APB Peripheral Space (8 MB at 0x4000_0000)
        uc.mem_map(TRAP_BASE, TRAP_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map AON / APB at 0x{:X}: {:?}", TRAP_BASE, e))?;

        // 8. Map SPRD APB Peripheral Space (32 MB at 0x7000_0000: Debug UART, Clock Gates, APB)
        uc.mem_map(SPRD_APB_PERIPH_BASE, SPRD_APB_PERIPH_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map APB Peripherals at 0x{:X}: {:?}", SPRD_APB_PERIPH_BASE, e))?;

        // 9. Map SPRD ADI Controller (8 KB at 0xA160_0000)
        uc.mem_map(SPRD_ADI_CTRL_BASE, SPRD_ADI_CTRL_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map ADI Controller at 0x{:X}: {:?}", SPRD_ADI_CTRL_BASE, e))?;

        // 10. Map SPRD Analog Die Window (8 KB at 0xA161_0000)
        uc.mem_map(SPRD_ADI_ANALOG_BASE, SPRD_ADI_ANALOG_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map ADI Analog Window at 0x{:X}: {:?}", SPRD_ADI_ANALOG_BASE, e))?;

        // 11. Map ARM High Exception Vectors (64 KB at 0xFFFF_0000)
        uc.mem_map(HIGH_VECTORS_BASE, HIGH_VECTORS_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map High Vectors at 0x{:X}: {:?}", HIGH_VECTORS_BASE, e))?;

        // Initialize PMIC EIC Power Key register in memory (0x40608280) to 2 (bit 1 = 1: Power key released)
        let _ = uc.mem_write(0x4060_8280, &2u32.to_le_bytes());

        // Setup Instruction Hook & IRQ Dispatcher
        uc.add_code_hook(0, 0xFFFF_FFFF, |uc, addr, _size| {
            let trace_on = {
                let ctx = uc.get_data_mut();
                ctx.instruction_count += 1;
                ctx.trace
            };

            // Multi-stage boot detection and LCD output progression (active during PAC boot)
            let stage_update = {
                let ctx = uc.get_data_mut();
                if ctx.current_stage.is_some() {
                    let is_open = ctx.gpio.is_flip_open();
                    let new_stage = if addr == 0x0000_6400 {
                        Some(BootStage::Boot0)
                    } else if (addr >= 0x8010_0000 && addr < 0x8020_0000) && ctx.current_stage == Some(BootStage::Boot0) {
                        Some(BootStage::Boot1)
                    } else if ((addr >= 0x80a0_0000 && addr < 0x8300_0000) || (addr >= 0x8000_0000 && addr < 0x8010_0000)) && ctx.current_stage == Some(BootStage::Boot1) {
                        Some(BootStage::Kernel)
                    } else if addr >= 0x8800_0000 && ctx.current_stage == Some(BootStage::Kernel) {
                        Some(BootStage::User)
                    } else {
                        if ctx.current_stage == Some(BootStage::Kernel) {
                            ctx.kernel_insn_count += 1;
                        }
                        None
                    };

                    if let Some(stg) = new_stage {
                        if ctx.current_stage != Some(stg) {
                            ctx.current_stage = Some(stg);
                            Some((stg, is_open))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((stg, is_open)) = stage_update {
                let ctx = uc.get_data_mut();
                let pct = stg.progress_pct();
                println!("[BOOT PROGRESS] Boot stage advanced: {:?} ({}%) at PC 0x{:08X}", stg, pct, addr);
                if !ctx.display.gc9307.has_custom_framebuffer {
                    match stg {
                        BootStage::Boot0 => {
                            ctx.display.gc9307.render_boot_stage(BootStage::Boot0, "LPDDR2 & IRAM 0x00006200 Ready", pct);
                            ctx.display.st7735.render_cover_screen(Some(BootStage::Boot0), is_open);
                        }
                        BootStage::Boot1 => {
                            ctx.display.gc9307.render_boot_stage(BootStage::Boot1, "SCI FTL Mounted -> Verifying Kernel...", pct);
                            ctx.display.st7735.render_cover_screen(Some(BootStage::Boot1), is_open);
                        }
                        BootStage::Kernel => {
                            ctx.display.gc9307.render_boot_stage(BootStage::Kernel, "ThreadX Preemptive Scheduler Active", pct);
                            ctx.display.st7735.render_cover_screen(Some(BootStage::Kernel), is_open);
                        }
                        BootStage::User => {
                            ctx.display.gc9307.render_boot_stage(BootStage::User, "Userland MS_MMI_Main Active", pct);
                            ctx.display.st7735.render_cover_screen(Some(BootStage::User), is_open);
                        }
                    }
                }
            } else {
                let ctx = uc.get_data_mut();
                if ctx.current_stage == Some(BootStage::Kernel) && ctx.kernel_insn_count > 0 && ctx.kernel_insn_count % 250_000 == 0 && !ctx.display.gc9307.has_custom_framebuffer {
                    let insn_k = ctx.kernel_insn_count / 1_000;
                    let status = format!("* Exec: {}k insns | PC: 0x{:08X}", insn_k, addr);
                    ctx.display.gc9307.render_boot_stage(BootStage::Kernel, &status, 75);
                }
            }

            // BOOT1 handoff & FTL interception:
            // 0x801136E0: SCI_FTL_Load(dev_id)
            // 0x80100C24: load_kernel()
            // 0x80113CA4: SCI_FTL_Unload(dev_id)
            if addr == 0x8011_36E0 {
                let _ = uc.mem_write(0x8057_1AF0, &2u32.to_le_bytes()); // device state = 2 (LOADED)
                let _ = uc.mem_write(0x8057_1B00, &2048u32.to_le_bytes()); // sector size
                let _ = uc.mem_write(0x8057_1B04, &64u32.to_le_bytes()); // erase unit
                let _ = uc.mem_write(0x8057_1B08, &1024u32.to_le_bytes()); // total sectors
                let _ = uc.mem_write(0x8057_1B0C, &1u32.to_le_bytes()); // attrib
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::R0, 0); // r0 = 0 (success)
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            } else if addr == 0x8010_0C24 {
                println!("[BOOT1] load_kernel(): KernelImg staged at 0x80a05e00 -> handoff ready");
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::R0, 1); // r0 = 1 (success)
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            } else if addr == 0x8011_3CA4 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::R0, 0); // r0 = 0 (success)
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if trace_on {
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let sp = uc.reg_read(RegisterARM::SP).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                println!("[TRACE] PC = 0x{:08X} | CPSR: 0x{:08X} | SP: 0x{:08X} | R0: 0x{:08X} | LR: 0x{:08X}", addr, cpsr, sp, r0, lr);
            }

            // Check for pending unmasked IRQ to dispatch to CPU exception vector
            let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
            if (cpsr & 0x80) == 0 {
                let pending = uc.get_data().intc.get_irq_masked_status();
                if pending != 0 {
                    let vector_base = if addr < DRAM_BASE { 0x0000_0000u64 } else { 0x8000_0000u64 };
                    println!("[IRQ DISPATCH] Pending IRQs: 0x{:08X} at PC: 0x{:08X} (vector: 0x{:08X})", pending, addr, vector_base + 0x18);
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

        // Setup SPRD DMC / DDR PHY Read Hook (0x3000_0000..0x3001_0000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_DMC_BASE,
            SPRD_DMC_BASE + SPRD_DMC_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_DMC_BASE) as u32;
                let mut cur = [0u8; 4];
                let _ = uc.mem_read(addr, &mut cur);
                let mut val = u32::from_le_bytes(cur);

                match offset {
                    0x0008 => {
                        let tap = if (val & (1 << 15)) != 0 {
                            (val >> 1) & 0x1F
                        } else {
                            val & 0x1F
                        };
                        let bit17 = if tap >= 16 { 1 << 17 } else { 0 };
                        val = (val & !(1 << 17)) | bit17;
                    }
                    0x0304 | 0x0404 | 0x0504 => {
                        val |= 0x1000_0000;
                    }
                    0x0104 => {
                        val &= !0x07F0_0000;
                    }
                    0x0180 => {
                        // DDR BIST status: bit 2 = DONE (1), bit 3 = PASS (1)
                        val |= 0x0000_000C;
                    }
                    _ => {}
                }

                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add DMC read hook: {:?}", e))?;

        // Setup SPRD Boot Debug UART Hooks (0x7010_0000..0x7010_1000)
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            SPRD_DEBUG_UART_BASE,
            SPRD_DEBUG_UART_BASE + SPRD_DEBUG_UART_SIZE as u64,
            |uc, _mem_type, addr, _size, value| {
                let offset = (addr - SPRD_DEBUG_UART_BASE) as u32;
                let val = (value as u64 & 0xFFFF_FFFF) as u32;
                if offset == 0 {
                    let ch = (val & 0xFF) as u8;
                    let ctx = uc.get_data_mut();
                    ctx.uart.tx_buffer.push(ch);
                    if ctx.uart.print_stdout {
                        print!("{}", ch as char);
                        let _ = std::io::stdout().flush();
                    }
                }
                true
            },
        ).map_err(|e| format!("Failed to add Debug UART write hook: {:?}", e))?;

        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_DEBUG_UART_BASE,
            SPRD_DEBUG_UART_BASE + SPRD_DEBUG_UART_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_DEBUG_UART_BASE) as u32;
                let val: u32 = match offset {
                    0x000C => 0x0000_0000, // STS0: TX FIFO empty
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add Debug UART read hook: {:?}", e))?;

        // Setup SPRD AON System Timer Read Hook (0x4023_0000..0x4023_1000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_SYSTIMER_BASE,
            SPRD_SYSTIMER_BASE + SPRD_SYSTIMER_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_SYSTIMER_BASE) as u32;
                let val: u32 = match offset {
                    0x0000 | 0x0004 | 0x000C => {
                        // Monotonically advancing counter (1 tick per 16 instructions)
                        (uc.get_data().instruction_count >> 4) as u32
                    }
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add SYSTIMER read hook: {:?}", e))?;

        // Setup SPRD Hardware SFC Hooks (0x21A0_0000..0x21A1_0000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_HW_SFC_BASE,
            SPRD_HW_SFC_BASE + SPRD_HW_SFC_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_HW_SFC_BASE) as u32;
                let mut cur = [0u8; 4];
                let _ = uc.mem_read(addr, &mut cur);
                let mut val = u32::from_le_bytes(cur);

                match offset {
                    0x0010 => {
                        // SFC status: bit 24 = CMD_DONE (1), bit 27 = ERROR (0)
                        val = 1 << 24;
                    }
                    0x019C => {
                        // Winbond W25N01GV NAND Flash ID (NAND #58: 0xBAEF)
                        val = 0x0000_BAEF;
                    }
                    _ => {}
                }

                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add HW SFC read hook: {:?}", e))?;

        // Setup SPRD LCD Controller Hooks (0x2080_0000..0x2080_2000)
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            SPRD_LCDC_BASE,
            SPRD_LCDC_BASE + SPRD_LCDC_SIZE as u64,
            |uc, _mem_type, addr, _size, value| {
                let offset = (addr - SPRD_LCDC_BASE) as u32;
                let val = (value as u64 & 0xFFFF_FFFF) as u32;
                if offset == 0x0000 {
                    // Control register: bit 3 = Start Refresh
                    if (val & 0x08) != 0 {
                        let mut y_base_bytes = [0u8; 4];
                        if uc.mem_read(SPRD_LCDC_BASE + 0x24, &mut y_base_bytes).is_ok() {
                            let y_base = u32::from_le_bytes(y_base_bytes);
                            let raw_y = y_base as u64;
                            let fb_addr = if raw_y >= 0x7000_0000 && raw_y < 0x9000_0000 {
                                raw_y
                            } else if (raw_y << 2) >= 0x7000_0000 && (raw_y << 2) < 0x9000_0000 {
                                raw_y << 2
                            } else {
                                raw_y
                            };
                            let mut size_xy_bytes = [0u8; 4];
                            let (width, height) = if uc.mem_read(SPRD_LCDC_BASE + 0x2C, &mut size_xy_bytes).is_ok() {
                                let sxy = u32::from_le_bytes(size_xy_bytes);
                                let w = (sxy & 0xFFFF) as usize;
                                let h = ((sxy >> 16) & 0xFFFF) as usize;
                                if w > 0 && h > 0 && w <= GC9307_WIDTH && h <= GC9307_HEIGHT {
                                    (w, h)
                                } else {
                                    (GC9307_WIDTH, GC9307_HEIGHT)
                                }
                            } else {
                                (GC9307_WIDTH, GC9307_HEIGHT)
                            };
                            let byte_len = width * height * 2;
                            let mut fb_data = vec![0u8; byte_len];
                            if uc.mem_read(fb_addr, &mut fb_data).is_ok() {
                                let ctx = uc.get_data_mut();
                                for y in 0..height {
                                    for x in 0..width {
                                        let idx = (y * width + x) * 2;
                                        let pixel = u16::from_le_bytes([fb_data[idx], fb_data[idx + 1]]);
                                        ctx.display.gc9307.framebuffer[y][x] = pixel;
                                    }
                                }
                                ctx.display.gc9307.has_custom_framebuffer = true;
                            }
                        }
                        // Set Refresh Done status (bit 0 = 1) at 0x2080_011C
                        let _ = uc.mem_write(SPRD_LCDC_BASE + 0x011C, &1u32.to_le_bytes());
                    }
                } else if offset == 0x0114 {
                    // IRQ clear: clear status bit 0
                    let _ = uc.mem_write(SPRD_LCDC_BASE + 0x011C, &0u32.to_le_bytes());
                }
                true
            },
        ).map_err(|e| format!("Failed to add LCDC write hook: {:?}", e))?;

        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_LCDC_BASE,
            SPRD_LCDC_BASE + SPRD_LCDC_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_LCDC_BASE) as u32;
                let val: u32 = match offset {
                    0x011C => 1, // Refresh Done
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add LCDC read hook: {:?}", e))?;

        // Setup SPRD AON Pinmux & Boot Key Read Hook (0x402A_0000..0x402A_2000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_AON_PINMUX_BASE,
            SPRD_AON_PINMUX_BASE + SPRD_AON_PINMUX_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_AON_PINMUX_BASE) as u32;
                let val: u32 = match offset {
                    0x0018 => 0, // Normal boot mode (no hardware test strap)
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add Pinmux read hook: {:?}", e))?;

        // Setup SPRD Secure Boot & Chip ID Read Hook (0x402E_0000..0x402E_4000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_SECURE_BOOT_BASE,
            SPRD_SECURE_BOOT_BASE + SPRD_SECURE_BOOT_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_SECURE_BOOT_BASE) as u32;
                let val: u32 = match offset {
                    0x00FC => 0x9117_0000, // UMS9117 Chip ID
                    0x0568 => 0x0000_0003, // Security Gates 1 & 2 active
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add Secure Boot read hook: {:?}", e))?;

        // Setup SPRD ADI Controller Hooks (0xA160_0000..0xA160_2000)
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            SPRD_ADI_CTRL_BASE,
            SPRD_ADI_CTRL_BASE + SPRD_ADI_CTRL_SIZE as u64,
            |uc, _mem_type, addr, _size, value| {
                let offset = (addr - SPRD_ADI_CTRL_BASE) as u32;
                let val = (value as u64 & 0xFFFF_FFFF) as u32;
                if offset == 0x0238 {
                    // ADI Read Command: prepare matching response at 0xA160023C
                    let tag = (val & 0x1FFFF) >> 2;
                    let resp = tag << 16; // bit 31=0 (not busy), mock_val=0
                    let _ = uc.mem_write(SPRD_ADI_CTRL_BASE + 0x023C, &resp.to_le_bytes());
                }
                true
            },
        ).map_err(|e| format!("Failed to add ADI write hook: {:?}", e))?;

        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_ADI_CTRL_BASE,
            SPRD_ADI_CTRL_BASE + SPRD_ADI_CTRL_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_ADI_CTRL_BASE) as u32;
                let val: u32 = match offset {
                    0x0240 => 0, // Write FIFO ready
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add ADI read hook: {:?}", e))?;

        // Setup SPRD PMIC EIC & Power Key Read Hook (0x4060_8000..0x4060_C000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_PMIC_EIC_BASE,
            SPRD_PMIC_EIC_BASE + SPRD_PMIC_EIC_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_PMIC_EIC_BASE) as u32;
                let val: u32 = match offset {
                    0x0280 => {
                        let mut eic_val = 0x0002u32; // bit 1 = 1 (Power idle, active low), bit 9 = 0 (Key 8 idle, active high)
                        if uc.get_data().kpd.power_button_down {
                            eic_val &= !(1 << 1);
                        }
                        if uc.get_data().kpd.key_status == 0x22 && uc.get_data().kpd.last_key_down {
                            eic_val |= 1 << 9;
                        }
                        eic_val
                    }
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add PMIC EIC read hook: {:?}", e))?;

        // Setup SPRD PMIC ADI Write Hook (Torch, Haptics & ADI Register Access) (0x4060_0000..0x4060_2000)
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            SPRD_PMIC_ADI_BASE,
            SPRD_PMIC_ADI_BASE + SPRD_PMIC_ADI_SIZE as u64,
            |uc, _mem_type, addr, _size, value| {
                let offset = (addr - SPRD_PMIC_ADI_BASE) as u32;
                if offset == 0x0028 {
                    // PMIC ADI Base Command Register: Payload reads target PMIC register -> response at 0x4060002C
                    let target_addr = value as u64 & 0xFFFF_FFFF;
                    let resp = if (target_addr & 0xFFFF) == 0x8280 || (target_addr & 0xFFF) == 0x0280 {
                        let mut eic_val = 0x0002u32; // bit 1 = 1 (Power idle, active low), bit 9 = 0 (Key 8 idle, active high)
                        if uc.get_data().kpd.power_button_down {
                            eic_val &= !(1 << 1);
                        }
                        if uc.get_data().kpd.key_status == 0x22 && uc.get_data().kpd.last_key_down {
                            eic_val |= 1 << 9;
                        }
                        eic_val
                    } else {
                        let mut resp_bytes = [0u8; 4];
                        let _ = uc.mem_read(target_addr, &mut resp_bytes);
                        u32::from_le_bytes(resp_bytes)
                    };
                    let _ = uc.mem_write(SPRD_PMIC_ADI_BASE + 0x002C, &resp.to_le_bytes());
                }
                true
            },
        ).map_err(|e| format!("Failed to add PMIC ADI write hook: {:?}", e))?;

        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_PMIC_ADI_BASE,
            SPRD_PMIC_ADI_BASE + SPRD_PMIC_ADI_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_PMIC_ADI_BASE) as u32;
                if offset == 0x002C {
                    return true;
                }
                let val: u32 = match offset {
                    0x0030 => 0, // Write FIFO ready
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add PMIC ADI read hook: {:?}", e))?;

        // Setup Unisoc T107 USB OTG Controller Hooks (0x2020_0000..0x2020_2000)
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            SPRD_USB_BASE,
            SPRD_USB_BASE + SPRD_USB_SIZE as u64,
            |uc, _mem_type, addr, _size, value| {
                let offset = (addr - SPRD_USB_BASE) as u32;
                let val = (value as u64 & 0xFFFF_FFFF) as u32;
                uc.get_data_mut().usb.write_reg(addr as u32, val);

                if offset == 0x1c84 && (val & 1) != 0 {
                    // EP5 TX DMA Trigger: guest requested packet transmission
                    let tx_tr_ptr = uc.get_data().usb.tx_tr_ptr as u64;
                    if tx_tr_ptr != 0 {
                        let mut desc = [0u8; 8];
                        if uc.mem_read(tx_tr_ptr, &mut desc).is_ok() {
                            let src_addr = u32::from_le_bytes([desc[0], desc[1], desc[2], desc[3]]) as u64;
                            let len = u16::from_le_bytes([desc[6], desc[7]]) as usize;
                            if len > 0 && src_addr != 0 {
                                let mut tx_buf = vec![0u8; len];
                                if uc.mem_read(src_addr, &mut tx_buf).is_ok() {
                                    // Set TX complete flag (bit 18)
                                    uc.get_data_mut().usb.tx_status |= 1 << 18;
                                    let tx_st = uc.get_data().usb.tx_status;
                                    let _ = uc.mem_write(SPRD_USB_BASE + 0x1c88, &tx_st.to_le_bytes());

                                    // Process TX packet through host libc server engine
                                    let rx_opt = uc.get_data_mut().usb.ingest_tx_data(&tx_buf);
                                    if let Some(rx_data) = rx_opt {
                                        // Deliver libc server response to guest EP6 RX DMA buffer
                                        let rx_tr_ptr = uc.get_data().usb.rx_tr_ptr as u64;
                                        if rx_tr_ptr != 0 {
                                            let mut rx_desc = [0u8; 4];
                                            if uc.mem_read(rx_tr_ptr, &mut rx_desc).is_ok() {
                                                let dst_addr = u32::from_le_bytes(rx_desc) as u64;
                                                if dst_addr != 0 {
                                                    let _ = uc.mem_write(dst_addr, &rx_data);
                                                    let rem = ((12800usize.saturating_sub(rx_data.len())) as u32) << 16;
                                                    uc.get_data_mut().usb.rx_rem_len = rem;
                                                    let _ = uc.mem_write(SPRD_USB_BASE + 0x1e90, &rem.to_le_bytes());

                                                    uc.get_data_mut().usb.rx_status |= 1 << 18;
                                                    let rx_st = uc.get_data().usb.rx_status;
                                                    let _ = uc.mem_write(SPRD_USB_BASE + 0x1e88, &rx_st.to_le_bytes());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if offset == 0x1e84 && (val & 1) != 0 {
                    // Guest re-armed RX DMA
                    if uc.get_data().usb.state == UsbState::Connected && !uc.get_data().usb.handshake_drained {
                        // Empty 0-byte transfer completion to cleanly finish post-connect drain loop
                        let rem = 12800u32 << 16;
                        uc.get_data_mut().usb.rx_rem_len = rem;
                        let _ = uc.mem_write(SPRD_USB_BASE + 0x1e90, &rem.to_le_bytes());

                        uc.get_data_mut().usb.rx_status |= 1 << 18;
                        let rx_st = uc.get_data().usb.rx_status;
                        let _ = uc.mem_write(SPRD_USB_BASE + 0x1e88, &rx_st.to_le_bytes());
                    }
                } else if offset == 0x1c88 {
                    let tx_st = uc.get_data().usb.tx_status;
                    let _ = uc.mem_write(SPRD_USB_BASE + 0x1c88, &tx_st.to_le_bytes());
                } else if offset == 0x1e88 {
                    let rx_st = uc.get_data().usb.rx_status;
                    let _ = uc.mem_write(SPRD_USB_BASE + 0x1e88, &rx_st.to_le_bytes());
                }
                true
            },
        ).map_err(|e| format!("Failed to add USB write hook: {:?}", e))?;

        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_USB_BASE,
            SPRD_USB_BASE + SPRD_USB_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let val = uc.get_data().usb.read_reg(addr as u32);
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add USB read hook: {:?}", e))?;

        // Setup SPRD Keypad Matrix Hooks (0x4025_0000..0x4025_1000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_KEYPAD_BASE,
            SPRD_KEYPAD_BASE + SPRD_KEYPAD_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_KEYPAD_BASE) as u32;
                let val = uc.get_data().kpd.read(offset);
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add Keypad read hook: {:?}", e))?;

        uc.add_mem_hook(
            HookType::MEM_WRITE,
            SPRD_KEYPAD_BASE,
            SPRD_KEYPAD_BASE + SPRD_KEYPAD_SIZE as u64,
            |uc, _mem_type, addr, _size, value| {
                let offset = (addr - SPRD_KEYPAD_BASE) as u32;
                let val = (value as u64 & 0xFFFF_FFFF) as u32;
                uc.get_data_mut().kpd.write(offset, val);
                true
            },
        ).map_err(|e| format!("Failed to add Keypad write hook: {:?}", e))?;

        // Setup SPRD SPI1 Controller Hooks for Display (0x70B0_0000..0x70B0_1000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_APB_SPI1_BASE,
            SPRD_APB_SPI1_BASE + SPRD_APB_SPI1_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_APB_SPI1_BASE) as u32;
                let val: u32 = match offset {
                    0x0000 => {
                        // SPI TXD / RX FIFO: Return GC9307 ID byte stream (0x00, 0x93, 0x07)
                        let idx = uc.get_data().display.gc9307.id_read_idx;
                        let id_bytes = [0x00u32, 0x93u32, 0x07u32];
                        let ret = id_bytes.get(idx % 3).copied().unwrap_or(0);
                        uc.get_data_mut().display.gc9307.id_read_idx += 1;
                        ret
                    }
                    0x0028 => 0x0000_0100, // int_raw: bit 8 = 1 (TX_END_IRQ)
                    0x0034 => 0x0000_0080, // sts2: bit 7 = 1 (TXF_REAL_EMPTY), bit 8 = 0 (not busy), bit 5 = 0 (RX FIFO not empty)
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add SPI1 read hook: {:?}", e))?;

        uc.add_mem_hook(
            HookType::MEM_WRITE,
            SPRD_APB_SPI1_BASE,
            SPRD_APB_SPI1_BASE + SPRD_APB_SPI1_SIZE as u64,
            |uc, _mem_type, addr, _size, value| {
                let offset = (addr - SPRD_APB_SPI1_BASE) as u32;
                let val = (value as u64 & 0xFFFF_FFFF) as u32;
                if offset == 0x0000 {
                    let is_data = uc.get_data().display.gc9307.dc_mode;
                    let byte_val = (val & 0xFF) as u8;
                    let ctx = uc.get_data_mut();
                    if !is_data && (byte_val == 0x2A || byte_val == 0x2B || byte_val == 0x2C || byte_val == 0x00) {
                        ctx.display.gc9307.write_cmd(byte_val);
                    } else {
                        ctx.display.gc9307.write_data(byte_val);
                    }
                } else if offset == 0x0044 {
                    let is_data = (val & (1 << 15)) != 0;
                    uc.get_data_mut().display.gc9307.dc_mode = is_data;
                }
                true
            },
        ).map_err(|e| format!("Failed to add SPI1 write hook: {:?}", e))?;

        // Setup SPRD LCM Controller Hooks (0x20A0_0000..0x20A0_1000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_LCM_BASE,
            SPRD_LCM_BASE + SPRD_LCM_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_LCM_BASE) as u32;
                let val: u32 = match offset {
                    0x0000 => 0, // LCM_CR(0): bit 1 = 0 (LCM not busy)
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add LCM read hook: {:?}", e))?;

        // Setup LCM Data Read Hook at 0x6002_0000 (GC9307 LCD ID access fallback)
        uc.add_mem_hook(
            HookType::MEM_READ,
            0x6002_0000,
            0x6002_1000,
            |uc, _mem_type, addr, size, _value| {
                let idx = uc.get_data().display.gc9307.id_read_idx;
                let id_bytes = [0x00u32, 0x93u32, 0x07u32];
                let ret = id_bytes.get(idx % 3).copied().unwrap_or(0);
                uc.get_data_mut().display.gc9307.id_read_idx += 1;
                let bytes = ret.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add LCM Data read hook: {:?}", e))?;

        // Setup LCM Data Write Hook at 0x6002_0000
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            0x6002_0000,
            0x6002_1000,
            |uc, _mem_type, addr, _size, value| {
                let offset = (addr - 0x6002_0000) as u32;
                let val = (value as u64 & 0xFFFF_FFFF) as u32;
                let ctx = uc.get_data_mut();
                if offset == 0x0000 {
                    ctx.display.gc9307.write_cmd((val & 0xFF) as u8);
                } else {
                    ctx.display.gc9307.write_data((val & 0xFF) as u8);
                    if (val >> 8) != 0 {
                        ctx.display.gc9307.write_data(((val >> 8) & 0xFF) as u8);
                    }
                }
                true
            },
        ).map_err(|e| format!("Failed to add LCM Data write hook: {:?}", e))?;

        // Setup Unmapped Trap Read/Write Hook
        uc.add_mem_hook(
            HookType::MEM_READ_UNMAPPED | HookType::MEM_WRITE_UNMAPPED,
            0,
            0xFFFF_FFFF,
            |uc, mem_type, addr, size, value| {
                let pc = uc.reg_read(RegisterARM::PC).unwrap_or(0);
                let page = addr & !0xFFF;
                let is_new = uc.get_data_mut().logged_traps.insert(addr as u32);
                if mem_type == MemType::READ_UNMAPPED {
                    if is_new {
                        println!("\n[N2660Emu MMIO TRAP] Unmapped READ at 0x{:08X} (size {}) from PC 0x{:08X}", addr, size, pc);
                        println!("[INFO] Unmapped MMIO read returned: 0x00000000\n");
                    }
                } else {
                    if is_new {
                        println!("\n[N2660Emu MMIO TRAP] Unmapped WRITE at 0x{:08X} (val 0x{:08X}, size {}) from PC 0x{:08X}\n", addr, value as u32, size, pc);
                    }
                }
                // Record page to map safely outside JIT execution (prevents Apple Silicon assertion failure)
                uc.get_data_mut().pending_unmapped_page = Some(page);
                false
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
        
        let is_thumb = (entry_point & 1) != 0;
        let mut cpsr = 0x6000_00D3; // ARM Supervisor (SVC) mode, IRQ/FIQ masked
        if is_thumb {
            cpsr |= 0x20;
        }
        self.uc.reg_write(RegisterARM::CPSR, cpsr)
            .map_err(|e| format!("Failed to write CPSR: {:?}", e))?;
        // Enable VFP / NEON via CP15 CPACR and FPEXC
        let _ = self.uc.reg_write(RegisterARM::C1_C0_2, 0x00F0_0000);
        let _ = self.uc.reg_write(RegisterARM::FPEXC, 0x4000_0000);

        // Safe halt loop stub at 0x8070_5FF0 ("b ." = 0xEAFFFFFE)
        let _ = self.uc.mem_write(0x8070_5FF0, &[0xFE, 0xFF, 0xFF, 0xEA]);
        let _ = self.uc.reg_write(RegisterARM::LR, 0x8070_5FF0);

        self.uc.reg_write(RegisterARM::PC, entry_point & !1)
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
        _fdl2_addr: u64,
        default_entry: u64,
    ) -> Result<Option<u64>, String> {
        let mut pac_boot_entry: Option<u64> = None;
        let mut kernel_entry: Option<u64> = None;

        for f in &pac.files {
            let id_lower = f.id.to_lowercase();
            let is_dhtb = f.data.len() > 512 && &f.data[0..4] == b"DHTB";

            if id_lower == "fdl" || id_lower.contains("fdl1") {
                let load_target = fdl1_addr;
                println!("[PAC] Staging FDL1 partition ({} bytes) at 0x{:08X}", f.data.len(), load_target);
                self.load_bytes(&f.data, load_target)?;
            } else if id_lower.contains("fdl2") {
                println!("[PAC] Staging FDL2 partition ({} bytes) at 0x80020000", f.data.len());
                let _ = self.load_bytes(&f.data, 0x8002_0000);
            } else if id_lower.contains("boot0") || id_lower.contains("spl") {
                println!("[PAC] Staging BOOT0/SPL partition ({} bytes) into SPI NAND Block 0 & SRAM", f.data.len());
                let chunk_len = f.data.len().min(2048);
                self.uc.get_data_mut().sfc.write_page(0, 0, &f.data[..chunk_len]);
                let _ = self.load_bytes(&f.data, FLASH_DIRECT_BASE);

                // Stage into SRAM
                let _ = self.load_bytes(&f.data, 0x0000_6200);

                if is_dhtb {
                    // Code begins at offset 512 (0x00006400)
                    pac_boot_entry = Some(0x0000_6400);
                } else if let Ok(hdr) = SprdHeader::parse(&f.data) {
                    if f.data.len() > 32 {
                        let _ = self.load_bytes(&f.data[32..], hdr.load_addr as u64);
                    }
                    pac_boot_entry = Some(hdr.entry_point as u64);
                } else {
                    pac_boot_entry = Some(0x0000_6200);
                }
            } else if id_lower.contains("boot1") || id_lower.contains("lk") {
                println!("[PAC] Staging BOOT1/LK partition ({} bytes) at 0x80100000", f.data.len());
                let _ = self.load_bytes(&f.data, 0x8010_0000);
            } else if id_lower.contains("kernel") || id_lower.contains("boot") || id_lower == "kernelimg" {
                let (load_addr, entry) = if f.load_addr == 0x8000_0000 {
                    (0x8000_0000u64, 0x8000_0000u64)
                } else {
                    (0x80a0_5e00u64, 0x80a0_6000u64)
                };
                println!("[PAC] Staging Kernel/Boot partition '{}' ({} bytes) at 0x{:08X}", f.id, f.data.len(), load_addr);
                self.load_bytes(&f.data, load_addr)?;
                kernel_entry = Some(entry);
            } else if id_lower.contains("userimg") || id_lower.contains("user_img") {
                let load_addr = 0x8800_0000u64;
                println!("[PAC] Staging UserImg partition '{}' ({} bytes) at 0x{:08X}", f.id, f.data.len(), load_addr);
                self.load_bytes(&f.data, load_addr)?;
            } else if id_lower.contains("system") || id_lower.contains("rootfs") || id_lower.contains("mmires") || id_lower.contains("nv") {
                // Flash storage partitions
                self.uc.get_data_mut().sfc.load_image(&f.data);
            } else if f.load_addr != 0 && f.load_addr < 0x8000_0000 && (f.load_addr & 0xFFFF_0000) != 0 {
                // Secondary partitions with explicit non-tag physical address
                println!("[PAC] Staging partition '{}' ({} bytes) at 0x{:08X}", f.id, f.data.len(), f.load_addr);
                let _ = self.load_bytes(&f.data, f.load_addr as u64);
            }
        }

        // Default boot entry priority: BOOT0 (cold boot SPL) > Kernel > default_entry
        self.set_boot_stage(BootStage::Boot0);
        Ok(pac_boot_entry.or(kernel_entry).or(Some(default_entry)))
    }

    pub fn set_boot_stage(&mut self, stage: BootStage) {
        let ctx = self.uc.get_data_mut();
        ctx.current_stage = Some(stage);
        let is_open = ctx.gpio.is_flip_open();
        let pct = stage.progress_pct();
        match stage {
            BootStage::Boot0 => {
                ctx.display.gc9307.render_boot_stage(BootStage::Boot0, "LPDDR2 & IRAM 0x00006200 Ready", pct);
            }
            BootStage::Boot1 => {
                ctx.display.gc9307.render_boot_stage(BootStage::Boot1, "SCI FTL Mounted -> Verifying Kernel...", pct);
            }
            BootStage::Kernel => {
                ctx.display.gc9307.render_boot_stage(BootStage::Kernel, "ThreadX Preemptive Scheduler Active", pct);
            }
            BootStage::User => {
                ctx.display.gc9307.render_boot_stage(BootStage::User, "Userland MS_MMI_Main Active", pct);
            }
        }
        ctx.display.st7735.render_cover_screen(Some(stage), is_open);
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
        let mut pc = self.uc.reg_read(RegisterARM::PC).unwrap_or(self.entry_point);
        let cpsr = self.uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
        if (cpsr & 0x20) != 0 {
            pc |= 1;
        }
        // Avoid spawning background timer thread when instruction count is specified,
        // preventing race conditions with Apple JIT write-protection (uc->nested assertion).
        let effective_timeout = if count > 0 { 0 } else { timeout_us };
        match self.uc.emu_start(pc, 0xFFFF_FFFF, effective_timeout, count) {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = format!("{:?}", e);
                if let Some(page) = self.uc.get_data_mut().pending_unmapped_page.take() {
                    let _ = self.uc.mem_map(page, 0x1000, Prot::ALL);
                    return Ok(true);
                }
                if err_str.contains("TIMEOUT") || err_str.contains("OK") {
                    Ok(true)
                } else {
                    let lr = self.uc.reg_read(RegisterARM::LR).unwrap_or(0);
                    let sp = self.uc.reg_read(RegisterARM::SP).unwrap_or(0);
                    let r0 = self.uc.reg_read(RegisterARM::R0).unwrap_or(0);
                    let r1 = self.uc.reg_read(RegisterARM::R1).unwrap_or(0);
                    let r2 = self.uc.reg_read(RegisterARM::R2).unwrap_or(0);
                    let r3 = self.uc.reg_read(RegisterARM::R3).unwrap_or(0);
                    let fault_pc = self.uc.reg_read(RegisterARM::PC).unwrap_or(pc);
                    Err(format!("Emulation error at fault PC 0x{:08X} (initial PC 0x{:08X}, LR: 0x{:08X}, SP: 0x{:08X}, R0: 0x{:08X}, R1: 0x{:08X}, R2: 0x{:08X}, R3: 0x{:08X}, CPSR: 0x{:08X}): {:?}", fault_pc & !1, pc & !1, lr, sp, r0, r1, r2, r3, cpsr, e))
                }
            }
        }
    }

    pub fn inject_key(&mut self, key: &str, pressed: bool) -> bool {
        let is_power = matches!(key.to_lowercase().as_str(), "end" | "power" | "clear" | "x");
        if is_power {
            let pb_val = if pressed { 0u32 } else { 2u32 };
            let _ = self.uc.mem_write(0x4060_8280, &pb_val.to_le_bytes());
        }

        let (changed, int_en, int_raw, key_status) = {
            let ctx = self.uc.get_data_mut();
            let changed = ctx.kpd.inject_key(key, pressed);
            let int_en = ctx.kpd.int_en;
            let int_raw = ctx.kpd.int_raw;
            let key_status = ctx.kpd.key_status;

            if changed && pressed {
                if key == "call" {
                    ctx.in_call = true;
                } else if key == "end" {
                    ctx.in_call = false;
                    ctx.dial_buffer.clear();
                } else if ("0"..="9").contains(&key) || key == "*" || key == "#" {
                    if ctx.dial_buffer.len() < 14 {
                        ctx.dial_buffer.push_str(key);
                    }
                }
                if ctx.current_stage.is_none() {
                    ctx.display.gc9307.render_mmi_home(&ctx.dial_buffer, ctx.in_call, 0);
                }
            }
            (changed, int_en, int_raw, key_status)
        };

        let _ = self.uc.mem_write(SPRD_KEYPAD_BASE + 0x08, &int_raw.to_le_bytes());
        let _ = self.uc.mem_write(SPRD_KEYPAD_BASE + 0x2C, &key_status.to_le_bytes());

        if changed && pressed && (int_en & 1) != 0 {
            self.uc.get_data_mut().intc.assert_irq(SPRD_IRQ_KPD);
        }

        changed
    }

    pub fn get_usb_stdout(&self) -> &[u8] {
        &self.uc.get_data().usb.stdout_log
    }

    pub fn set_usb_args(&mut self, args: Vec<String>) {
        self.uc.get_data_mut().usb.args = args;
    }

    pub fn press_key(&mut self, key: &str) -> bool {
        self.inject_key(key, true)
    }

    pub fn release_key(&mut self, key: &str) -> bool {
        self.inject_key(key, false)
    }

    pub fn set_flip_state(&mut self, is_open: bool) {
        let ctx = self.uc.get_data_mut();
        ctx.gpio.set_flip_state(is_open);
        let stage = ctx.current_stage;
        ctx.display.st7735.render_cover_screen(stage, is_open);
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

    pub fn get_boot_stage(&self) -> Option<BootStage> {
        self.uc.get_data().current_stage
    }

    pub fn has_custom_framebuffer(&self) -> bool {
        self.uc.get_data().display.gc9307.has_custom_framebuffer
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
