#!/usr/bin/env python3
"""
N2660Emu: Unified Nokia 2660 Flip Hardware Emulator CLI
Supports QEMU system emulation and Unicorn engine execution.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: GPL-2.0-or-later
"""

import sys
import os
import subprocess
import argparse
import shutil

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

def run_unicorn_engine(bin_path, entry_addr, trace=False, timeout=10):
    script_path = os.path.join(PROJECT_ROOT, "tools", "unicorn_runner.py")
    cmd = [
        sys.executable,
        script_path,
        bin_path,
        "--entry", hex(entry_addr),
        "--timeout", str(timeout)
    ]
    if trace:
        cmd.append("--trace")

    print(f"[N2660Emu] Launching Unicorn Engine on '{bin_path}'...")
    res = subprocess.run(cmd)
    return res.returncode

def run_qemu_system(firmware_path, machine="virt", cpu="cortex-a7", ram="48M", gdb=False, wait_gdb=False, extra_args=None):
    qemu_bin = shutil.which("qemu-system-arm")
    custom_qemu = os.path.join(PROJECT_ROOT, "build", "qemu", "qemu-system-arm")
    if os.path.isfile(custom_qemu) and os.access(custom_qemu, os.X_OK):
        qemu_bin = custom_qemu
        machine = "nokia-2660"

    if not qemu_bin:
        print("Error: 'qemu-system-arm' executable not found on host.")
        return 1

    cmd = [
        qemu_bin,
        "-M", machine,
        "-cpu", cpu,
        "-m", "2048M" if machine == "virt" else ram,
        "-nographic",
        "-kernel", firmware_path
    ]

    if gdb:
        cmd.extend(["-gdb", "tcp::1234"])
        if wait_gdb:
            cmd.append("-S")
            print("[N2660Emu] GDB server waiting on tcp::1234 (-S enabled). Connect with: arm-none-eabi-gdb")

    if extra_args:
        cmd.extend(extra_args)

    print(f"[N2660Emu] Executing QEMU ({machine}, {cpu}): {' '.join(cmd)}")
    try:
        res = subprocess.run(cmd)
        return res.returncode
    except KeyboardInterrupt:
        print("\n[N2660Emu] Session terminated by user.")
        return 0

def main():
    parser = argparse.ArgumentParser(
        description="N2660Emu: Nokia 2660 Flip (Unisoc T107) Emulation Environment",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  Run baremetal test payload in Unicorn VM:
    n2660emu tests/baremetal/test_firmware.bin --engine unicorn

  Run with instruction tracing:
    n2660emu tests/baremetal/test_firmware.bin --engine unicorn --trace

  Run in QEMU with GDB server enabled:
    n2660emu tests/baremetal/test_firmware.elf --engine qemu --gdb
"""
    )
    parser.add_argument("firmware", help="Path to firmware ELF or raw binary")
    parser.add_argument("--engine", choices=["auto", "unicorn", "qemu", "qemu-virt"], default="auto",
                        help="Emulation backend engine (default: auto)")
    parser.add_argument("--entry", type=lambda x: int(x, 0), default=0x80000000,
                        help="Entry address for raw binary (default: 0x80000000)")
    parser.add_argument("--trace", action="store_true", help="Enable CPU instruction tracing")
    parser.add_argument("--gdb", action="store_true", help="Start GDB stub server on port 1234")
    parser.add_argument("-S", "--wait-gdb", action="store_true", help="Freeze CPU at startup and wait for GDB connection")
    parser.add_argument("--timeout", type=int, default=10, help="Execution timeout in seconds")

    args, unknown = parser.parse_known_args()

    fw_path = os.path.abspath(args.firmware)
    if not os.path.isfile(fw_path):
        print(f"Error: Firmware file '{fw_path}' does not exist.")
        sys.exit(1)

    # Determine engine if auto
    engine = args.engine
    if engine == "auto":
        if fw_path.endswith(".bin"):
            engine = "unicorn"
        else:
            engine = "qemu"

    if engine == "unicorn":
        ret = run_unicorn_engine(fw_path, args.entry, trace=args.trace, timeout=args.timeout)
    else:
        machine = "virt" if engine == "qemu-virt" else "nokia-2660"
        ret = run_qemu_system(fw_path, machine=machine, gdb=args.gdb, wait_gdb=args.wait_gdb, extra_args=unknown)

    sys.exit(ret)

if __name__ == "__main__":
    main()
