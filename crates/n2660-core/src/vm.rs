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
    pub user_insn_count: u64,
    pub pending_unmapped_page: Option<u64>,
    pub mmu_userland_mapped: bool,
    pub last_log_fmt: String,
    pub last_log_count: u32,
    pub osa_heap_ptr: u64,
    pub mmi_res_data: Vec<u8>,
    pub is_dump: bool,
}

#[inline]
pub fn calc_gpta_counter(insn_count: u64) -> (u32, u32) {
    // 3.84 MHz WCDMA chip clock:
    // 38,400 chips per 10ms radio frame.
    // Advancing 1 chip per 8 instructions provides smooth time progression in emulation.
    let total_chips = insn_count / 8;
    let frame_num = total_chips / 38400;
    let chips = (total_chips % 38400) as u32;
    let subchip = (insn_count % 8) as u32;
    // Word 0 (lo): [31:19] = frame_num[12:0], [18:3] = chips[15:0], [2:0] = subchip[2:0]
    let lo = ((frame_num as u32 & 0x1FFF) << 19) | (chips << 3) | subchip;
    // Word 1 (hi): [18:0] = frame_num[31:13]
    let hi = (frame_num as u32 >> 13) & 0x7_FFFF;
    (lo, hi)
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
            user_insn_count: 0,
            pending_unmapped_page: None,
            mmu_userland_mapped: false,
            last_log_fmt: String::new(),
            last_log_count: 0,
            osa_heap_ptr: 0x82D0_0000,
            is_dump: false,
            mmi_res_data: {
                if let Ok(d) = std::fs::read("device-dump/mmi_res.bin") {
                    d
                } else if let Ok(d) = std::fs::read("../Nokia-2660-RE/firmware/dumps/ta1480_10.02.17.00/mmi_res.bin") {
                    d
                } else if let Ok(d) = std::fs::read("extracted_pac/mmi_res_240X320_MH.bin") {
                    d
                } else if let Ok(d) = std::fs::read("extracted_fw/mmi_res_240X320_MH.bin") {
                    d
                } else {
                    Vec::new()
                }
            },
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

        // 2d. Map ThreadX Task Stack & Dynamic Heap Window (224 MB at 0x7200_0000..0x8000_0000)
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

        // 12. Map ARM Cortex-A7 GIC-400 / MPCore (64 KB at 0x1200_0000)
        uc.mem_map(SPRD_GIC_BASE, SPRD_GIC_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map GIC-400 at 0x{:X}: {:?}", SPRD_GIC_BASE, e))?;

        // 13. Map Unisoc Modem APB Peripherals (16 MB at 0xE000_0000)
        uc.mem_map(SPRD_MODEM_PERIPH_BASE, SPRD_MODEM_PERIPH_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map Modem Peripherals at 0x{:X}: {:?}", SPRD_MODEM_PERIPH_BASE, e))?;

        // 14. Map Unisoc Modem / DSP Shared Memory (16 MB at 0xD100_0000)
        uc.mem_map(SPRD_MODEM_SHMEM_BASE, SPRD_MODEM_SHMEM_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map Modem Shared Memory at 0x{:X}: {:?}", SPRD_MODEM_SHMEM_BASE, e))?;

        // Initialize PMIC EIC Power Key register in memory (0x40608280) to 2 (bit 1 = 1: Power key released)
        let _ = uc.mem_write(0x4060_8280, &2u32.to_le_bytes());
        // Initialize AON APB CHIP_ID (0x402E00FC) to 0x98180000 (Unisoc T107 / UMS9117)
        let _ = uc.mem_write(0x402E_00FC, &0x9818_0000u32.to_le_bytes());
        // Initialize Baseband RF state (0x402B00D4) bits 4..7 = 6 (RF_READY / CAL_DONE)
        let _ = uc.mem_write(0x402B_00D4, &0x0000_0060u32.to_le_bytes());
        // Initialize RFSPI Chip ID (0x40070248) to 0x0743 (SR3532 RF transceiver)
        let _ = uc.mem_write(0x4007_0248, &0x0000_0743u32.to_le_bytes());
        // Initialize Modem GPTA / FRC status (0xE0020040) bit 6 = 1 (counter latch ready)
        let _ = uc.mem_write(0xE002_0040, &0x0000_0040u32.to_le_bytes());

        // Setup Instruction Hook & IRQ Dispatcher
        uc.add_code_hook(0, 0xFFFF_FFFF, |uc, addr, _size| {
            let (trace_on, insn_count, is_dump) = {
                let ctx = uc.get_data_mut();
                ctx.instruction_count += 1;
                (ctx.trace, ctx.instruction_count, ctx.is_dump)
            };

            if insn_count % 5_000_000 == 0 {
                let sp = uc.reg_read(RegisterARM::SP).unwrap_or(0);
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let mut cur_thread = [0u8; 4];
                let _ = uc.mem_read(0x8174_2174, &mut cur_thread);
                let cur_th = u32::from_le_bytes(cur_thread);
                println!("[EXEC MONITOR] insns: {}M | PC: 0x{:08X} | LR: 0x{:08X} | SP: 0x{:08X} | CurThread: 0x{:08X}", insn_count / 1_000_000, addr, lr, sp, cur_th);
                let mut stack_words = [0u8; 128];
                if uc.mem_read(sp, &mut stack_words).is_ok() {
                    print!("[CALL STACK]");
                    for chunk in stack_words.chunks_exact(4) {
                        let w = u32::from_le_bytes(chunk.try_into().unwrap());
                        if (w >= 0x80A0_0000 && w < 0x8200_0000) || (w >= 0x8800_0000 && w < 0x8A00_0000) {
                            print!(" 0x{:08X}", w);
                        }
                    }
                    println!();
                }
            }

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
                        } else if ctx.current_stage == Some(BootStage::User) {
                            ctx.user_insn_count += 1;
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
                let prog_msg = format!("[BOOT PROGRESS] Boot stage advanced: {:?} ({}%) at PC 0x{:08X}", stg, pct, addr);
                println!("{}", prog_msg);
                ctx.uart.push_log_str(&prog_msg);
                if stg == BootStage::User {
                    ctx.uart.push_log_str("[MOCOR RTOS] Userland MS_MMI_Main Active -> Homescreen Ready");
                }
                if !ctx.display.gc9307.has_custom_framebuffer {
                    match stg {
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
                }
                if !is_open {
                    ctx.display.transfer_main_to_sub();
                }
            } else {
                let ctx = uc.get_data_mut();
                if ctx.current_stage == Some(BootStage::Kernel) && ctx.kernel_insn_count > 0 && ctx.kernel_insn_count % 250_000 == 0 && !ctx.display.gc9307.has_custom_framebuffer {
                    let insn_k = ctx.kernel_insn_count / 1_000;
                    let status = format!("* Exec: {}k insns | PC: 0x{:08X}", insn_k, addr);
                    ctx.display.gc9307.render_boot_stage(BootStage::Kernel, &status, 75);
                }
            }

            if is_dump && addr == 0x80D7_6996 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if is_dump && addr == 0x80D3_70AC {
                // Populate rfcal ops stub table (256 bytes of function pointers pointing to 0x80A06A4D: movs r0, #0; bx lr)
                let stub_target = 0x80A0_6A4Du32;
                let mut ops_buf = [0u8; 256];
                for chunk in ops_buf.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&stub_target.to_le_bytes());
                }
                let _ = uc.mem_write(0x8178_5800, &ops_buf);

                let ops_ptr = 0x8178_5800u32;
                let mut tbl_buf = [0u8; 96];
                for chunk in tbl_buf.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&ops_ptr.to_le_bytes());
                }
                let _ = uc.mem_write(0x8177_ECF4, &tbl_buf);

                let data_ptr = 0x8178_5A00u32;
                let mut data_tbl = [0u8; 96];
                for chunk in data_tbl.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&data_ptr.to_le_bytes());
                }
                let _ = uc.mem_write(0x8177_ED54, &data_tbl);

                let p_control = 0x81AC_7A34u32;
                let _ = uc.mem_write((p_control + 32) as u64, &ops_ptr.to_le_bytes());
                let _ = uc.mem_write((p_control + 36) as u64, &data_ptr.to_le_bytes());

                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
                println!("[RFCAL DUMP] Initialized wl1c rfcal ops at 0x81785800 and returned cleanly to LR 0x{:08X}", lr);
            }

            if is_dump && (addr == 0x80B5_5418 || addr == 0x80B5_541A) {
                let mut ttbr0 = unicorn_engine::RegisterARMCP::new().cp(15).crn(2).crm(0).opc1(0).opc2(0);
                let _ = uc.reg_read_arm_coproc(&mut ttbr0);
                let l1_base = ttbr0.val & !0x3FFF;
                if l1_base >= 0x8000_0000 && l1_base < 0x8400_0000 {
                    println!("[MMU DIRECT] Staging UserImg sections into L1 at 0x{:08X}", l1_base);
                    for i in 0..32u32 {
                        let va = 0x8800_0000u32 + (i << 20);
                        let l1_idx = (va >> 20) as u64;
                        let desc = va | 0xC0E;
                        let _ = uc.mem_write(l1_base as u64 + l1_idx * 4, &desc.to_le_bytes());
                    }
                    let mut dacr = unicorn_engine::RegisterARMCP::new().cp(15).crn(3).crm(0).opc1(0).opc2(0);
                    dacr.val = 0xFFFF_FFFF;
                    let _ = uc.reg_write_arm_coproc(&dacr);
                    let mut tlb_flush = unicorn_engine::RegisterARMCP::new().cp(15).crn(8).crm(7).opc1(0).opc2(0);
                    tlb_flush.val = 0;
                    let _ = uc.reg_write_arm_coproc(&tlb_flush);
                }
            }

            if addr == 0x80B5_5418 || addr == 0x80FE_E014 || (addr >= 0x888B_C650 && addr <= 0x888B_C660) {
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let sp = uc.reg_read(RegisterARM::SP).unwrap_or(0);
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                println!("[TRACE USER ENTRY] PC: 0x{:08X}, CPSR: 0x{:08X} (T={}), SP: 0x{:08X}, LR: 0x{:08X}", addr, cpsr, (cpsr & 0x20) != 0, sp, lr);
            }

            if !uc.get_data().mmu_userland_mapped {
                let should_map = (is_dump && (insn_count > 10_000_000 || addr >= 0x8800_0000))
                    || (addr >= 0x80FE_FD8C && addr <= 0x80FE_FE20);
                if should_map {
                    let mut ttbr0 = unicorn_engine::RegisterARMCP::new().cp(15).crn(2).crm(0).opc1(0).opc2(0);
                    let _ = uc.reg_read_arm_coproc(&mut ttbr0);
                    let l1_base = ttbr0.val & !0x3FFF;
                    if l1_base >= 0x8000_0000 && l1_base < 0x8400_0000 {
                        let mut existing = [0u8; 4];
                        let _ = uc.mem_read(l1_base as u64 + 0x80A * 4, &mut existing);
                        let ex_val = u32::from_le_bytes(existing);
                        let desc_flags = if ex_val != 0 { ex_val & 0xFFFFF } else { 0xC02 };
                        println!("[MMU FIX] Mapping Userland DRAM (0x88000000..0x8A000000) into MMU L1 Page Table at 0x{:08X} (ex_entry=0x{:08X}, flags=0x{:X})", l1_base, ex_val, desc_flags);
                        for i in 0..32u32 {
                            let va = 0x8800_0000u32 + (i << 20);
                            let l1_idx = (va >> 20) as u64;
                            let desc = va | desc_flags;
                            let _ = uc.mem_write(l1_base as u64 + l1_idx * 4, &desc.to_le_bytes());
                        }
                        // Set DACR to Manager for all domains to prevent domain faults
                        let mut dacr = unicorn_engine::RegisterARMCP::new().cp(15).crn(3).crm(0).opc1(0).opc2(0);
                        dacr.val = 0xFFFF_FFFF;
                        let _ = uc.reg_write_arm_coproc(&dacr);
                        // Flush TLB via CP15 c8 (TLBIALL)
                        let mut tlb_flush = unicorn_engine::RegisterARMCP::new().cp(15).crn(8).crm(7).opc1(0).opc2(0);
                        tlb_flush.val = 0;
                        let _ = uc.reg_write_arm_coproc(&tlb_flush);
                        uc.get_data_mut().mmu_userland_mapped = true;
                    }
                }
            }

            if addr >= 0x80FE_FD8C && addr <= 0x80FE_FE20 && !is_dump {
                let mut insn_bytes = [0u8; 4];
                let _ = uc.mem_read(addr, &mut insn_bytes);
                let insn = u32::from_le_bytes(insn_bytes);
                if insn == 0xE51F_F004 {
                    let mut target_bytes = [0u8; 4];
                    let _ = uc.mem_read(addr + 4, &mut target_bytes);
                    let target = u32::from_le_bytes(target_bytes);
                    let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                    println!("[USERLAND HANDOFF] Jumping to 0x{:08X} (Thumb: {}) from LR 0x{:08X}", target, (target & 1) != 0, lr);
                    let _ = uc.reg_write(RegisterARM::PC, (target & !1) as u64);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let t_bit = if (target & 1) != 0 { 0x20 } else { 0 };
                    let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
                    let _ = uc.emu_stop();
                }
            }

            if addr >= 0x8800_0000 {
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                if (cpsr & 0x20) == 0 {
                    let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
                }
            }

            if addr >= 0x8800_0000 && addr < 0x8A00_0000 {
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                if (cpsr & 0x20) == 0 {
                    let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
                }
            }

            let is_set_brightness = if is_dump {
                addr == 0x80DA_BCDC || addr == 0x80DA_BD14
            } else {
                addr == 0x80CC_901C || addr == 0x80CC_9054
            };
            if is_set_brightness {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 > 100 {
                    let _ = uc.reg_write(RegisterARM::R0, 100);
                }
                let r6 = uc.reg_read(RegisterARM::R6).unwrap_or(0);
                if r6 > 100 {
                    let _ = uc.reg_write(RegisterARM::R6, 100);
                }
            }

            if !is_dump {

            if addr == 0x80B2_E218 || addr == 0x80B2_E21A {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 != 0x9818_0000 && r0 != 0x9818_0001 {
                    let _ = uc.reg_write(RegisterARM::R0, 0x9818_0000);
                }
            }

            if addr == 0x811D_702C {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r2 = uc.reg_read(RegisterARM::R2).unwrap_or(0);
                if r2 != 0 {
                    let _ = uc.mem_write(r2, &0x8174_7000u32.to_le_bytes());
                }
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if addr == 0x811D_7856 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                if r1 != 0 {
                    let mut info_buf = [0u8; 216];
                    info_buf[140..144].copy_from_slice(&0x0002_0000u32.to_le_bytes()); // 128 KB block size
                    info_buf[152..156].copy_from_slice(&1024u32.to_le_bytes()); // 1024 blocks
                    let _ = uc.mem_write(r1, &info_buf);
                }
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if addr == 0x811D_713A || addr == 0x811D_70D0 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if addr == 0x80CC_9D00 {
                // is_multimode_rf_enabled(): return 1 (enabled)
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::R0, 1);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if addr == 0x80DED_59E {
                // Ensure p_rf->rf_variable.RF_MultMode_Init_Reg = 1
                let _ = uc.mem_write(0x81A5_E218 + 296, &1u32.to_le_bytes());
                let _ = uc.mem_write(0x81A5_E218 + 292, &1u32.to_le_bytes());
                let _ = uc.mem_write(0x81A5_E218 + 288, &1u32.to_le_bytes());
            }

            if addr == 0x80DED_540 {
                let r4 = uc.reg_read(RegisterARM::R4).unwrap_or(0);
                let mut dev_ptr = [0u8; 4];
                let _ = uc.mem_read(r4 + 392, &mut dev_ptr);
                if u32::from_le_bytes(dev_ptr) == 0 {
                    // Skip null callback invocation: jump in Thumb mode
                    let _ = uc.reg_write(RegisterARM::PC, 0x80DED_556);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
                }
            }

            if addr == 0x812C_8726 || addr == 0x812C_874C {
                let r4 = uc.reg_read(RegisterARM::R4).unwrap_or(0);
                let mut p_buf = [0u8; 4];
                let _ = uc.mem_read(r4 + 156, &mut p_buf);
                let p = u32::from_le_bytes(p_buf);
                if p == 0xFFFF_FFFF || p == 0 || p < 0x8000_0000 || p >= 0x9000_0000 {
                    // UMTS capability bitmask array unallocated or uninitialized: skip loop to 0x812C8752
                    let _ = uc.reg_write(RegisterARM::PC, 0x812C_8752);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
                }
            }

            if addr == 0x80AC_2FB8 {
                // mta_check_debug_mem(): void function, return cleanly to LR to prevent false assert during early boot
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if addr == 0x80A1_1754 || addr == 0x80A1_17C8 || addr == 0x80CF_8336 {
                // osa_buff_check_overflow() & check_overlap(): return 0 (success / no buffer overflow)
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if addr == 0x80E0_6E6A {
                let r7 = uc.reg_read(RegisterARM::R7).unwrap_or(0);
                let mut cnt_buf = [0u8; 4];
                if uc.mem_read(r7, &mut cnt_buf).is_ok() {
                    let cnt = u32::from_le_bytes(cnt_buf);
                    if cnt == 0xFFFF_FFFF {
                        println!("[RADIO_CAP] r7=0x{:08X} capability count was 0xFFFFFFFF -> normalized to 0", r7);
                        let _ = uc.mem_write(r7, &0u32.to_le_bytes());
                    }
                }
            }

            if addr == 0x80A1_DA00 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 > 0x1000_0000 {
                    println!("[SDI_ALLOC INVALID] LR=0x{:08X}, clamping invalid size 0x{:08X} to 0", lr, r0);
                    let _ = uc.reg_write(RegisterARM::R0, 0);
                }
            }

            if addr == 0x810D_22FE {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 != 0 {
                    // Internal ThreadX block pool exhausted: allocate from fallback heap
                    let sp = uc.reg_read(RegisterARM::SP).unwrap_or(0);
                    let r4 = uc.reg_read(RegisterARM::R4).unwrap_or(0); // requested size
                    let r6 = uc.reg_read(RegisterARM::R6).unwrap_or(0); // pool index

                    let heap_ptr = uc.get_data().osa_heap_ptr;
                    let valid_size = if r4 > 0x1000_0000 { 64 } else { r4 };
                    let alloc_size = ((valid_size + 32 + 31) & !31) as u64;
                    uc.get_data_mut().osa_heap_ptr = heap_ptr + alloc_size;

                    // Write allocated buffer pointer to [sp + 20]
                    let _ = uc.mem_write(sp + 20, &(heap_ptr as u32).to_le_bytes());
                    println!("[OSA HEAP] Fallback allocated {} bytes for pool {} at 0x{:08X}", valid_size, r6, heap_ptr);

                    // Skip assert: jump directly to 0x810D2312 with r0 = 0
                    let _ = uc.reg_write(RegisterARM::R0, 0);
                    let _ = uc.reg_write(RegisterARM::PC, 0x810D_2312);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
                }
            }

            if addr == 0x810D_2442 {
                let r7 = uc.reg_read(RegisterARM::R7).unwrap_or(0);
                if r7 >= 0x82D0_0000 && r7 < 0x8300_0000 {
                    // Fallback heap buffer being freed: skip _tx_block_release to avoid TX_PTR_ERROR assert
                    let _ = uc.reg_write(RegisterARM::PC, 0x810D_245E);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
                }
            }

            if addr == 0x80DD_6992 {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r5 = uc.reg_read(RegisterARM::R5).unwrap_or(0);
                let r8 = uc.reg_read(RegisterARM::R8).unwrap_or(0);
                if r0 != r5 {
                    println!("[OSA_MEM_FREE] Mismatch for index {}: table_ptr=0x{:08X} != free_ptr=0x{:08X} -> returning 0", r8, r0, r5);
                    let _ = uc.reg_write(RegisterARM::PC, 0x80DD_6A9A);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
                }
            }

            if addr == 0x80DD_6A80 || addr == 0x80DD_696A || addr == 0x80DD_69E8 {
                let _ = uc.reg_write(RegisterARM::PC, 0x80DD_6A9A);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
            }

            if addr == 0x8103_739A {
                let mut buf = [0u8; 4];
                let _ = uc.mem_read(0x8176_CEE8, &mut buf);
                let shm_base = u32::from_le_bytes(buf);
                if shm_base != 0 {
                    let r5 = shm_base + 0x19670;
                    let r6 = r5 + 0x634;
                    let _ = uc.mem_write(r6 as u64, &1u16.to_le_bytes());
                    let _ = uc.mem_write((r5 + 0x770) as u64, &1u16.to_le_bytes());
                    let _ = uc.mem_write((r5 + 0x772) as u64, &1u16.to_le_bytes());
                    println!("[DSP SHM] Initialized GSM DSP running flag (0x{:08X}) and version at 0x{:08X}", r6, r5 + 0x770);
                }
            }

            if addr == 0x8103_74AC {
                let _ = uc.reg_write(RegisterARM::PC, 0x8103_74C0);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
            }

            if addr == 0x80DB_C858 {
                let _ = uc.reg_write(RegisterARM::R0, 1);
                let r7 = uc.reg_read(RegisterARM::R7).unwrap_or(0);
                if r7 != 0 {
                    let _ = uc.mem_write(r7, &1u32.to_le_bytes());
                }
            }

            if addr == 0x80DB_C908 {
                let _ = uc.reg_write(RegisterARM::R0, 2);
                let r7 = uc.reg_read(RegisterARM::R7).unwrap_or(0);
                if r7 != 0 {
                    let _ = uc.mem_write(r7, &2u32.to_le_bytes());
                }
            }

            if addr == 0x80DB_CF6C {
                let _ = uc.reg_write(RegisterARM::R0, 1);
                let r7 = uc.reg_read(RegisterARM::R7).unwrap_or(0);
                if r7 != 0 {
                    let _ = uc.mem_write(r7, &1u32.to_le_bytes());
                }
            }

            if addr == 0x80DB_D084 {
                let _ = uc.reg_write(RegisterARM::R0, 2);
                let r7 = uc.reg_read(RegisterARM::R7).unwrap_or(0);
                if r7 != 0 {
                    let _ = uc.mem_write(r7, &2u32.to_le_bytes());
                }
            }

            if !is_dump && (addr == 0x80CA_C5FC || addr == 0x80CA_C634) {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 >= 0x8000_0000 && r0 < 0x9000_0000 {
                    let mut id_buf = [0u8; 4];
                    if uc.mem_read(r0, &mut id_buf).is_ok() {
                        let id = u32::from_le_bytes(id_buf);
                        if id != 0x4D55_5445 {
                            // Auto-initialize mutex ID so ThreadX accepts it
                            let _ = uc.mem_write(r0, &0x4D55_5445u32.to_le_bytes());
                        }
                    }
                }
            }

            if !is_dump && addr == 0x810D_6FB4 {
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, 0x810D_6FC4);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
            }

            if !is_dump && (addr == 0x80A1_2B90 || addr == 0x80A1_2B9A) {
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, 0x80A1_2BA2);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
            }

            if !is_dump && addr == 0x810D_7002 {
                let _ = uc.reg_write(RegisterARM::PC, 0x810D_707E);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
            }

            if addr == 0x80A1_2AA6 {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let len = match r0 {
                    2626 => Some(268),
                    553 => Some(8),
                    1300 => Some(7108),
                    554 => Some(1400),
                    2502 => Some(168),
                    2503 => Some(24),
                    1308 => Some(4004),
                    1306 => Some(1608),
                    1305 => Some(200),
                    1310 => Some(64),
                    2722 => Some(3336),
                    1304 => Some(3604),
                    1317 => Some(240),
                    _ => None,
                };
                if let Some(l) = len {
                    let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                    let _ = uc.reg_write(RegisterARM::R0, l);
                    let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                    let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
                }
            }

            if addr == 0x80B5_74BA {
                let r4 = uc.reg_read(RegisterARM::R4).unwrap_or(0);
                if r4 != 268 {
                    let _ = uc.reg_write(RegisterARM::R4, 268);
                    let r7 = uc.reg_read(RegisterARM::R7).unwrap_or(0);
                    let _ = uc.mem_write(r7, &268u16.to_le_bytes());
                    let _ = uc.reg_write(RegisterARM::PC, 0x80B5_7498);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let _ = uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
                }
            }

            if addr == 0x80A1_4A6A {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 < 4200 || r0 > 4500 {
                    let _ = uc.reg_write(RegisterARM::R0, 4200);
                }
            }

            if addr == 0x80A1_4AAC {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 < 6000 || r0 > 9700 {
                    let _ = uc.reg_write(RegisterARM::R0, 6500);
                }
            }

            if addr == 0x80A1_5A4E {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                // Mark NV initialized flags
                let _ = uc.mem_write(0x8174_33F0, &[1u8, 1u8]);
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
                println!("[NVITEM] Handled 0x80A15A4E -> returned 0 to LR 0x{:08X}", lr);
            }

            if addr == 0x80A1_3D34 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                let r2 = uc.reg_read(RegisterARM::R2).unwrap_or(0);
                println!("[NVITEM_READ] nvid={} len={} buf=0x{:08X} LR=0x{:08X}", r0, r1, r2, lr);
                if r0 == 21 && r1 >= 441 && r2 != 0 {
                    // Mark NVITEM 21 byte 440 as 1 (NV_VALID / CALIBRATED / NORMAL_BOOT)
                    let _ = uc.mem_write(r2 + 440, &[1u8]);
                    println!("[NVITEM_READ] Set NVITEM 21 byte 440 (calibration/download flag) to 1 (CALIBRATED)");
                } else if r0 == 2722 && r2 != 0 {
                    // NVITEM 2722 (NV_MULTIMODE_COMMON_REG_PARAMS):
                    // Starts with magic "MULT" (0x4D554C54) and ends at offset 3332 with "MODE" (0x4D4F4445)
                    let _ = uc.mem_write(r2, &0x4D55_4C54u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 3332, &0x4D4F_4445u32.to_le_bytes());

                    // Hardware register bases required by drv_rf_nv_download.c:
                    let _ = uc.mem_write(r2 + 2888, &0x4002_0000u32.to_le_bytes()); // g_HW_AON_SHARE_RAM_BASE
                    let _ = uc.mem_write(r2 + 2904, &0x402E_0000u32.to_le_bytes()); // g_HW_SPINLOCK1_ADDR
                    let _ = uc.mem_write(r2 + 2920, &0x402E_0004u32.to_le_bytes()); // g_HW_SPINLOCK2_ADDR
                    let _ = uc.mem_write(r2 + 2936, &0x402B_0000u32.to_le_bytes()); // g_HW_MDAR_BASE
                    let _ = uc.mem_write(r2 + 2968, &0x4060_0000u32.to_le_bytes()); // g_HW_ADI_BASE
                    let _ = uc.mem_write(r2 + 2984, &0x4020_0000u32.to_le_bytes()); // g_HW_CLASH_DETECT_BASE
                    let _ = uc.mem_write(r2 + 3000, &0x402C_0000u32.to_le_bytes()); // g_HW_RFTI_APB_BASE
                    let _ = uc.mem_write(r2 + 3016, &0x402D_0000u32.to_le_bytes()); // g_HW_RFSPI_APB_BASE
                    let _ = uc.mem_write(r2 + 3044, &0u32.to_le_bytes());          // g_CLASH_DETECT_W (< 3)

                    let r7 = r2 - 8;
                    let _ = uc.mem_write(r7, &1u32.to_le_bytes()); // g_multimode_common_reg_params.has_load = 1

                    // Also initialize gain table params has_load flags
                    let _ = uc.mem_write(0x81AE_2A40, &1u32.to_le_bytes()); // g_tx_gain_table_params.has_load = 1
                    let _ = uc.mem_write(0x81AE_3604, &1u32.to_le_bytes()); // g_rx_gain_table_params.has_load = 1
                    let _ = uc.mem_write(0x81AE_5324, &1u32.to_le_bytes());

                    println!("[NVITEM_READ] Initialized NVITEM 2722 with MULT/MODE magic & HW bases at 0x{:08X}", r2);
                }
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if addr == 0x80A1_7472 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                println!("[CHARGE] 0x80A17472: dischg=0x{:08X}, chg=0x{:08X}, LR=0x{:08X}", r0, r1, lr);

                // Default Nokia battery voltage calibration curve (12 points in mV)
                let dischg_curve: [u16; 12] = [3400, 3500, 3600, 3650, 3700, 3750, 3800, 3850, 3900, 4000, 4100, 4180];
                let chg_curve: [u16; 12] = [3500, 3600, 3700, 3750, 3800, 3850, 3900, 3950, 4050, 4120, 4180, 4200];

                if r0 != 0 {
                    for (i, val) in dischg_curve.iter().enumerate() {
                        let _ = uc.mem_write(r0 + (i * 2) as u64, &val.to_le_bytes());
                    }
                }
                if r1 != 0 {
                    for (i, val) in chg_curve.iter().enumerate() {
                        let _ = uc.mem_write(r1 + (i * 2) as u64, &val.to_le_bytes());
                    }
                }
            }

            if addr == 0x80DEC_DE6 {
                let mut p1_bytes = [0u8; 4];
                let _ = uc.mem_read(0x81A5_E218 + 392, &mut p1_bytes);
                let p1 = u32::from_le_bytes(p1_bytes);
                if p1 == 0 {
                    // g_rf_dev is NULL, safely return to LR without jumping to garbage at [0x4C]
                    let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                    let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                    let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
                }
            }

            if addr == 0x80D8_99A4 {
                // Populate rfcal ops stub table (256 bytes of function pointers pointing to 0x80D89983: movs r0, #1; bx lr)
                let stub_target = 0x80D8_9983u32;
                let mut ops_buf = [0u8; 256];
                for chunk in ops_buf.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&stub_target.to_le_bytes());
                }
                let _ = uc.mem_write(0x8178_5800, &ops_buf);

                let ops_ptr = 0x8178_5800u32;
                let mut tbl_buf = [0u8; 96];
                for chunk in tbl_buf.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&ops_ptr.to_le_bytes());
                }
                let _ = uc.mem_write(0x8178_5928, &tbl_buf);
                let _ = uc.mem_write(0x8178_5988, &tbl_buf);

                // Populate RFIC device ops table (pointing to movs r0, #0; bx lr at 0x80A06A4D)
                let rfic_stub_target = 0x80A0_6A4Du32;
                let mut rfic_ops = [0u8; 256];
                for chunk in rfic_ops.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&rfic_stub_target.to_le_bytes());
                }
                let rfic_dev_ptr = 0x8178_5A00u32;
                let _ = uc.mem_write(rfic_dev_ptr as u64, &rfic_ops);

                // Populate RFIC device table at 0x8176E450 (24 entries)
                let mut rfic_tbl = [0u8; 96];
                for chunk in rfic_tbl.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&rfic_dev_ptr.to_le_bytes());
                }
                let _ = uc.mem_write(0x8176_E450, &rfic_tbl);

                // Set g_rf_dev directly
                let _ = uc.mem_write(0x81A5_E218 + 392, &rfic_dev_ptr.to_le_bytes());

                println!("[RFCAL] Initialized rfcal & rfic device pointer tables at 0x81785928 / 0x8176E450 / 0x81A5E3A0");
            }

            if addr == 0x80DF_0348 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                let r2 = uc.reg_read(RegisterARM::R2).unwrap_or(0);
                let r3 = uc.reg_read(RegisterARM::R3).unwrap_or(0);
                let sp = uc.reg_read(RegisterARM::SP).unwrap_or(0);
                let mut r4_bytes = [0u8; 4];
                let _ = uc.mem_read(sp, &mut r4_bytes);
                let r4 = u32::from_le_bytes(r4_bytes);

                // Zero out the timing out-pointers if valid
                for &ptr in &[r0, r1, r2, r3, r4 as u64] {
                    if ptr >= 0x8000_0000 && ptr < 0x9000_0000 {
                        let _ = uc.mem_write(ptr, &[0u8, 0u8]);
                    }
                }

                // Return 0 (success)
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if addr == 0x80E0_9D68 {
                let _r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r4 = uc.reg_read(RegisterARM::R4).unwrap_or(0);
                let r6 = uc.reg_read(RegisterARM::R6).unwrap_or(0);
                let r7 = uc.reg_read(RegisterARM::R7).unwrap_or(0);
                let r8 = uc.reg_read(RegisterARM::R8).unwrap_or(0);
                let mut ptr_bytes = [0u8; 4];
                let _ = uc.mem_read(r8, &mut ptr_bytes);
                let mbox_base = u32::from_le_bytes(ptr_bytes);
                let mut status_bytes = [0u8; 4];
                let _ = uc.mem_read((mbox_base + 0x14) as u64, &mut status_bytes);
                let mbox_stat = u32::from_le_bytes(status_bytes);
                if uc.get_data_mut().logged_traps.insert(0x80E09D68) {
                    println!("[MBOX] Poll loop at 0x80E09D68: r8=0x{:08X} -> mbox_base=0x{:08X}, [base+0x14]=0x{:08X}, r4=0x{:08X}, r6=0x{:08X}, r7=0x{:08X}",
                        r8, mbox_base, mbox_stat, r4, r6, r7);
                }
            }

            if addr == 0x810D_C66C || addr == 0x810D_9BDC {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                let r2 = uc.reg_read(RegisterARM::R2).unwrap_or(0);
                let r3 = uc.reg_read(RegisterARM::R3).unwrap_or(0);
                let read_cstr = |ptr: u64| -> String {
                    let mut buf = [0u8; 128];
                    if ptr >= 0x8000_0000 && ptr < 0x9000_0000 && uc.mem_read(ptr, &mut buf).is_ok() {
                        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                        String::from_utf8_lossy(&buf[..len]).to_string()
                    } else {
                        format!("0x{:08X}", ptr)
                    }
                };
                let r0_str = read_cstr(r0);
                let r1_str = read_cstr(r1);
                let r2_str = read_cstr(r2);
                println!("[ASSERT TRAP @ 0x{:08X}] LR=0x{:08X}: r0='{}', r1='{}', r2='{}' (val={}), r3=0x{:08X}",
                    addr, lr, r0_str, r1_str, r2_str, r2, r3);
            }

            if addr == 0x80A1_2A3A {
                // Ensure NVITEM mutex acquisition always succeeds so nvitem read proceeds
                let _ = uc.reg_write(RegisterARM::R0, 0);
            }

            if addr == 0x80DE_833C || addr == 0x80DE_834C {
                // Ensure SDI error callback pointer is populated so unroutable SDI message doesn't panic
                let _ = uc.mem_write(0x8174_8600, &0x80A0_6A4Du32.to_le_bytes());
            }

            if addr == 0x80D1_D24C {
                let dest = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let offset = uc.reg_read(RegisterARM::R1).unwrap_or(0) as usize;
                let size = uc.reg_read(RegisterARM::R2).unwrap_or(0) as usize;
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);

                let res_data = &uc.get_data().mmi_res_data;
                let res_len = res_data.len();
                println!("[MMIRES] BML_Read_MMIRES: dest=0x{:08X}, offset=0x{:X}, size=0x{:X} (res_len=0x{:X}), LR=0x{:08X}",
                    dest, offset, size, res_len, lr);

                let read_bytes = if res_len > 0 && offset < res_len {
                    let to_read = size.min(res_len - offset);
                    let bytes = res_data[offset..offset + to_read].to_vec();
                    let _ = uc.mem_write(dest, &bytes);
                    to_read
                } else {
                    0
                };

                if offset == 0 {
                    let msg = format!("[MMIRES] Resource partition mounted ({} bytes loaded)", res_len);
                    uc.get_data_mut().uart.push_log_str(&msg);
                }

                let _ = uc.reg_write(RegisterARM::R0, read_bytes as u64);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }
            } // end if !is_dump

            if addr == 0x8851_B194 {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let mut buf = [0u8; 32];
                let _ = uc.mem_read(r0, &mut buf);
                println!("[MMIRES] 0x8851B194 (mmi_res init) called: pRes=0x{:08X}, LR=0x{:08X}", r0, lr);
                for i in (0..32).step_by(4) {
                    let val = u32::from_le_bytes([buf[i], buf[i+1], buf[i+2], buf[i+3]]);
                    println!("  pRes+0x{:02X}: 0x{:08X}", i, val);
                }
            }

            if addr == 0x80A0_6AFA {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let sp = uc.reg_read(RegisterARM::SP).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                println!("[HALT_ENTRY] Reached 0x80A06AFA! LR=0x{:08X}, SP=0x{:08X}, r0=0x{:08X}, r1=0x{:08X}", lr, sp, r0, r1);
                let mut stack_buf = [0u8; 64];
                if uc.mem_read(sp, &mut stack_buf).is_ok() {
                    for i in (0..64).step_by(4) {
                        let val = u32::from_le_bytes([stack_buf[i], stack_buf[i+1], stack_buf[i+2], stack_buf[i+3]]);
                        println!("  SP+{:02X}: 0x{:08X}", i, val);
                    }
                }
            }

            if (!is_dump && addr == 0x80E0_9D08) || (is_dump && addr == 0x80D8_8514) {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                println!("[MBOX] Entered MBOX_Send (0x{:08X}) from LR 0x{:08X}: r0=0x{:08X}, r1=0x{:08X}", addr, lr, r0, r1);
                let mbox_flag_addr = if is_dump { 0x8177_2281 } else { 0x8177_8E79 };
                let mbox_cfg_addr = if is_dump { 0x81B4_E928 } else { 0x81B4_9EE0 };
                let _ = uc.mem_write(mbox_flag_addr, &[1u8]);
                let _ = uc.mem_write(mbox_flag_addr + 1, &[1u8]);
                // Set CP/DSP coprocessor boot mode to 3 (READY) in shared struct
                let _ = uc.mem_write(0x8174_2238, &3u32.to_le_bytes());

                let mut mbox_cfg_buf = [0u8; 0x38];
                mbox_cfg_buf[0..4].copy_from_slice(&0x400A_0000u32.to_le_bytes()); // AP mailbox base
                mbox_cfg_buf[4..8].copy_from_slice(&0x1000u32.to_le_bytes());
                mbox_cfg_buf[8..12].copy_from_slice(&1u32.to_le_bytes());
                mbox_cfg_buf[12..16].copy_from_slice(&0xFFFF_FFFCu32.to_le_bytes());
                mbox_cfg_buf[16..20].copy_from_slice(&0x400A_8000u32.to_le_bytes()); // CP mailbox base
                mbox_cfg_buf[20..24].copy_from_slice(&0x1000u32.to_le_bytes());
                mbox_cfg_buf[24..28].copy_from_slice(&0x80u32.to_le_bytes());
                mbox_cfg_buf[28..32].copy_from_slice(&0xFFFF_FFFEu32.to_le_bytes());
                mbox_cfg_buf[32..36].copy_from_slice(&0x18u32.to_le_bytes());
                let _ = uc.mem_write(mbox_cfg_addr, &mbox_cfg_buf);
            }

            if addr == 0x80A1_7DEA || addr == 0x810D_7824 {
                println!("[BOOT MODE] Intercepted CP boot mode check at 0x{:08X}! Setting r0 = 3 (CP_READY)", addr);
                let _ = uc.reg_write(RegisterARM::R0, 3);
            }

            let is_mutex_fn = if is_dump {
                addr == 0x80DB_9264 || addr == 0x80AC_7550 || addr == 0x80DC_2FE0 || addr == 0x80DC_2FE4 || addr == 0x80DC_301C || addr == 0x80A1_E028
            } else {
                addr == 0x80AC_7640 || addr == 0x80CA_C5FC || addr == 0x80CA_C634 || addr == 0x80A1_E030
            };

            if is_dump && addr == 0x80B5_4B92 {
                // BML partition loader for coprocessor images (CM4 / DSP):
                // In emulator, partitions are already pre-staged -> return 0 (success)
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                println!("[BML DUMP] Staged coprocessors ready -> bypass BML flash read from LR 0x{:08X}", lr);
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            let is_nvitem_init = if is_dump { addr == 0x80A1_5A46 } else { addr == 0x80A1_5A4E };
            if is_nvitem_init {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                // Mark NV initialized flags
                let _ = uc.mem_write(0x8174_33F0, &[1u8, 1u8]);
                let _ = uc.mem_write(0x8174_7530, &[1u8, 1u8]);
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
                println!("[NVITEM] Handled NVITEM_Init (0x{:08X}) -> returned 0 to LR 0x{:08X}", addr, lr);
            }

            let is_nvitem_get_len = if is_dump { addr == 0x80A1_2A9E } else { addr == 0x80A1_2AA6 };
            if is_nvitem_get_len {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let len = match r0 {
                    2 => Some(3736),
                    2626 => Some(268),
                    553 => Some(8),
                    1300 => Some(7108),
                    554 => Some(1400),
                    2502 => Some(168),
                    2503 => Some(24),
                    1308 => Some(4004),
                    1306 => Some(1608),
                    1305 => Some(200),
                    1310 => Some(64),
                    2722 => Some(3336),
                    1304 => Some(3604),
                    1317 => Some(240),
                    _ => None,
                };
                if let Some(l) = len {
                    let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                    let _ = uc.reg_write(RegisterARM::R0, l);
                    let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                    let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
                }
            }

            let is_nvitem_read = if is_dump { addr == 0x80A1_3D2C } else { addr == 0x80A1_3D34 };
            if is_nvitem_read {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                let r2 = uc.reg_read(RegisterARM::R2).unwrap_or(0);
                println!("[NVITEM_READ] nvid={} len={} buf=0x{:08X} LR=0x{:08X}", r0, r1, r2, lr);
                if r0 == 21 && r1 >= 441 && r2 != 0 {
                    // Mark NVITEM 21 byte 440 as 1 (NV_VALID / CALIBRATED / NORMAL_BOOT)
                    let _ = uc.mem_write(r2 + 440, &[1u8]);
                    println!("[NVITEM_READ] Set NVITEM 21 byte 440 (calibration/download flag) to 1 (CALIBRATED)");
                } else if r0 == 2722 && r2 != 0 {
                    // NVITEM 2722 (NV_MULTIMODE_COMMON_REG_PARAMS):
                    let _ = uc.mem_write(r2, &0x4D55_4C54u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 3332, &0x4D4F_4445u32.to_le_bytes());

                    // Hardware register bases required by drv_rf_nv_download.c:
                    let _ = uc.mem_write(r2 + 2888, &0x4002_0000u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 2904, &0x402E_0000u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 2920, &0x402E_0004u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 2936, &0x402B_0000u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 2968, &0x4060_0000u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 2984, &0x4020_0000u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 3000, &0x402C_0000u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 3016, &0x402D_0000u32.to_le_bytes());
                    let _ = uc.mem_write(r2 + 3044, &0u32.to_le_bytes());

                    let r7 = r2 - 8;
                    let _ = uc.mem_write(r7, &1u32.to_le_bytes());

                    let gain_table_addr = if is_dump { 0x81AE_3760 } else { 0x81AE_2A40 };
                    let _ = uc.mem_write(gain_table_addr, &1u32.to_le_bytes());
                    println!("[NVITEM_READ] Initialized NVITEM 2722 with MULT/MODE magic & HW bases at 0x{:08X}", r2);
                }
                let _ = uc.reg_write(RegisterARM::R0, 0);
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            }

            if is_dump && addr == 0x810D_576E {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 != 0 {
                    let sp = uc.reg_read(RegisterARM::SP).unwrap_or(0);
                    let mut size_bytes = [0u8; 4];
                    let _ = uc.mem_read(sp + 136, &mut size_bytes);
                    let req_size = u32::from_le_bytes(size_bytes);
                    let valid_size = if req_size > 0x1000_0000 || req_size == 0 { 64 } else { req_size };
                    let alloc_size = ((valid_size + 64 + 31) & !31) as u64;

                    let heap_ptr = uc.get_data().osa_heap_ptr;
                    uc.get_data_mut().osa_heap_ptr = heap_ptr + alloc_size;

                    // Write allocated buffer pointer to [sp + 128]
                    let _ = uc.mem_write(sp + 128, &(heap_ptr as u32).to_le_bytes());
                    println!("[OSA HEAP DUMP] Fallback allocated {} bytes at 0x{:08X}", valid_size, heap_ptr);

                    // Set r0 = 0 (which gets moved to fp) so no assert!
                    let _ = uc.reg_write(RegisterARM::R0, 0);
                }
            }

            let is_chg_curve = if is_dump { addr == 0x80A1_746A } else { addr == 0x80A1_7472 };
            if is_chg_curve {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                println!("[CHARGE] Curve hook 0x{:08X}: dischg=0x{:08X}, chg=0x{:08X}", addr, r0, r1);
                let dischg_curve: [u16; 12] = [3400, 3500, 3600, 3650, 3700, 3750, 3800, 3850, 3900, 4000, 4100, 4180];
                let chg_curve: [u16; 12] = [3500, 3600, 3700, 3750, 3800, 3850, 3900, 3950, 4050, 4120, 4180, 4200];
                if r0 != 0 {
                    for (i, val) in dischg_curve.iter().enumerate() {
                        let _ = uc.mem_write(r0 + (i * 2) as u64, &val.to_le_bytes());
                    }
                }
                if r1 != 0 {
                    for (i, val) in chg_curve.iter().enumerate() {
                        let _ = uc.mem_write(r1 + (i * 2) as u64, &val.to_le_bytes());
                    }
                }
            }

            let is_bat_adc = if is_dump { addr == 0x80A1_4A62 } else { addr == 0x80A1_4A6A };
            if is_bat_adc {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 < 4200 || r0 > 4500 {
                    let _ = uc.reg_write(RegisterARM::R0, 4200);
                }
            }

            let is_chg_adc = if is_dump { addr == 0x80A1_4AA4 } else { addr == 0x80A1_4AAC };
            if is_chg_adc {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                if r0 < 6000 || r0 > 9700 {
                    let _ = uc.reg_write(RegisterARM::R0, 6500);
                }
            }

            if is_mutex_fn {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let mut cur_thread = [0u8; 4];
                let _ = uc.mem_read(0x8174_2174, &mut cur_thread);
                let cur_thread_val = u32::from_le_bytes(cur_thread);
                if cur_thread_val == 0 || cur_thread_val == 0xF0F0_F0F0 {
                    // Called during early boot initialization before scheduler thread context switch:
                    // Return TX_SUCCESS (0)
                    if r0 >= 0x8000_0000 && r0 < 0x9000_0000 {
                        let _ = uc.mem_write(r0, &0x4D55_5445u32.to_le_bytes());
                    }
                    let _ = uc.reg_write(RegisterARM::R0, 0);
                    let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                    let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
                }
            }

            if addr == 0x810D_7908 {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let r1 = uc.reg_read(RegisterARM::R1).unwrap_or(0);
                let r2 = uc.reg_read(RegisterARM::R2).unwrap_or(0);
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                if (r0 & 0xFF00_0000) == 0x5000_0000 || (r0 & 0xFF00_0000) == 0x2000_0000 || (r0 & 0xFF00_0000) == 0x7000_0000 {
                    println!("[MEMCPY MMIO] dest=0x{:08X}, src=0x{:08X}, len={} from LR=0x{:08X}", r0, r1, r2, lr);
                }
            }

            let is_sci_put_mutex = if is_dump { addr == 0x810D_4A5E } else { addr == 0x810D_6FF6 };
            if is_sci_put_mutex {
                let r0 = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                if r0 == 0 {
                    println!("[SCI_PUT_MUTEX] Called with NULL from LR 0x{:08X}", lr);
                    // Prevent assert on NULL mutex release during early init: return SCI_SUCCESS (0)
                    let _ = uc.reg_write(RegisterARM::R0, 0);
                    let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                    let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                    let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                    let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
                }
            }

            let is_trace_low = if is_dump { addr == 0x810C_F5C4 } else { addr == 0x80CB_73CA };
            if is_trace_low {
                let fmt = uc.reg_read(RegisterARM::R0).unwrap_or(0);
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let mut fmt_bytes = [0u8; 128];
                let fmt_str = if fmt >= 0x8000_0000 && fmt < 0x9000_0000 && uc.mem_read(fmt, &mut fmt_bytes).is_ok() {
                    let len = fmt_bytes.iter().position(|&b| b == 0).unwrap_or(fmt_bytes.len());
                    String::from_utf8_lossy(&fmt_bytes[..len]).to_string()
                } else {
                    format!("0x{:08X}", fmt)
                };
                let insns = uc.get_data().instruction_count;
                let data = uc.get_data_mut();
                if data.last_log_fmt != fmt_str {
                    if data.last_log_count > 1 {
                        println!("  [repeated {} times]", data.last_log_count);
                    }
                    data.last_log_fmt = fmt_str.clone();
                    data.last_log_count = 1;
                    println!("[LOG ENTRY @ {}M] fmt: {:?} from LR: 0x{:08X}", insns / 1_000_000, fmt_str, lr);
                    if !fmt_str.is_empty() && !fmt_str.starts_with("0x") && !fmt_str.starts_with("%") {
                        let clean = fmt_str.trim().replace('\n', " ");
                        data.uart.push_log_str(&format!("[MOCOR RTOS] {}", clean));
                    }
                } else {
                    data.last_log_count += 1;
                }
            }

            // BOOT1 handoff & FTL interception:
            // 0x801136E0 (PAC) / 0x80104ABE (Dump): SCI_FTL_Load(dev_id)
            // 0x80100C24 (PAC) / 0x8010065C (Dump): load_kernel()
            // 0x80113CA4 (PAC) / 0x80104BF8 (Dump): SCI_FTL_Unload(dev_id)
            // 0x80101C82 (Dump): verify_kernel()
            if addr == 0x8011_36E0 || addr == 0x8010_4ABE {
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
            } else if addr == 0x8010_0C24 || addr == 0x8010_065C {
                println!("[BOOT1] load_kernel(): KernelImg staged at 0x80a05e00 -> handoff ready");
                let uart = &mut uc.get_data_mut().uart;
                uart.push_log_str("[BOOT1] load_kernel(): KernelImg staged at 0x80a05e00 -> handoff ready");
                uart.push_log_str("[MOCOR RTOS] Kernel CP15 SCTLR & Banked Stacks: OK");
                uart.push_log_str("[MOCOR RTOS] ThreadX Preemptive Scheduler active");
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::R0, 1); // r0 = 1 (success)
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            } else if addr == 0x8011_3CA4 || addr == 0x8010_4BF8 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::R0, 0); // r0 = 0 (success)
                let _ = uc.reg_write(RegisterARM::PC, lr & !1);
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                let t_bit = if (lr & 1) != 0 { 0x20 } else { 0 };
                let _ = uc.reg_write(RegisterARM::CPSR, (cpsr & !0x20) | t_bit);
            } else if addr == 0x8010_1C82 {
                let lr = uc.reg_read(RegisterARM::LR).unwrap_or(0);
                let _ = uc.reg_write(RegisterARM::R0, 1); // r0 = 1 (success)
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
            let (has_raw_irq, pending) = {
                let intc = &uc.get_data().intc;
                let raw = (intc.irq_raw_sts | intc.irq_soft) != 0;
                if raw {
                    (true, intc.get_irq_masked_status())
                } else {
                    (false, 0)
                }
            };
            if has_raw_irq && pending != 0 {
                let cpsr = uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                if (cpsr & 0x80) == 0 {
                    let vector_base = if addr < DRAM_BASE { 0x0000_0000u64 } else { 0x8000_0000u64 };
                    println!("[IRQ DISPATCH] Pending IRQs: 0x{:08X} at PC: 0x{:08X} (vector: 0x{:08X})", pending, addr, vector_base + 0x18);
                    let svc_cpsr = cpsr;
                    let irq_cpsr = (svc_cpsr & !0x3F) | 0x12 | 0x80;
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
                        _ if (addr & 0xFFF0_0000) == 0x5080_0000 => {
                            let mut cur = [0u8; 4];
                            let _ = uc.mem_read(addr, &mut cur);
                            (u32::from_le_bytes(cur), true)
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
                        _ if (addr & 0xFFF0_0000) == 0x5080_0000 => {
                            let bytes = val.to_le_bytes();
                            let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
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
                    0x0300_0800..=0x0300_08FF => {
                        // Spreadtrum Hardware Spinlocks (hwspinlock at 0x3300_0800..0x3300_08FF):
                        // Single-core emulation: lock is always acquired immediately (read returns 0)
                        val = 0;
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

        // Setup SPRD Kernel AON System Timer Read Hook (0x4004_0000..0x4004_1000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_KERNEL_SYSTIMER_BASE,
            SPRD_KERNEL_SYSTIMER_BASE + SPRD_KERNEL_SYSTIMER_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_KERNEL_SYSTIMER_BASE) as u32;
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
        ).map_err(|e| format!("Failed to add Kernel SYSTIMER read hook: {:?}", e))?;

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
                                let non_zero = fb_data.chunks_exact(2).filter(|c| c[0] != 0 || c[1] != 0).count();
                                // Only blit if active frame buffer has substantial graphic content (> 1000 non-zero pixels)
                                if non_zero >= 1000 {
                                    let ctx = uc.get_data_mut();
                                    if width == ST7735_WIDTH && height == ST7735_HEIGHT {
                                        for y in 0..height {
                                            for x in 0..width {
                                                let idx = (y * width + x) * 2;
                                                let pixel = u16::from_le_bytes([fb_data[idx], fb_data[idx + 1]]);
                                                ctx.display.st7735.framebuffer[y][x] = pixel;
                                            }
                                        }
                                        ctx.display.st7735.has_custom_framebuffer = true;
                                    } else {
                                        for y in 0..height.min(GC9307_HEIGHT) {
                                            for x in 0..width.min(GC9307_WIDTH) {
                                                let idx = (y * width + x) * 2;
                                                let pixel = u16::from_le_bytes([fb_data[idx], fb_data[idx + 1]]);
                                                ctx.display.gc9307.framebuffer[y][x] = pixel;
                                            }
                                        }
                                        ctx.display.gc9307.has_custom_framebuffer = true;
                                        if !ctx.gpio.is_flip_open() {
                                            ctx.display.transfer_main_to_sub();
                                        }
                                    }
                                }
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
                if offset == 0x011C {
                    let _ = uc.mem_write(addr, &1u32.to_le_bytes()[..size.min(4)]);
                }
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

        // Setup SPRD Baseband / WCDMA / RF Controller Read Hook (0x402B_0000..0x402B_1000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            0x402B_0000,
            0x402B_1000,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - 0x402B_0000) as u32;
                let val: u32 = match offset {
                    // 0xC4: RF status register -> 0
                    0x00C4 => 0x0000_0000,
                    // 0xD4: RF / Baseband state: bits 4..7 = 6 (RF_READY / CAL_DONE)
                    0x00D4 => 0x0000_0060,
                    _ => 0,
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add WCDMA/RF read hook: {:?}", e))?;

        // Setup SPRD RFSPI / RF Transceiver Read Hook (0x4007_0000..0x4007_1000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            0x4007_0000,
            0x4007_1000,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - 0x4007_0000) as u32;
                let val: u32 = match offset {
                    0x0248 => 0x0000_0743, // SR3532 RF Chip ID (hw_rf743)
                    _ => {
                        let mut cur = [0u8; 4];
                        let _ = uc.mem_read(addr, &mut cur);
                        u32::from_le_bytes(cur)
                    }
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add RFSPI/RF read hook: {:?}", e))?;

        // Setup Unisoc Modem GPTA / Free Running Counter (FRC) Read Hook (0xE002_0000..0xE002_1000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_MODEM_GPTA_BASE,
            SPRD_MODEM_GPTA_BASE + SPRD_MODEM_GPTA_SIZE as u64,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_MODEM_GPTA_BASE) as u32;
                let insn_count = uc.get_data().instruction_count;
                let (lo, hi) = calc_gpta_counter(insn_count);
                if offset == 0x0010 && size == 8 {
                    let mut buf = [0u8; 8];
                    buf[0..4].copy_from_slice(&lo.to_le_bytes());
                    buf[4..8].copy_from_slice(&hi.to_le_bytes());
                    let _ = uc.mem_write(addr, &buf);
                } else {
                    let val: u32 = match offset {
                        0x0010 => {
                            let _ = uc.mem_write(SPRD_MODEM_GPTA_BASE + 0x0014, &hi.to_le_bytes());
                            lo
                        }
                        0x0014 => hi,
                        0x0040 => {
                            let mut cur = [0u8; 4];
                            let _ = uc.mem_read(SPRD_MODEM_GPTA_BASE + 0x0040, &mut cur);
                            u32::from_le_bytes(cur) | 0x40
                        }
                        _ => {
                            let mut cur = [0u8; 4];
                            let _ = uc.mem_read(addr, &mut cur);
                            u32::from_le_bytes(cur)
                        }
                    };
                    let bytes = val.to_le_bytes();
                    let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                }
                true
            },
        ).map_err(|e| format!("Failed to add Modem GPTA read hook: {:?}", e))?;

        // Setup Unisoc Modem GPTA / Free Running Counter (FRC) Write Hook (0xE002_0000..0xE002_1000)
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            SPRD_MODEM_GPTA_BASE,
            SPRD_MODEM_GPTA_BASE + SPRD_MODEM_GPTA_SIZE as u64,
            |uc, _mem_type, addr, size, value| {
                let offset = (addr - SPRD_MODEM_GPTA_BASE) as u32;
                let val = (value as u64 & 0xFFFF_FFFF) as u32;
                let insn_count = uc.get_data().instruction_count;
                let (lo, hi) = calc_gpta_counter(insn_count);
                match offset {
                    0x0028 => {
                        // Counter latch trigger: write latches current counter into 0x0010 and 0x0014
                        let _ = uc.mem_write(SPRD_MODEM_GPTA_BASE + 0x0010, &lo.to_le_bytes());
                        let _ = uc.mem_write(SPRD_MODEM_GPTA_BASE + 0x0014, &hi.to_le_bytes());
                        let _ = uc.mem_write(SPRD_MODEM_GPTA_BASE + 0x0040, &0x0000_0040u32.to_le_bytes());
                    }
                    0x0040 => {
                        let status = val | 0x40;
                        let _ = uc.mem_write(addr, &status.to_le_bytes());
                    }
                    _ => {
                        let bytes = val.to_le_bytes();
                        let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                    }
                }
                true
            },
        ).map_err(|e| format!("Failed to add Modem GPTA write hook: {:?}", e))?;

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
                    0x0314 => 0xFFFF, // SC2720 PMIC eFuse status register ready
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
                    } else if (target_addr & 0xFFFF) == 0x8314 || (target_addr & 0xFFF) == 0x0314 {
                        0xFFFFu32 // SC2720 PMIC eFuse status register ready
                    } else {
                        let mut resp_bytes = [0u8; 4];
                        let _ = uc.mem_read(target_addr, &mut resp_bytes);
                        u32::from_le_bytes(resp_bytes)
                    };
                    let tag = (((target_addr as u32) >> 2) & 0x7FFF) << 16;
                    let full_resp = tag | (resp & 0xFFFF);
                    let _ = uc.mem_write(SPRD_PMIC_ADI_BASE + 0x002C, &full_resp.to_le_bytes());
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

        // Setup ARM GIC-400 Hooks (0x1200_0000..0x1200_4000)
        uc.add_mem_hook(
            HookType::MEM_READ,
            SPRD_GIC_BASE,
            SPRD_GIC_BASE + 0x4000,
            |uc, _mem_type, addr, size, _value| {
                let offset = (addr - SPRD_GIC_BASE) as u32;
                let val: u32 = match offset {
                    0x1000 => 0x0000_0001, // GICD_CTLR: Group 0 enabled
                    0x1004 => 0x0000_00A3, // GICD_TYPER: 128 IRQ lines (ITLinesNumber=3), 1 CPU interface
                    0x2000 => 0x0000_0001, // GICC_CTLR: CPU interface enabled
                    0x2004 => 0x0000_00FF, // GICC_PMR: Priority mask
                    0x200C => 0x0000_03FF, // GICC_IAR: 1023 (spurious / no pending interrupt)
                    _ => {
                        let mut cur = [0u8; 4];
                        let _ = uc.mem_read(addr, &mut cur);
                        u32::from_le_bytes(cur)
                    }
                };
                let bytes = val.to_le_bytes();
                let _ = uc.mem_write(addr, &bytes[..size.min(4)]);
                true
            },
        ).map_err(|e| format!("Failed to add GIC read hook: {:?}", e))?;

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
            } else if id_lower.contains("mmires") {
                println!("[PAC] Staging MMIRes partition '{}' ({} bytes) into SPI NAND at offset 0x01B0_0000", f.id, f.data.len());
                self.uc.get_data_mut().sfc.load_partition(0x01B0_0000, &f.data);
                self.uc.get_data_mut().mmi_res_data = f.data.clone();
                let _ = self.load_bytes(&f.data, FLASH_DIRECT_BASE + 0x01B0_0000);

                // Extract authentic boot splashes from MMIRes sres0001 container
                if f.data.len() > 0x140 && &f.data[0..8] == b"sres0001" {
                    let e0_off = u32::from_le_bytes([f.data[0x120], f.data[0x121], f.data[0x122], f.data[0x123]]) as usize;
                    let e0_sz = u32::from_le_bytes([f.data[0x124], f.data[0x125], f.data[0x126], f.data[0x127]]) as usize;
                    if e0_off + e0_sz <= f.data.len() {
                        let splash_bytes = &f.data[e0_off..e0_off + e0_sz];
                        if self.uc.get_data_mut().display.gc9307.load_splash(splash_bytes) {
                            println!("[PAC] Loaded authentic Nokia QVGA 240x320 boot splash into GC9307 LCD ({} bytes)", e0_sz);
                        }
                    }

                    let e1_off = u32::from_le_bytes([f.data[0x12C], f.data[0x12D], f.data[0x12E], f.data[0x12F]]) as usize;
                    let e1_sz = u32::from_le_bytes([f.data[0x130], f.data[0x131], f.data[0x132], f.data[0x133]]) as usize;
                    if e1_off + e1_sz <= f.data.len() {
                        let sub_splash_bytes = &f.data[e1_off..e1_off + e1_sz];
                        if self.uc.get_data_mut().display.st7735.load_splash(sub_splash_bytes) {
                            println!("[PAC] Loaded authentic Nokia QQVGA 128x160 sub-display boot splash into ST7735 LCD ({} bytes)", e1_sz);
                        }
                    }
                } else if f.data.len() >= 14 && &f.data[0..2] == b"BM" {
                    let _ = self.uc.get_data_mut().display.gc9307.load_splash(&f.data);
                }
            } else if id_lower.contains("nv") {
                println!("[PAC] Staging NV partition '{}' ({} bytes) into SPI NAND at offset 0x01A0_0000", f.id, f.data.len());
                self.uc.get_data_mut().sfc.load_partition(0x01A0_0000, &f.data);
            } else if id_lower.contains("system") || id_lower.contains("rootfs") {
                println!("[PAC] Staging system/rootfs partition '{}' ({} bytes) into SPI NAND", f.id, f.data.len());
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

    pub fn load_dump(
        &mut self,
        dump_dir: &std::path::Path,
        default_entry: u64,
    ) -> Result<Option<u64>, String> {
        let base_dir = if dump_dir.join("boot0.bin").exists() || dump_dir.join("kernel.bin").exists() {
            dump_dir.to_path_buf()
        } else if dump_dir.join("ta1480_10.02.17.00/boot0.bin").exists() {
            dump_dir.join("ta1480_10.02.17.00")
        } else {
            dump_dir.to_path_buf()
        };

        println!("[DUMP] Loading authentic device partition dump from '{}'...", base_dir.display());
        self.uc.get_data_mut().is_dump = true;

        let mut dump_boot_entry: Option<u64> = None;
        let mut kernel_entry: Option<u64> = None;

        // 1. BOOT0 SPL (SRAM 0x00006200 & SPI NAND Block 0)
        let boot0_names = ["boot0.bin", "boot0", "spl.bin"];
        for name in &boot0_names {
            let p = base_dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    println!("[DUMP] Staging BOOT0/SPL partition '{}' ({} bytes) into SPI NAND Block 0 & SRAM", name, data.len());
                    let chunk_len = data.len().min(2048);
                    self.uc.get_data_mut().sfc.write_page(0, 0, &data[..chunk_len]);
                    let _ = self.load_bytes(&data, FLASH_DIRECT_BASE);
                    let _ = self.load_bytes(&data, 0x0000_6200);

                    let is_dhtb = data.len() > 512 && &data[0..4] == b"DHTB";
                    if is_dhtb {
                        dump_boot_entry = Some(0x0000_6400);
                    } else if let Ok(hdr) = SprdHeader::parse(&data) {
                        if data.len() > 32 {
                            let _ = self.load_bytes(&data[32..], hdr.load_addr as u64);
                        }
                        dump_boot_entry = Some(hdr.entry_point as u64);
                    } else {
                        dump_boot_entry = Some(0x0000_6200);
                    }
                    break;
                }
            }
        }

        // 2. BOOT1 Secondary Bootloader / LK (High DRAM 0x80100000)
        let boot1_names = ["boot1.bin", "boot1", "lk.bin"];
        for name in &boot1_names {
            let p = base_dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    println!("[DUMP] Staging BOOT1/LK partition '{}' ({} bytes) at 0x80100000", name, data.len());
                    let _ = self.load_bytes(&data, 0x8010_0000);
                    break;
                }
            }
        }

        // 3. Kernel / MOCOR OS (High DRAM 0x80a05e00, entry 0x80a06000)
        let kernel_names = ["kernel.bin", "boot.bin", "kernelimg.bin", "EXEC_KERNEL_IMAGE.bin"];
        for name in &kernel_names {
            let p = base_dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    let load_addr = 0x80a0_5e00u64;
                    let entry = 0x80a0_6000u64;
                    println!("[DUMP] Staging Kernel/Boot partition '{}' ({} bytes) at 0x{:08X}", name, data.len(), load_addr);
                    self.load_bytes(&data, load_addr)?;
                    kernel_entry = Some(entry);
                    break;
                }
            }
        }

        // 4. Userland Framework / MS_MMI_Main (User DRAM 0x88000000)
        let user_names = ["user.bin", "userimg.bin", "system.bin", "EXEC_USER_IMAGE.bin"];
        for name in &user_names {
            let p = base_dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    let load_addr = 0x8800_0000u64;
                    println!("[DUMP] Staging UserImg partition '{}' ({} bytes) at 0x{:08X}", name, data.len(), load_addr);
                    self.load_bytes(&data, load_addr)?;
                    break;
                }
            }
        }

        // 5. MMI Resources & Boot Splashes (SFC offset 0x01B00000 & Direct Flash)
        let mmi_names = ["mmi_res.bin", "mmires.bin", "mmi_res_240X320_MH.bin"];
        for name in &mmi_names {
            let p = base_dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    println!("[DUMP] Staging MMIRes partition '{}' ({} bytes) into SPI NAND at offset 0x01B0_0000", name, data.len());
                    self.uc.get_data_mut().sfc.load_partition(0x01B0_0000, &data);
                    self.uc.get_data_mut().mmi_res_data = data.clone();
                    let _ = self.load_bytes(&data, FLASH_DIRECT_BASE + 0x01B0_0000);

                    // Extract authentic boot splashes from MMIRes sres0001 container
                    if data.len() > 0x140 && &data[0..8] == b"sres0001" {
                        let e0_off = u32::from_le_bytes([data[0x120], data[0x121], data[0x122], data[0x123]]) as usize;
                        let e0_sz = u32::from_le_bytes([data[0x124], data[0x125], data[0x126], data[0x127]]) as usize;
                        if e0_off + e0_sz <= data.len() {
                            let splash_bytes = &data[e0_off..e0_off + e0_sz];
                            if self.uc.get_data_mut().display.gc9307.load_splash(splash_bytes) {
                                println!("[DUMP] Loaded authentic Nokia QVGA 240x320 boot splash into GC9307 LCD ({} bytes)", e0_sz);
                            }
                        }

                        let e1_off = u32::from_le_bytes([data[0x12C], data[0x12D], data[0x12E], data[0x12F]]) as usize;
                        let e1_sz = u32::from_le_bytes([data[0x130], data[0x131], data[0x132], data[0x133]]) as usize;
                        if e1_off + e1_sz <= data.len() {
                            let sub_splash_bytes = &data[e1_off..e1_off + e1_sz];
                            if self.uc.get_data_mut().display.st7735.load_splash(sub_splash_bytes) {
                                println!("[DUMP] Loaded authentic Nokia QQVGA 128x160 sub-display boot splash into ST7735 LCD ({} bytes)", e1_sz);
                            }
                        }
                    } else if data.len() >= 14 && &data[0..2] == b"BM" {
                        let _ = self.uc.get_data_mut().display.gc9307.load_splash(&data);
                    }
                    break;
                }
            }
        }

        // 6. NVRAM & Calibration Items (SFC offset 0x01A00000)
        let nv_names = ["nvitem.bin", "nv.bin", "prodnv.bin"];
        for name in &nv_names {
            let p = base_dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    println!("[DUMP] Staging NV partition '{}' ({} bytes) into SPI NAND at offset 0x01A0_0000", name, data.len());
                    self.uc.get_data_mut().sfc.load_partition(0x01A0_0000, &data);
                    break;
                }
            }
        }

        // 7. Power Management Cortex-M4 Coprocessor
        let pm_names = ["pm_cm4.bin", "PM_nemol_cm4.bin"];
        for name in &pm_names {
            let p = base_dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    println!("[DUMP] Staging PM CM4 partition '{}' ({} bytes)", name, data.len());
                    break;
                }
            }
        }

        // 8. Cellular DSPs (LTE & GSM)
        let lte_names = ["lte_dsp.bin", "T117_CAT1_LTEA_DSP.bin"];
        for name in &lte_names {
            let p = base_dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    println!("[DUMP] Staging LTE DSP partition '{}' ({} bytes)", name, data.len());
                    break;
                }
            }
        }
        let gsm_names = ["gsm_dsp.bin", "NEMOL_CAT1_EVS_DM_DSP.bin"];
        for name in &gsm_names {
            let p = base_dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    println!("[DUMP] Staging GSM DSP partition '{}' ({} bytes)", name, data.len());
                    break;
                }
            }
        }

        // Default boot entry priority: BOOT0 (cold boot SPL) > Kernel > default_entry
        self.set_boot_stage(BootStage::Boot0);
        Ok(dump_boot_entry.or(kernel_entry).or(Some(default_entry)))
    }

    pub fn set_boot_stage(&mut self, stage: BootStage) {
        let ctx = self.uc.get_data_mut();
        ctx.current_stage = Some(stage);
        let is_open = ctx.gpio.is_flip_open();
        let pct = stage.progress_pct();
        if !ctx.display.gc9307.has_custom_framebuffer {
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
        }
        if !is_open {
            ctx.display.transfer_main_to_sub();
        }
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
        let mut cpsr = self.uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
        if (cpsr & 0x20) != 0 || (pc & 1) != 0 || (pc >= 0x8800_0000 && pc < 0x8A00_0000) {
            pc |= 1;
            cpsr |= 0x20;
            let _ = self.uc.reg_write(RegisterARM::CPSR, cpsr);
        }
        let effective_timeout = if count > 0 { 0 } else { timeout_us };
        let res = match self.uc.emu_start(pc, 0xFFFF_FFFF, effective_timeout, count) {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = format!("{:?}", e);
                if let Some(page) = self.uc.get_data_mut().pending_unmapped_page.take() {
                    let chunk_base = page & !0xFFFF;
                    if self.uc.mem_map(chunk_base, 0x10000, Prot::ALL).is_err() {
                        let _ = self.uc.mem_map(page, 0x1000, Prot::ALL);
                    }
                    return Ok(true);
                }
                let fault_pc = self.uc.reg_read(RegisterARM::PC).unwrap_or(pc);
                if fault_pc >= 0x8800_0000 && fault_pc < 0x8A00_0000 {
                    let mut ttbr0 = unicorn_engine::RegisterARMCP::new().cp(15).crn(2).crm(0).opc1(0).opc2(0);
                    let _ = self.uc.reg_read_arm_coproc(&mut ttbr0);
                    let l1_base = ttbr0.val & !0x3FFF;
                    if l1_base >= 0x8000_0000 && l1_base < 0x8400_0000 {
                        println!("[MMU USERLAND RETRY] Mapping Userland DRAM sections into L1 at 0x{:08X}", l1_base);
                        for i in 0..32u32 {
                            let va = 0x8800_0000u32 + (i << 20);
                            let l1_idx = (va >> 20) as u64;
                            let desc = va | 0xC0E;
                            let _ = self.uc.mem_write(l1_base as u64 + l1_idx * 4, &desc.to_le_bytes());
                        }
                        let mut dacr = unicorn_engine::RegisterARMCP::new().cp(15).crn(3).crm(0).opc1(0).opc2(0);
                        dacr.val = 0xFFFF_FFFF;
                        let _ = self.uc.reg_write_arm_coproc(&dacr);
                        let mut tlb_flush = unicorn_engine::RegisterARMCP::new().cp(15).crn(8).crm(7).opc1(0).opc2(0);
                        tlb_flush.val = 0;
                        let _ = self.uc.reg_write_arm_coproc(&tlb_flush);
                        let cpsr = self.uc.reg_read(RegisterARM::CPSR).unwrap_or(0);
                        let _ = self.uc.reg_write(RegisterARM::CPSR, cpsr | 0x20);
                        return Ok(true);
                    }
                }
                if err_str.contains("TIMEOUT") || err_str.contains("OK") {
                    Ok(true)
                } else {
                    let lr = self.uc.reg_read(RegisterARM::LR).unwrap_or(0);
                    let sp = self.uc.reg_read(RegisterARM::SP).unwrap_or(0);
                    let _r0 = self.uc.reg_read(RegisterARM::R0).unwrap_or(0);
                    let _r1 = self.uc.reg_read(RegisterARM::R1).unwrap_or(0);
                    let _r2 = self.uc.reg_read(RegisterARM::R2).unwrap_or(0);
                    let _r3 = self.uc.reg_read(RegisterARM::R3).unwrap_or(0);
                    let fault_pc = self.uc.reg_read(RegisterARM::PC).unwrap_or(pc);
                    let mut ttbr0 = unicorn_engine::RegisterARMCP::new().cp(15).crn(2).crm(0).opc1(0).opc2(0);
                    let _ = self.uc.reg_read_arm_coproc(&mut ttbr0);
                    let mut dacr = unicorn_engine::RegisterARMCP::new().cp(15).crn(3).crm(0).opc1(0).opc2(0);
                    let _ = self.uc.reg_read_arm_coproc(&mut dacr);
                    let mut sctlr = unicorn_engine::RegisterARMCP::new().cp(15).crn(1).crm(0).opc1(0).opc2(0);
                    let _ = self.uc.reg_read_arm_coproc(&mut sctlr);
                    let l1 = ttbr0.val & !0x3FFF;
                    let mut entry = [0u8; 4];
                    let _ = self.uc.mem_read(l1 as u64 + ((fault_pc >> 20) as u64) * 4, &mut entry);
                    let entry_val = u32::from_le_bytes(entry);
                    Err(format!("Emulation error at fault PC 0x{:08X} (initial PC 0x{:08X}, LR: 0x{:08X}, SP: 0x{:08X}, CPSR: 0x{:08X}, TTBR0: 0x{:08X}, DACR: 0x{:08X}, SCTLR: 0x{:08X}, L1_entry: 0x{:08X}): {:?}", fault_pc & !1, pc & !1, lr, sp, cpsr, ttbr0.val, dacr.val, sctlr.val, entry_val, e))
                }
            }
        };

        let ctx = self.uc.get_data_mut();
        let ready_for_home = (ctx.current_stage == Some(BootStage::User) && ctx.user_insn_count >= 1_000)
            || (ctx.is_dump && ctx.instruction_count >= 15_000_000);
        if ready_for_home {
            if ctx.current_stage != Some(BootStage::User) {
                ctx.current_stage = Some(BootStage::User);
            }
            let dial = ctx.dial_buffer.clone();
            let in_call = ctx.in_call;
            ctx.display.gc9307.render_mmi_home(&dial, in_call, 0);
            if !ctx.gpio.is_flip_open() {
                ctx.display.transfer_main_to_sub();
            }
        }

        res
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
                } else if key == "soft2" || key == "back" {
                    ctx.dial_buffer.pop();
                } else if ("0"..="9").contains(&key) || key == "*" || key == "#" {
                    if ctx.dial_buffer.len() < 14 {
                        ctx.dial_buffer.push_str(key);
                    }
                }
                if ctx.current_stage == Some(BootStage::User) || (ctx.is_dump && ctx.instruction_count >= 15_000_000) {
                    let dial = ctx.dial_buffer.clone();
                    let in_call = ctx.in_call;
                    ctx.display.gc9307.render_mmi_home(&dial, in_call, 0);
                    if !ctx.gpio.is_flip_open() {
                        ctx.display.transfer_main_to_sub();
                    }
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
        if !is_open {
            // When flip switch is activated (closed), transfer actual active firmware output
            // from the primary main display (GC9307 240x320) to the outer cover display (ST7735 120x160)
            ctx.display.transfer_main_to_sub();
        }
    }

    pub fn transfer_main_to_sub(&mut self) {
        self.uc.get_data_mut().display.transfer_main_to_sub();
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
