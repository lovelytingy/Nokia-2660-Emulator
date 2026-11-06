# Root Makefile for Nokia 2660 Emulator (N2660Emu)

PYTHON ?= python3

all: firmware

firmware:
	$(MAKE) -C tests/baremetal

test: firmware
	$(PYTHON) tests/unit/test_memory_map.py

run: firmware
	./bin/n2660emu tests/baremetal/test_firmware.bin --engine unicorn

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
	rm -rf build __pycache__ tools/__pycache__ tests/unit/__pycache__

.PHONY: all firmware test run trace disasm clean
