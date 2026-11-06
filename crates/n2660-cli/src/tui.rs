// SPDX-License-Identifier: MIT
//! Interactive ANSI Truecolor Terminal GUI for Nokia 2660 Flip

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use n2660_core::types::{GC9307_HEIGHT, GC9307_WIDTH};
use n2660_core::vm::T107Vm;
use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};

pub fn run_interactive_tui(vm: &mut T107Vm) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, cursor::Hide)?;

    let mut flip_open = true;
    let mut paused = false;
    let mut last_key_pressed: Option<&'static str> = None;
    let mut active_key_desc = "None";
    let start_time = Instant::now();

    loop {
        // 1. Process terminal keyboard input
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let mut key_str: Option<&'static str> = None;
                    let mut desc = "None";

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            terminal::disable_raw_mode()?;
                            execute!(out, LeaveAlternateScreen, cursor::Show)?;
                            return Ok(());
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            paused = !paused;
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') => {
                            flip_open = !flip_open;
                            vm.set_flip_state(flip_open);
                        }
                        KeyCode::Char('0') => { key_str = Some("0"); desc = "0"; }
                        KeyCode::Char('1') => { key_str = Some("1"); desc = "1"; }
                        KeyCode::Char('2') => { key_str = Some("2"); desc = "2"; }
                        KeyCode::Char('3') => { key_str = Some("3"); desc = "3"; }
                        KeyCode::Char('4') => { key_str = Some("4"); desc = "4"; }
                        KeyCode::Char('5') => { key_str = Some("5"); desc = "5"; }
                        KeyCode::Char('6') => { key_str = Some("6"); desc = "6"; }
                        KeyCode::Char('7') => { key_str = Some("7"); desc = "7"; }
                        KeyCode::Char('8') => { key_str = Some("8"); desc = "8"; }
                        KeyCode::Char('9') => { key_str = Some("9"); desc = "9"; }
                        KeyCode::Char('*') => { key_str = Some("star"); desc = "*"; }
                        KeyCode::Char('#') => { key_str = Some("hash"); desc = "#"; }
                        KeyCode::Char('c') | KeyCode::Char('C') => { key_str = Some("call"); desc = "CALL"; }
                        KeyCode::Char('x') | KeyCode::Char('X') => { key_str = Some("end"); desc = "END"; }
                        KeyCode::Char('[') => { key_str = Some("soft1"); desc = "SOFT1 (Menu)"; }
                        KeyCode::Char(']') => { key_str = Some("soft2"); desc = "SOFT2 (Back)"; }
                        KeyCode::Enter => { key_str = Some("ok"); desc = "OK"; }
                        KeyCode::Up => { key_str = Some("up"); desc = "UP"; }
                        KeyCode::Down => { key_str = Some("down"); desc = "DOWN"; }
                        KeyCode::Left => { key_str = Some("left"); desc = "LEFT"; }
                        KeyCode::Right => { key_str = Some("right"); desc = "RIGHT"; }
                        _ => {}
                    }

                    if let Some(k) = key_str {
                        if let Some(prev) = last_key_pressed {
                            vm.release_key(prev);
                        }
                        vm.press_key(k);
                        last_key_pressed = Some(k);
                        active_key_desc = desc;
                    }
                } else if key.kind == KeyEventKind::Release {
                    if let Some(prev) = last_key_pressed.take() {
                        vm.release_key(prev);
                        active_key_desc = "None";
                    }
                }
            }
        }

        // 2. Step VM execution
        if !paused {
            let _ = vm.step(50_000, 25_000);
        }

        // 3. Render ANSI Screen
        execute!(out, cursor::MoveTo(0, 0))?;
        let pc = vm.get_pc();
        let sp = vm.get_sp();
        let cpsr = vm.get_cpsr();
        let total_insns = vm.get_instruction_count();
        let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
        let mips = (total_insns as f64 / elapsed) / 1_000_000.0;

        let flip_str = if flip_open { "\x1b[32mOPEN\x1b[0m" } else { "\x1b[33mCLOSED\x1b[0m" };
        let state_str = if paused { "\x1b[31mPAUSED\x1b[0m" } else { "\x1b[32mRUNNING\x1b[0m" };

        writeln!(out, "\x1b[1;36m=== Nokia 2660 Flip Terminal Emulator (Unisoc T107) ===\x1b[0m")?;
        writeln!(out, "State: {} | Flip: {} | Key: \x1b[1m{}\x1b[0m | MIPS: {:.2}", state_str, flip_str, active_key_desc, mips)?;
        writeln!(out, "CPU: PC=0x{:08X} | SP=0x{:08X} | CPSR=0x{:08X} | Insns: {}", pc, sp, cpsr, total_insns)?;
        writeln!(out, "{:-<64}", "")?;

        // Downsample 240x320 display to 60 columns x 20 lines (each character cell = 2 vertical pixels via ▀)
        let down_w = 60usize;
        let down_h = 40usize;
        let step_x = GC9307_WIDTH / down_w;
        let step_y = GC9307_HEIGHT / down_h;
        let fb = vm.get_gc9307_framebuffer();

        for y in (0..down_h).step_by(2) {
            let mut line = String::with_capacity(down_w * 24);
            line.push_str("  |");
            for x in 0..down_w {
                let px_x = x * step_x;
                let py1 = y * step_y;
                let py2 = (y + 1) * step_y;

                let c1 = rgb565_to_rgb888(fb[py1][px_x]);
                let c2 = rgb565_to_rgb888(fb[py2][px_x]);

                line.push_str(&format!(
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                    c1.0, c1.1, c1.2, c2.0, c2.1, c2.2
                ));
            }
            line.push_str("\x1b[0m|");
            writeln!(out, "{}", line)?;
        }

        writeln!(out, "{:-<64}", "")?;
        writeln!(out, "Keypad: [[]Menu  []]Back  [c]Call  [x]End  [f]Flip  [p]Pause  [q]Quit")?;
        writeln!(out, "        [0-9, *, #] Numbers | Arrows: Up/Down/Left/Right | Enter: OK")?;
        out.flush()?;

        std::thread::sleep(Duration::from_millis(16));
    }
}

fn rgb565_to_rgb888(c: u16) -> (u8, u8, u8) {
    let r = (((c >> 11) & 0x1F) * 255 / 31) as u8;
    let g = (((c >> 5) & 0x3F) * 255 / 63) as u8;
    let b = ((c & 0x1F) * 255 / 31) as u8;
    (r, g, b)
}
