# Contributing to N2660Emu

Thank you for your interest in contributing to **N2660Emu**! Because this is a hardware emulation and reverse-engineering research project, we enforce strict intellectual property and cleanroom policies to protect contributors and the open-source community.

---

## 1. Intellectual Property & Cleanroom Guidelines

To ensure the repository remains legally compliant, all contributors must adhere to the following rules:

### A. Original Code Only (No Decompiled Code)
- All contributions must be your **own original expression**.
- **Do not copy and paste decompiler output** (from Ghidra, IDA Pro, Binary Ninja, etc.) directly into emulator source code.
- Decompilers and disassemblers should only be used to **understand behavior and interface specifications** (e.g., register offsets, bit flags, state transitions). Once understood, write your own clean C or Python implementation.

### B. Functional Facts vs. Copyrightable Expression
- You are free to document and use:
  - Hardware register physical addresses (e.g. `0x50000000`).
  - Bitfield layouts and masks.
  - Peripheral state machine behavior.
  - Interrupt vectors and memory layouts.
- These are unprotected functional facts under the *Idea-Expression Dichotomy* and Section 52(1)(ab) of the Indian Copyright Act, 1957.

---

## 2. Strict Firmware Exclusion Policy

- **Never submit proprietary binaries**: Do **not** commit, attach, or link to any manufacturer firmware, ROM dumps, bootloaders (`fdl1.bin`, `fdl2.bin`, `boot0.bin`, `boot1.bin`), modem blobs, or OS images.
- Pull requests containing proprietary binary files or copyrighted blobs will be immediately rejected and permanently purged from git history.
- Unit tests and automated tests must strictly use **independently created baremetal test payloads** (like the ones under `tests/baremetal/`).

---

## 3. Third-Party Code & Attribution

- N2660Emu is licensed under the **MIT License**.
- Any third-party code contributed to the repository must be under a compatible permissive license (e.g., MIT, BSD-2-Clause, BSD-3-Clause, Apache 2.0).
- If you adapt code from another open-source project (e.g., Linux kernel drivers or QEMU models), you must:
  - Retain the original copyright notices and license text.
  - Clearly credit the original authors in the commit message and file header.

---

## 4. Developer Certificate of Origin (DCO)

By submitting a pull request or patch, you certify that:
1. The contribution was created in whole or in part by you, and you have the right to submit it under the MIT License; or
2. The contribution is based on existing open-source work that is documented with a compatible open-source license; and
3. You have not violated any non-disclosure agreements, proprietary software licenses, or copyright protections in creating this contribution.
