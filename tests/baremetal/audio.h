/*
 * Baremetal audio and tone generator driver for Unisoc SPRD Audio Subsystem
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef AUDIO_H
#define AUDIO_H

#include <stdint.h>
#include <stddef.h>

#define SPRD_AUDIO_BASE       0x500C0000

#define REG_AUD_CTL           (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0000))
#define REG_AUD_STS           (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0004))
#define REG_AUD_TXD           (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0008))
#define REG_AUD_RXD           (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x000C))
#define REG_AUD_INT_EN        (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0010))
#define REG_AUD_INT_RAW       (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0014))
#define REG_AUD_INT_CLR       (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0018))
#define REG_AUD_FIFO_CTL      (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x001C))
#define REG_AUD_DAC_CTL       (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0020))
#define REG_AUD_PATH_CTL      (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0024))

#define REG_TONE_CTL          (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0030))
#define REG_TONE_FREQ         (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0034))
#define REG_TONE_DUR          (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x0038))
#define REG_TONE_VOL          (*(volatile uint32_t *)(SPRD_AUDIO_BASE + 0x003C))

/* Audio paths */
#define AUD_PATH_EARPIECE     1
#define AUD_PATH_LOUDSPEAKER  2
#define AUD_PATH_HEADPHONE    3

/* Control Bitmasks */
#define AUD_CTL_ENABLE        (1 << 0)
#define AUD_CTL_TX_ENABLE     (1 << 1)
#define AUD_CTL_STEREO        (1 << 8)

/* Tone Bitmasks */
#define TONE_CTL_ENABLE       (1 << 0)
#define TONE_CTL_CONTINUOUS   (1 << 1)
#define TONE_CTL_SINE_WAVE    (1 << 2)

void audio_init(uint8_t stereo);
void audio_set_volume(uint8_t volume, uint8_t path);
void audio_write_pcm(uint16_t left, uint16_t right);
void audio_play_tone(uint32_t freq_hz, uint32_t duration_ms, uint8_t volume);
uint32_t audio_get_status(void);

#endif /* AUDIO_H */
