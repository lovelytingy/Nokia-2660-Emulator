// SPDX-License-Identifier: MIT
//! Unisoc SPRD Hardware Peripherals

pub mod uart;
pub mod timer;
pub mod gpio;
pub mod intc;
pub mod kpd;
pub mod spi_lcd;
pub mod sfc;
pub mod audio;
pub mod misc;

pub use uart::SprdUart;
pub use timer::SprdTimer;
pub use gpio::SprdGpio;
pub use intc::SprdIntc;
pub use kpd::SprdKpd;
pub use spi_lcd::{SprdDisplaySubsystem, Gc9307Controller, St7735Controller};
pub use sfc::SprdSfc;
pub use audio::{SprdAudio, ToneEvent};
pub use misc::SprdMisc;
