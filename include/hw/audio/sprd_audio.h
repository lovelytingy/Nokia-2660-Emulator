/*
 * Unisoc SPRD Audio Subsystem & Tone Generator Controller
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef HW_AUDIO_SPRD_AUDIO_H
#define HW_AUDIO_SPRD_AUDIO_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qom/object.h"

#define TYPE_SPRD_AUDIO "sprd-audio"
OBJECT_DECLARE_SIMPLE_TYPE(SprdAudioState, SPRD_AUDIO)

/* Register Offsets */
#define SPRD_AUD_CTL          0x0000  /* Audio Control Register */
#define SPRD_AUD_STS          0x0004  /* Audio Status Register */
#define SPRD_AUD_TXD          0x0008  /* Audio TX PCM FIFO Data */
#define SPRD_AUD_RXD          0x000C  /* Audio RX PCM FIFO Data */
#define SPRD_AUD_INT_EN       0x0010  /* Interrupt Enable */
#define SPRD_AUD_INT_RAW      0x0014  /* Raw Interrupt Status */
#define SPRD_AUD_INT_CLR      0x0018  /* Interrupt Clear */
#define SPRD_AUD_FIFO_CTL     0x001C  /* FIFO Control & Watermark */
#define SPRD_AUD_DAC_CTL      0x0020  /* DAC Control & Volume */
#define SPRD_AUD_PATH_CTL     0x0024  /* Audio Output Path Routing */

#define SPRD_TONE_CTL         0x0030  /* Tone Generator Control */
#define SPRD_TONE_FREQ        0x0034  /* Tone Frequency (Hz) */
#define SPRD_TONE_DUR         0x0038  /* Tone Duration (ms) */
#define SPRD_TONE_VOL         0x003C  /* Tone Volume (0-100%) */

/* Control Register Bitmasks */
#define AUD_CTL_EN            (1 << 0)
#define AUD_CTL_TX_EN         (1 << 1)
#define AUD_CTL_RX_EN         (1 << 2)
#define AUD_CTL_LOOPBACK      (1 << 3)
#define AUD_CTL_STEREO        (1 << 8)
#define AUD_CTL_16BIT         (0 << 10)
#define AUD_CTL_24BIT         (1 << 10)

/* Status Register Bitmasks */
#define AUD_STS_TX_EMPTY      (1 << 0)
#define AUD_STS_TX_FULL       (1 << 1)
#define AUD_STS_RX_EMPTY      (1 << 2)
#define AUD_STS_RX_FULL       (1 << 3)

/* Path Routing */
#define AUD_PATH_NONE         0
#define AUD_PATH_EARPIECE     1
#define AUD_PATH_SPEAKER      2
#define AUD_PATH_HEADPHONE    3

/* Tone Generator Bitmasks */
#define TONE_CTL_EN           (1 << 0)
#define TONE_CTL_CONT         (1 << 1)
#define TONE_CTL_SINE         (1 << 2)
#define TONE_CTL_BUSY         (1 << 7)

#define AUDIO_FIFO_DEPTH      64

struct SprdAudioState {
    /*< private >*/
    SysBusDevice parent_obj;

    /*< public >*/
    MemoryRegion iomem;
    qemu_irq irq;

    /* Registers */
    uint32_t aud_ctl;
    uint32_t aud_sts;
    uint32_t aud_int_en;
    uint32_t aud_int_raw;
    uint32_t aud_fifo_ctl;
    uint32_t aud_dac_ctl;
    uint32_t aud_path_ctl;

    /* Tone Generator */
    uint32_t tone_ctl;
    uint32_t tone_freq;
    uint32_t tone_dur;
    uint32_t tone_vol;

    /* Internal FIFO state */
    uint32_t tx_fifo[AUDIO_FIFO_DEPTH];
    uint32_t tx_fifo_head;
    uint32_t tx_fifo_tail;
    uint32_t tx_fifo_count;

    uint32_t total_samples_played;
};

#endif /* HW_AUDIO_SPRD_AUDIO_H */
