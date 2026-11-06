// SPDX-License-Identifier: MIT
//! Unisoc SPRD Audio Subsystem, DAC and Tone Buzzer model

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToneEvent {
    pub freq: u32,
    pub dur: u32,
    pub vol: u8,
}

#[derive(Debug, Clone, Default)]
pub struct SprdAudio {
    pub ctl: u32,
    pub tone_freq: u32,
    pub tone_dur: u32,
    pub tone_ctl: u32,
    pub pcm_samples: Vec<i16>,
    pub tone_events: Vec<ToneEvent>,
}

impl SprdAudio {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&self, offset: u32) -> u32 {
        match offset {
            0x00 => self.ctl,
            0x04 => 0x0000_1004, // Status STS (sample count 16, TX ready)
            0x08 => self.tone_freq,
            0x0C => self.tone_dur,
            0x10 => self.tone_ctl,
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u32, val: u32) {
        match offset {
            0x00 => self.ctl = val,
            0x04 => {
                // PCM FIFO write (16-bit sample)
                self.pcm_samples.push((val & 0xFFFF) as i16);
            }
            0x08 => self.tone_freq = val,
            0x0C => self.tone_dur = val,
            0x10 => {
                self.tone_ctl = val;
                if (val & 0x1) != 0 {
                    let vol = ((val >> 8) & 0xFF) as u8;
                    self.tone_events.push(ToneEvent {
                        freq: self.tone_freq,
                        dur: self.tone_dur,
                        vol,
                    });
                    println!(
                        "[N2660Emu Audio] Beep Tone: {} Hz, {} ms, volume {}%",
                        self.tone_freq, self.tone_dur, vol
                    );
                }
            }
            _ => {}
        }
    }
}
