// SPDX-License-Identifier: MIT
//! N2660 Core: Ultra-high performance hardware emulation engine for the Nokia 2660 Flip (Unisoc T107 / UMS9117)

pub mod types;
pub mod peripherals;
pub mod bsl;
pub mod atags;
pub mod bootrom;
pub mod pac;
pub mod vm;

pub use types::*;
pub use peripherals::*;
pub use bsl::{crc16_ccitt, hdlc_stuff, hdlc_unstuff, build_bsl_packet, parse_bsl_packet, BslPacket};
pub use atags::AtagsBuilder;
pub use bootrom::{SprdHeader, SPRD_MAGIC};
pub use pac::{PacArchive, PacFileRecord};
pub use vm::{T107Vm, VmContext};

pub fn version() -> &'static str {
    "0.1.0"
}
