#!/usr/bin/env python3
"""
N2660Emu: Unified Nokia 2660 Flip Hardware Emulator CLI
Supports QEMU system emulation and Unicorn engine execution.

Copyright (c) 2026 N2660Emu Developers
SPDX-License-Identifier: MIT
"""

import sys
import os
import subprocess
import argparse
import shutil

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

def run_unicorn_engine(bin_path, entry_addr, trace=False, timeout=10, screen_dump=None, preview_screen=False, keys=None, flip="open", sub_screen_dump=None, preview_sub_screen=False, flash=None, fdl1=None, fdl1_addr=0x00006200, fdl2=None, fdl2_addr=0x80100000, bsl=False, audio_dump=None, bootrom=False, bootrom_file=None, boot0=None, boot_mode="auto", boot_key="none", atags=False, cmdline=None, atags_addr=0x80000100, board_id=0x9117):
    # Prefer ultra-fast native Rust emulation engine if compiled
    rust_bin = os.path.join(PROJECT_ROOT, "target", "release", "n2660emu-rs")
    if os.path.isfile(rust_bin) and os.access(rust_bin, os.X_OK):
        cmd = [
            rust_bin,
            "--entry", hex(entry_addr),
            "--timeout", str(timeout),
            "--flip", flip
        ]
        if bin_path:
            cmd.append(bin_path)
        if trace:
            cmd.append("--trace")
        if flash:
            cmd.extend(["--flash", flash])
        if boot0:
            cmd.extend(["--boot0", boot0])
        if fdl1:
            cmd.extend(["--fdl1", fdl1, "--fdl1-addr", hex(fdl1_addr)])
        if fdl2:
            cmd.extend(["--fdl2", fdl2, "--fdl2-addr", hex(fdl2_addr)])
        if bsl:
            cmd.append("--bsl")
        if bootrom:
            cmd.append("--bootrom")
        if atags:
            cmd.extend(["--atags", "--atags-addr", hex(atags_addr), "--board-id", hex(board_id)])
        if cmdline:
            cmd.extend(["--cmdline", cmdline])
        if screen_dump:
            cmd.extend(["--screen-dump", screen_dump])
        if sub_screen_dump:
            cmd.extend(["--sub-screen-dump", sub_screen_dump])
        if keys:
            for k in keys:
                cmd.extend(["--key", k])

        target_desc = boot0 if boot0 else (fdl1 if fdl1 else (bin_path or "BootROM"))
        print(f"[N2660Emu] Launching Rust Core Engine on '{target_desc}'...")
        res = subprocess.run(cmd)
        return res.returncode

    script_path = os.path.join(PROJECT_ROOT, "tools", "unicorn_runner.py")
    cmd = [
        sys.executable,
        script_path,
        "--entry", hex(entry_addr),
        "--timeout", str(timeout),
        "--flip", flip,
        "--boot-mode", boot_mode,
        "--boot-key", boot_key
    ]
    if bin_path:
        cmd.append(bin_path)
    if trace:
        cmd.append("--trace")
    if flash:
        cmd.extend(["--flash", flash])
    if bootrom:
        cmd.append("--bootrom")
    if bootrom_file:
        cmd.extend(["--bootrom-file", bootrom_file])
    if boot0:
        cmd.extend(["--boot0", boot0])
    if atags:
        cmd.extend(["--atags", "--atags-addr", hex(atags_addr), "--board-id", hex(board_id)])
    if cmdline:
        cmd.extend(["--cmdline", cmdline])
    if fdl1:
        cmd.extend(["--fdl1", fdl1, "--fdl1-addr", hex(fdl1_addr)])
    if fdl2:
        cmd.extend(["--fdl2", fdl2, "--fdl2-addr", hex(fdl2_addr)])
    if bsl:
        cmd.append("--bsl")
    if audio_dump:
        cmd.extend(["--audio-dump", audio_dump])
    if screen_dump:
        cmd.extend(["--screen-dump", screen_dump])
    if sub_screen_dump:
        cmd.extend(["--sub-screen-dump", sub_screen_dump])
    if preview_screen:
        cmd.append("--preview-screen")
    if preview_sub_screen:
        cmd.append("--preview-sub-screen")
    if keys:
        for k in keys:
            cmd.extend(["--key", k])

    target_desc = boot0 if boot0 else (fdl1 if fdl1 else (bin_path or "BootROM"))
    print(f"[N2660Emu] Launching Unicorn Engine on '{target_desc}'...")
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

def run_interactive_gui(bin_path, entry_addr=0x80000000, flip="open", view="main", fps=30, slice_insns=25000, headless=False, timeout=2.0):
    script_path = os.path.join(PROJECT_ROOT, "tools", "gui_runner.py")
    cmd = [
        sys.executable,
        script_path,
        "--entry", hex(entry_addr),
        "--flip", flip,
        "--view", view,
        "--fps", str(fps),
        "--slice", str(slice_insns)
    ]
    if bin_path:
        cmd.append(bin_path)
    if headless:
        cmd.extend(["--headless", "--timeout", str(timeout)])
    res = subprocess.run(cmd)
    return res.returncode

def run_desktop_gui(bin_path, entry_addr=0x80000000, flip="open", theme="Deep Blue"):
    script_path = os.path.join(PROJECT_ROOT, "tools", "desktop_gui.py")
    cmd = [
        sys.executable,
        script_path,
        "--entry", hex(entry_addr),
        "--flip", flip,
        "--theme", theme
    ]
    if bin_path:
        cmd.append(bin_path)
    res = subprocess.run(cmd)
    return res.returncode

def main():
    parser = argparse.ArgumentParser(
        description="N2660Emu: Nokia 2660 Flip (Unisoc T107) Emulation Environment",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  Launch native Nokia 2660 Flip desktop skin GUI:
    n2660emu --gui
    n2660emu --gui --theme "Pop Red"

  Launch interactive terminal TUI:
    n2660emu -i

  Run baremetal test payload in Unicorn VM:
    n2660emu tests/baremetal/test_firmware.bin --engine unicorn

  Run with instruction tracing:
    n2660emu tests/baremetal/test_firmware.bin --engine unicorn --trace

  Run in QEMU with GDB server enabled:
    n2660emu tests/baremetal/test_firmware.elf --engine qemu --gdb
"""
    )
    parser.add_argument("firmware", nargs="?", default=None, help="Path to firmware ELF or raw binary")
    parser.add_argument("--engine", choices=["auto", "unicorn", "qemu", "qemu-virt"], default="auto",
                        help="Emulation backend engine (default: auto)")
    parser.add_argument("--entry", type=lambda x: int(x, 0), default=0x80000000,
                        help="Entry address for raw binary (default: 0x80000000)")
    parser.add_argument("--trace", action="store_true", help="Enable CPU instruction tracing")
    parser.add_argument("--gdb", action="store_true", help="Start GDB stub server on port 1234")
    parser.add_argument("-S", "--wait-gdb", action="store_true", help="Freeze CPU at startup and wait for GDB connection")
    parser.add_argument("--timeout", type=int, default=10, help="Execution timeout in seconds")
    parser.add_argument("--screen-dump", type=str, default=None, help="Dump main display framebuffer to PPM image")
    parser.add_argument("--sub-screen-dump", type=str, default=None, help="Dump outer sub-display framebuffer to PPM image")
    parser.add_argument("--preview-screen", action="store_true", help="Render ANSI truecolor main screen in terminal")
    parser.add_argument("--preview-sub-screen", action="store_true", help="Render ANSI truecolor outer sub-screen in terminal")
    parser.add_argument("--key", type=str, action="append", help="Inject key press (e.g. --key 5 or --key ok)")
    parser.add_argument("--flip", choices=["open", "closed"], default="open", help="Hall effect flip sensor initial state (default: open)")
    parser.add_argument("--flash", type=str, default=None, help="Path to raw SPI NAND flash image to mount")
    parser.add_argument("--fdl1", type=str, default=None, help="Path to FDL1 binary to stage and run (default addr: 0x00006200)")
    parser.add_argument("--fdl1-addr", type=lambda x: int(x, 0), default=0x00006200, help="FDL1 load address (default: 0x00006200)")
    parser.add_argument("--fdl2", type=str, default=None, help="Path to FDL2 binary to stage into DRAM (default addr: 0x80100000)")
    parser.add_argument("--fdl2-addr", type=lambda x: int(x, 0), default=0x80100000, help="FDL2 load address (default: 0x80100000)")
    parser.add_argument("--pac", type=str, default=None, help="Path to Unisoc .PAC firmware container to unpack and run")
    parser.add_argument("--bsl", action="store_true", help="Simulate BootROM BSL handshake over UART0 when loading FDL1")
    parser.add_argument("--bootrom", action="store_true", help="Execute hardware BootROM reset vector at 0x00000000")
    parser.add_argument("--bootrom-file", type=str, default=None, help="Path to custom BootROM binary (default: roms/bootrom.bin)")
    parser.add_argument("--boot0", type=str, default=None, help="Path to BOOT0 binary to stage into SPI NAND Block 0")
    parser.add_argument("--boot-mode", choices=["auto", "bsl", "nand"], default="auto", help="Boot mode dispatch (default: auto)")
    parser.add_argument("--boot-key", choices=["star", "none"], default="none", help="Hardware boot key to hold at reset (default: none)")
    parser.add_argument("--audio-dump", type=str, default=None, help="Path to dump audio output as WAV file")
    parser.add_argument("--atags", action="store_true", help="Stage ARM Boot Tags (ATAGS) at 0x80000100 for kernel booting")
    parser.add_argument("--atags-addr", type=lambda x: int(x, 0), default=0x80000100, help="Physical DRAM address to stage ATAGS (default: 0x80000100)")
    parser.add_argument("--board-id", type=lambda x: int(x, 0), default=0x9117, help="ARM machine/board ID in r1 (default: 0x9117)")
    parser.add_argument("--cmdline", type=str, default=None, help="Kernel command line string passed via ATAG_CMDLINE")
    parser.add_argument("-i", "--interactive", action="store_true",
                        help="Launch interactive terminal GUI with live LCD, keypad matrix, and telemetry")
    parser.add_argument("-g", "--gui", action="store_true",
                        help="Launch native desktop GUI with Nokia 2660 Flip physical skin, interactive keypad, and live telemetry")
    parser.add_argument("--theme", choices=["Deep Blue", "Classic Black", "Pop Red"], default="Deep Blue",
                        help="Physical casing color theme for desktop GUI (default: Deep Blue)")
    parser.add_argument("--view", choices=["main", "sub", "dual"], default="main",
                        help="Initial display view for interactive mode (default: main)")
    parser.add_argument("--fps", type=int, default=30, help="Target UI refresh rate for interactive mode (default: 30)")

    args, unknown = parser.parse_known_args()

    fw_path = os.path.abspath(args.firmware) if args.firmware else None

    # Native Desktop Host GUI Mode
    if args.gui:
        demo_fw = fw_path
        if not demo_fw:
            candidates = [
                os.path.join(PROJECT_ROOT, "tests", "baremetal", "interactive_demo.bin"),
                os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
            ]
            for c in candidates:
                if os.path.isfile(c):
                    demo_fw = c
                    break
        if not demo_fw:
            print("Error: No firmware specified for desktop GUI mode and default binary not found.")
            sys.exit(1)
        ret = run_desktop_gui(demo_fw, entry_addr=args.entry, flip=args.flip, theme=args.theme)
        sys.exit(ret)

    # Interactive Terminal GUI Mode
    if args.interactive:
        demo_fw = fw_path
        if not demo_fw:
            candidates = [
                os.path.join(PROJECT_ROOT, "tests", "baremetal", "interactive_demo.bin"),
                os.path.join(PROJECT_ROOT, "tests", "baremetal", "test_firmware.bin")
            ]
            for c in candidates:
                if os.path.isfile(c):
                    demo_fw = c
                    break
        if not demo_fw:
            print("Error: No firmware specified for interactive mode and default binary not found.")
            sys.exit(1)
        ret = run_interactive_gui(demo_fw, entry_addr=args.entry, flip=args.flip, view=args.view, fps=args.fps)
        sys.exit(ret)

    if not args.firmware and not args.fdl1 and not args.bootrom and not args.boot0 and not args.pac:
        print("Error: Either firmware, --fdl1, --bootrom, --boot0, or --pac must be specified.")
        sys.exit(1)

    if fw_path and not os.path.isfile(fw_path):
        print(f"Error: Firmware file '{fw_path}' does not exist.")
        sys.exit(1)

    # Automated .PAC extraction and staging
    pac_file = args.pac or (fw_path if fw_path and fw_path.lower().endswith(".pac") else None)
    if pac_file:
        from tools.pac_extractor import PacExtractor
        print(f"[N2660Emu] Detected Unisoc .PAC firmware container: '{pac_file}'")
        extractor = PacExtractor(pac_file)
        extractor.parse()
        pac_cache_dir = os.path.join(PROJECT_ROOT, "build", "pac_cache", os.path.splitext(os.path.basename(pac_file))[0])
        extractor.extract_all(pac_cache_dir)
        for part in extractor.files:
            pid = part["id"].upper()
            target_path = os.path.join(pac_cache_dir, part["filename"])
            if os.path.isfile(target_path):
                if "FDL1" in pid and not args.fdl1:
                    args.fdl1 = target_path
                    if part["load_addr"]:
                        args.fdl1_addr = part["load_addr"]
                elif "FDL2" in pid and not args.fdl2:
                    args.fdl2 = target_path
                    if part["load_addr"]:
                        args.fdl2_addr = part["load_addr"]
                elif ("BOOT0" in pid or "SPL" in pid) and not args.boot0:
                    args.boot0 = target_path
        if not args.fdl1 and not args.boot0:
            fw_path = None
        elif args.fdl1:
            fw_path = None

    # Determine engine if auto
    engine = args.engine
    if engine == "auto":
        if args.bootrom or args.boot0 or args.fdl1 or (fw_path and fw_path.endswith(".bin")):
            engine = "unicorn"
        else:
            engine = "qemu"

    if engine == "unicorn":
        ret = run_unicorn_engine(fw_path, args.entry, trace=args.trace, timeout=args.timeout,
                                 screen_dump=args.screen_dump, preview_screen=args.preview_screen,
                                 keys=args.key, flip=args.flip,
                                 sub_screen_dump=args.sub_screen_dump, preview_sub_screen=args.preview_sub_screen,
                                 flash=args.flash, fdl1=args.fdl1, fdl1_addr=args.fdl1_addr,
                                 fdl2=args.fdl2, fdl2_addr=args.fdl2_addr, bsl=args.bsl,
                                 audio_dump=args.audio_dump,
                                 bootrom=args.bootrom, bootrom_file=args.bootrom_file,
                                 boot0=args.boot0, boot_mode=args.boot_mode, boot_key=args.boot_key,
                                 atags=args.atags, cmdline=args.cmdline,
                                 atags_addr=args.atags_addr, board_id=args.board_id)
    else:
        machine = "virt" if engine == "qemu-virt" else "nokia-2660"
        ret = run_qemu_system(fw_path, machine=machine, gdb=args.gdb, wait_gdb=args.wait_gdb, extra_args=unknown)

    sys.exit(ret)

if __name__ == "__main__":
    main()
