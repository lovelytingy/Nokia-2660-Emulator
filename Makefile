# Root Makefile for Nokia 2660 Emulator (N2660Emu)
# SPDX-License-Identifier: MIT

CARGO ?= cargo
RUST_BIN = target/release/n2660emu-rs

all: firmware bootrom rust

firmware:
	@if command -v arm-none-eabi-gcc >/dev/null 2>&1; then \
		$(MAKE) -C tests/baremetal; \
	elif [ -f tests/baremetal/test_firmware.bin ]; then \
		echo "[N2660Emu] arm-none-eabi-gcc not found; using existing tests/baremetal/test_firmware.bin"; \
	else \
		echo "[N2660Emu] Notice: arm-none-eabi-gcc not found. Install gcc-arm-none-eabi (Linux) or arm-none-eabi-gcc (macOS) to recompile baremetal payloads."; \
	fi

bootrom:
	@if command -v arm-none-eabi-gcc >/dev/null 2>&1; then \
		$(MAKE) -C roms; \
	elif [ -f roms/bootrom.bin ]; then \
		echo "[N2660Emu] arm-none-eabi-gcc not found; using existing roms/bootrom.bin"; \
	else \
		echo "[N2660Emu] Notice: arm-none-eabi-gcc not found. Install gcc-arm-none-eabi (Linux) or arm-none-eabi-gcc (macOS) to recompile cleanroom BootROM."; \
	fi

rust:
	$(CARGO) build --release

test: firmware bootrom rust
	$(CARGO) test --workspace -- --nocapture

run: firmware rust
	./$(RUST_BIN) tests/baremetal/test_firmware.bin

interactive: firmware rust
	./bin/n2660emu -i

gui: firmware rust
	./bin/n2660emu --gui

trace: firmware rust
	./bin/n2660emu tests/baremetal/test_firmware.bin --trace

screen: firmware rust
	mkdir -p build
	./bin/n2660emu tests/baremetal/test_firmware.bin --screen-dump build/screen.ppm

sub-screen: firmware rust
	mkdir -p build
	./bin/n2660emu tests/baremetal/test_firmware.bin --sub-screen-dump build/sub_screen.ppm

disasm: firmware rust
	./tools/disasm tests/baremetal/test_firmware.bin --base 0x80000000 -n 40

clean:
	$(MAKE) -C tests/baremetal clean
	$(MAKE) -C roms clean
	$(CARGO) clean
	rm -rf build

.PHONY: all firmware bootrom rust test run interactive gui trace screen sub-screen disasm clean
