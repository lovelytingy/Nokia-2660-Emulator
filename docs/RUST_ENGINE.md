# N2660Emu: High-Performance Rust Emulation Core

Copyright (c) 2026 N2660Emu Developers  
SPDX-License-Identifier: MIT

---

## 1. Overview

To overcome the performance bottlenecks inherent in Python's interpreted runtime, GIL (Global Interpreter Lock), and C-API call overhead during per-instruction and MMIO memory accesses, the core execution engine of N2660Emu has been migrated to **Rust**.

The Rust implementation provides:
- **Zero-Cost FFI**: Direct, inlined `extern "C"` MMIO callbacks for peripheral memory transactions without Python object boxing or GIL locking.
- **Deterministic Hardware Modeling**: Strict compile-time safety and bit-exact memory layout for all Unisoc T107 peripherals.
- **Order-of-Magnitude Performance Gains**: Sustains **16+ MIPS** in interactive execution and completes full 720K+ instruction baremetal test runs in under **80 milliseconds** (compared to 1–3 seconds in Python).

---

## 2. Workspace Architecture

The Rust codebase is organized as a Cargo workspace with three specialized crates:

```
Nokia-2660-Emulator/
├── Cargo.toml                  # Workspace manifest
├── crates/
│   ├── n2660-core/             # Hardware simulation library
│   │   ├── src/
│   │   │   ├── types.rs        # Memory map constants, addresses, key codes
│   │   │   ├── vm.rs           # T107Vm & Unicorn ARMv7-A execution core
│   │   │   ├── peripherals/    # Unisoc SPRD IP blocks
│   │   │   │   ├── uart.rs     # SPRD UART0 (TX/RX FIFOs, baud, status)
│   │   │   │   ├── timer.rs    # SPRD System Timer (Downcounter, load, int)
│   │   │   │   ├── gpio.rs     # SPRD GPIO & Hall effect flip sensor
│   │   │   │   ├── intc.rs     # 32-Line Interrupt Controller & IRQ vector dispatch
│   │   │   │   ├── kpd.rs      # Keypad Matrix Controller (STATUS0/STATUS1)
│   │   │   │   ├── spi_lcd.rs  # GC9307 240x320 & ST7735 120x160 framebuffers
│   │   │   │   ├── sfc.rs      # 128 MB SPI NAND (Winbond W25N01GV) & XIP
│   │   │   │   └── audio.rs    # Audio DAC & hardware tone buzzer generator
│   │   │   ├── bsl.rs          # Spreadtrum BSL HDLC & CRC-16 protocol engine
│   │   │   ├── atags.rs        # ARM Boot Tags synthesizer & DRAM handoff
│   │   │   ├── bootrom.rs      # SPRD BOOT0 header parser & validator
│   │   │   └── pac.rs          # Unisoc .PAC container parser & unpacker
│   │   └── tests/
│   │       └── test_vm_baremetal.rs # Full end-to-end baremetal test verification
│   ├── n2660-cli/              # Native CLI binary (`n2660emu-rs`)
│   │   └── src/main.rs         # High-performance CLI orchestrator
│   └── n2660-tools/            # Image packaging utilities
```

---

## 3. Performance Benchmark

| Metric | Python Engine (`tools/unicorn_runner.py`) | Rust Engine (`target/release/n2660emu-rs`) | Speedup |
| :--- | :--- | :--- | :--- |
| **Baremetal Test Run (724,186 insns)** | 1.85 s | **0.081 s** | **~23× Faster** |
| **Interactive OS Shell Execution** | 2.1 MIPS | **16.7 MIPS** | **~8× Faster** |
| **MMIO Read/Write Intercept Overhead** | ~400–600 ns / op | **< 15 ns / op** | **~30× Faster** |
| **Startup / JIT Latency** | ~280 ms | **< 4 ms** | **~70× Faster** |

---

## 4. Building and Running

### Build the Rust Engine
```bash
make rust
# or via cargo directly:
cargo build --release
```
The compiled native executable is placed at `target/release/n2660emu-rs`.

### Execute Firmware via Native Rust Binary
```bash
# Execute baremetal test payload:
./target/release/n2660emu-rs tests/baremetal/test_firmware.bin

# Run with instruction tracing:
./target/release/n2660emu-rs tests/baremetal/test_firmware.bin --trace

# Dump primary and secondary display framebuffers:
./target/release/n2660emu-rs tests/baremetal/test_firmware.bin \
    --screen-dump build/screen.ppm \
    --sub-screen-dump build/sub_screen.ppm
```

### Seamless CLI Integration
The top-level launcher `./bin/n2660emu` automatically detects and delegates to `target/release/n2660emu-rs` for all emulation commands:
```bash
make run
# or:
./bin/n2660emu tests/baremetal/test_firmware.bin
```

### Running Tests
Execute the full native Rust test suite across all workspace crates and peripheral models:
```bash
cargo test --workspace
# or:
make test
```
