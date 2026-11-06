# Root Makefile for Nokia 2660 Emulator (N2660Emu)

PYTHON ?= python3
CARGO ?= cargo
RUST_BIN = target/release/n2660emu-rs

all: firmware bootrom rust

firmware:
	$(MAKE) -C tests/baremetal

bootrom:
	$(MAKE) -C roms

rust:
	$(CARGO) build --release

test: firmware bootrom rust
	$(CARGO) test --workspace -- --nocapture
	$(PYTHON) -m unittest discover -s tests/unit -v

run: firmware rust
	./$(RUST_BIN) tests/baremetal/test_firmware.bin

interactive: firmware
	./bin/n2660emu -i

gui: firmware
	./bin/n2660emu --gui

trace: firmware
	./bin/n2660emu tests/baremetal/test_firmware.bin --engine unicorn --trace

screen: firmware
	./bin/n2660emu tests/baremetal/test_firmware.bin --engine unicorn --screen-dump build/screen.ppm --preview-screen

sub-screen: firmware
	./bin/n2660emu tests/baremetal/test_firmware.bin --engine unicorn --sub-screen-dump build/sub_screen.ppm --preview-sub-screen

disasm: firmware
	./tools/disasm.py tests/baremetal/test_firmware.bin --base 0x80000000 -n 40

clean:
	$(MAKE) -C tests/baremetal clean
	$(MAKE) -C roms clean
	$(CARGO) clean
	rm -rf build __pycache__ tools/__pycache__ tests/unit/__pycache__ roms/__pycache__

.PHONY: all firmware bootrom rust test run interactive gui trace screen sub-screen disasm clean
