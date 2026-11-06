/*
 * Baremetal audio and tone generator driver for Unisoc SPRD Audio Subsystem
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "audio.h"

void audio_init(uint8_t stereo)
{
    /* Reset Audio FIFO */
    REG_AUD_FIFO_CTL = 0x0001;

    /* Enable Audio Block & TX FIFO, configure stereo */
    uint32_t ctl = AUD_CTL_ENABLE | AUD_CTL_TX_ENABLE;
    if (stereo) {
        ctl |= AUD_CTL_STEREO;
    }
    REG_AUD_CTL = ctl;

    /* Default to Loudspeaker output */
    REG_AUD_PATH_CTL = AUD_PATH_LOUDSPEAKER;

    /* Set default DAC volume: 80% */
    REG_AUD_DAC_CTL = 0x00008001 | (80 << 8);
}

void audio_set_volume(uint8_t volume, uint8_t path)
{
    REG_AUD_DAC_CTL = 0x00000001 | ((uint32_t)volume << 8);
    REG_AUD_PATH_CTL = (uint32_t)path;
}

void audio_write_pcm(uint16_t left, uint16_t right)
{
    uint32_t sample = ((uint32_t)right << 16) | (uint32_t)left;
    REG_AUD_TXD = sample;
}

void audio_play_tone(uint32_t freq_hz, uint32_t duration_ms, uint8_t volume)
{
    REG_TONE_FREQ = freq_hz;
    REG_TONE_DUR = duration_ms;
    REG_TONE_VOL = volume;
    REG_TONE_CTL = TONE_CTL_ENABLE | TONE_CTL_SINE_WAVE;
}

uint32_t audio_get_status(void)
{
    return REG_AUD_STS;
}
