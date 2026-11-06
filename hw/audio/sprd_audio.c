/*
 * Unisoc SPRD Audio Subsystem & Tone Generator Controller
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/audio/sprd_audio.h"
#include "hw/irq.h"
#include "migration/vmstate.h"
#include "qemu/log.h"
#include "qemu/module.h"

static void sprd_audio_update_sts(SprdAudioState *s)
{
    s->aud_sts = 0;
    if (s->tx_fifo_count == 0) {
        s->aud_sts |= AUD_STS_TX_EMPTY;
    } else if (s->tx_fifo_count >= AUDIO_FIFO_DEPTH) {
        s->aud_sts |= AUD_STS_TX_FULL;
    }
    s->aud_sts |= (s->tx_fifo_count & 0xFF) << 8;

    /* RX FIFO initially empty */
    s->aud_sts |= AUD_STS_RX_EMPTY;
}

static uint64_t sprd_audio_read(void *opaque, hwaddr offset, unsigned size)
{
    SprdAudioState *s = SPRD_AUDIO(opaque);

    switch (offset) {
    case SPRD_AUD_CTL:
        return s->aud_ctl;
    case SPRD_AUD_STS:
        sprd_audio_update_sts(s);
        return s->aud_sts;
    case SPRD_AUD_RXD:
        /* Return microphone sample (silence) */
        return 0;
    case SPRD_AUD_INT_EN:
        return s->aud_int_en;
    case SPRD_AUD_INT_RAW:
        return s->aud_int_raw;
    case SPRD_AUD_FIFO_CTL:
        return s->aud_fifo_ctl;
    case SPRD_AUD_DAC_CTL:
        return s->aud_dac_ctl;
    case SPRD_AUD_PATH_CTL:
        return s->aud_path_ctl;
    case SPRD_TONE_CTL:
        return s->tone_ctl;
    case SPRD_TONE_FREQ:
        return s->tone_freq;
    case SPRD_TONE_DUR:
        return s->tone_dur;
    case SPRD_TONE_VOL:
        return s->tone_vol;
    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "%s: Bad read offset 0x%" HWADDR_PRIx "\n",
                      __func__, offset);
        return 0;
    }
}

static void sprd_audio_write(void *opaque, hwaddr offset, uint64_t value, unsigned size)
{
    SprdAudioState *s = SPRD_AUDIO(opaque);

    switch (offset) {
    case SPRD_AUD_CTL:
        s->aud_ctl = value;
        break;
    case SPRD_AUD_TXD:
        if (s->tx_fifo_count < AUDIO_FIFO_DEPTH) {
            s->tx_fifo[s->tx_fifo_head] = (uint32_t)value;
            s->tx_fifo_head = (s->tx_fifo_head + 1) % AUDIO_FIFO_DEPTH;
            s->tx_fifo_count++;
            s->total_samples_played++;
        }
        sprd_audio_update_sts(s);
        break;
    case SPRD_AUD_INT_EN:
        s->aud_int_en = value;
        break;
    case SPRD_AUD_INT_CLR:
        s->aud_int_raw &= ~value;
        break;
    case SPRD_AUD_FIFO_CTL:
        s->aud_fifo_ctl = value;
        if (value & 1) { /* Reset TX FIFO */
            s->tx_fifo_head = 0;
            s->tx_fifo_tail = 0;
            s->tx_fifo_count = 0;
            sprd_audio_update_sts(s);
        }
        break;
    case SPRD_AUD_DAC_CTL:
        s->aud_dac_ctl = value;
        break;
    case SPRD_AUD_PATH_CTL:
        s->aud_path_ctl = value;
        break;
    case SPRD_TONE_CTL:
        s->tone_ctl = value;
        if (value & TONE_CTL_EN) {
            /* If duration specified, tone sounds and self-clears */
            if (!(value & TONE_CTL_CONT)) {
                s->tone_ctl &= ~TONE_CTL_EN;
            }
        }
        break;
    case SPRD_TONE_FREQ:
        s->tone_freq = value;
        break;
    case SPRD_TONE_DUR:
        s->tone_dur = value;
        break;
    case SPRD_TONE_VOL:
        s->tone_vol = value;
        break;
    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "%s: Bad write offset 0x%" HWADDR_PRIx " = 0x%" PRIx64 "\n",
                      __func__, offset, value);
        break;
    }
}

static const MemoryRegionOps sprd_audio_ops = {
    .read = sprd_audio_read,
    .write = sprd_audio_write,
    .endianness = DEVICE_LITTLE_ENDIAN,
    .valid = {
        .min_access_size = 2,
        .max_access_size = 4,
    },
};

static void sprd_audio_reset(DeviceState *dev)
{
    SprdAudioState *s = SPRD_AUDIO(dev);

    s->aud_ctl = 0;
    s->aud_sts = AUD_STS_TX_EMPTY | AUD_STS_RX_EMPTY;
    s->aud_int_en = 0;
    s->aud_int_raw = 0;
    s->aud_fifo_ctl = (16 << 8) | (16 << 16);
    s->aud_dac_ctl = 0x00008001; /* DAC enabled, 50% volume */
    s->aud_path_ctl = AUD_PATH_SPEAKER;
    s->tone_ctl = 0;
    s->tone_freq = 1000;
    s->tone_dur = 50;
    s->tone_vol = 80;

    s->tx_fifo_head = 0;
    s->tx_fifo_tail = 0;
    s->tx_fifo_count = 0;
    s->total_samples_played = 0;
}

static void sprd_audio_init(Object *obj)
{
    SprdAudioState *s = SPRD_AUDIO(obj);
    SysBusDevice *sbd = SYS_BUS_DEVICE(obj);

    memory_region_init_io(&s->iomem, obj, &sprd_audio_ops, s,
                          "sprd-audio", 0x1000);
    sysbus_init_mmio(sbd, &s->iomem);
    sysbus_init_irq(sbd, &s->irq);
}

static const VMStateDescription vmstate_sprd_audio = {
    .name = "sprd_audio",
    .version_id = 1,
    .minimum_version_id = 1,
    .fields = (VMStateField[]) {
        VMSTATE_UINT32(aud_ctl, SprdAudioState),
        VMSTATE_UINT32(aud_sts, SprdAudioState),
        VMSTATE_UINT32(aud_int_en, SprdAudioState),
        VMSTATE_UINT32(aud_int_raw, SprdAudioState),
        VMSTATE_UINT32(aud_fifo_ctl, SprdAudioState),
        VMSTATE_UINT32(aud_dac_ctl, SprdAudioState),
        VMSTATE_UINT32(aud_path_ctl, SprdAudioState),
        VMSTATE_UINT32(tone_ctl, SprdAudioState),
        VMSTATE_UINT32(tone_freq, SprdAudioState),
        VMSTATE_UINT32(tone_dur, SprdAudioState),
        VMSTATE_UINT32(tone_vol, SprdAudioState),
        VMSTATE_END_OF_LIST()
    }
};

static void sprd_audio_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->reset = sprd_audio_reset;
    dc->vmsd = &vmstate_sprd_audio;
    dc->desc = "Unisoc SPRD Audio Subsystem and Tone Generator";
}

static const TypeInfo sprd_audio_info = {
    .name          = TYPE_SPRD_AUDIO,
    .parent        = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(SprdAudioState),
    .instance_init = sprd_audio_init,
    .class_init    = sprd_audio_class_init,
};

static void sprd_audio_register_types(void)
{
    type_register_static(&sprd_audio_info);
}

type_init(sprd_audio_register_types)
