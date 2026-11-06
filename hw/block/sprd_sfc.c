/*
 * Unisoc / Spreadtrum Serial Flash Controller (SFC) & SPI NAND Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "qemu/log.h"
#include "qemu/module.h"
#include "hw/block/sprd_sfc.h"

static void sprd_sfc_execute_cmd(SprdSfcState *s)
{
    s->sts = 0x01; /* Busy */

    switch (s->cmd) {
    case NAND_CMD_RESET:
        s->write_enabled = false;
        s->buf_ptr = 0;
        memset(s->page_buffer, 0xFF, sizeof(s->page_buffer));
        break;

    case NAND_CMD_READ_ID:
        /* Load JEDEC ID into start of buffer */
        s->buf_ptr = 0;
        s->page_buffer[0] = JEDEC_MFR_WINBOND;
        s->page_buffer[1] = (JEDEC_DEV_W25N01GV >> 8) & 0xFF;
        s->page_buffer[2] = JEDEC_DEV_W25N01GV & 0xFF;
        s->page_buffer[3] = 0x00;
        break;

    case NAND_CMD_PAGE_READ: {
        /* Page address in s->addr */
        uint32_t page_idx = s->addr;
        uint32_t flash_offset = page_idx * NAND_PAGE_SIZE;
        s->buf_ptr = 0;

        if (flash_offset + NAND_PAGE_SIZE <= SPRD_FLASH_SIZE && s->storage) {
            memcpy(s->page_buffer, &s->storage[flash_offset], NAND_PAGE_SIZE);
        } else {
            memset(s->page_buffer, 0xFF, NAND_PAGE_SIZE);
        }
        break;
    }

    case NAND_CMD_READ_CACHE:
    case NAND_CMD_READ_FAST:
        /* Buffer pointer set to column address in s->addr */
        s->buf_ptr = s->addr % NAND_PAGE_TOTAL_SIZE;
        break;

    case NAND_CMD_WRITE_ENABLE:
        s->write_enabled = true;
        s->status_reg3 |= 0x02; /* WEL bit */
        break;

    case NAND_CMD_WRITE_DISABLE:
        s->write_enabled = false;
        s->status_reg3 &= ~0x02;
        break;

    case NAND_CMD_PROGRAM_LOAD:
        s->buf_ptr = s->addr % NAND_PAGE_TOTAL_SIZE;
        break;

    case NAND_CMD_PROGRAM_EXEC: {
        /* Write page_buffer into NAND array */
        if (s->write_enabled && s->storage) {
            uint32_t page_idx = s->addr;
            uint32_t flash_offset = page_idx * NAND_PAGE_SIZE;
            if (flash_offset + NAND_PAGE_SIZE <= SPRD_FLASH_SIZE) {
                /* NAND program: bits can only be changed from 1 to 0 (bitwise AND) */
                for (uint32_t i = 0; i < NAND_PAGE_SIZE; i++) {
                    s->storage[flash_offset + i] &= s->page_buffer[i];
                }
            }
        }
        s->write_enabled = false;
        s->status_reg3 &= ~0x02;
        break;
    }

    case NAND_CMD_BLOCK_ERASE: {
        /* Erase 128 KB block: set all bytes to 0xFF */
        if (s->write_enabled && s->storage) {
            uint32_t block_idx = s->addr / NAND_PAGES_PER_BLOCK;
            uint32_t flash_offset = block_idx * NAND_BLOCK_SIZE;
            if (flash_offset + NAND_BLOCK_SIZE <= SPRD_FLASH_SIZE) {
                memset(&s->storage[flash_offset], 0xFF, NAND_BLOCK_SIZE);
            }
        }
        s->write_enabled = false;
        s->status_reg3 &= ~0x02;
        break;
    }

    case NAND_CMD_GET_FEATURE:
        s->buf_ptr = 0;
        if (s->addr == 0xC0) {
            s->page_buffer[0] = s->status_reg3;
        } else if (s->addr == 0xB0) {
            s->page_buffer[0] = s->status_reg2;
        } else {
            s->page_buffer[0] = s->status_reg1;
        }
        break;

    default:
        qemu_log_mask(LOG_UNIMP, "sprd_sfc: Unhandled flash command 0x%02X\n", s->cmd);
        break;
    }

    s->sts = 0x02; /* Done / Ready */
}

static uint64_t sprd_sfc_read(void *opaque, hwaddr offset, unsigned size)
{
    SprdSfcState *s = opaque;
    uint32_t val = 0;

    switch (offset) {
    case SPRD_SFC_CMD:
        val = s->cmd;
        break;

    case SPRD_SFC_ADDR:
        val = s->addr;
        break;

    case SPRD_SFC_DATA:
        if (s->buf_ptr + 4 <= NAND_PAGE_TOTAL_SIZE) {
            val = (uint32_t)s->page_buffer[s->buf_ptr] |
                  ((uint32_t)s->page_buffer[s->buf_ptr + 1] << 8) |
                  ((uint32_t)s->page_buffer[s->buf_ptr + 2] << 16) |
                  ((uint32_t)s->page_buffer[s->buf_ptr + 3] << 24);
            s->buf_ptr += 4;
        } else if (s->buf_ptr < NAND_PAGE_TOTAL_SIZE) {
            val = s->page_buffer[s->buf_ptr++];
        }
        break;

    case SPRD_SFC_CTL:
        val = s->ctl;
        break;

    case SPRD_SFC_STS:
        val = s->sts;
        break;

    case SPRD_SFC_BUF_PTR:
        val = s->buf_ptr;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_sfc: read at unhandled offset 0x%" HWADDR_PRIx "\n",
                      offset);
        break;
    }

    return val;
}

static void sprd_sfc_write(void *opaque, hwaddr offset, uint64_t val, unsigned size)
{
    SprdSfcState *s = opaque;

    switch (offset) {
    case SPRD_SFC_CMD:
        s->cmd = (uint32_t)val;
        sprd_sfc_execute_cmd(s);
        break;

    case SPRD_SFC_ADDR:
        s->addr = (uint32_t)val;
        break;

    case SPRD_SFC_DATA:
        if (s->buf_ptr + 4 <= NAND_PAGE_TOTAL_SIZE) {
            s->page_buffer[s->buf_ptr]     = val & 0xFF;
            s->page_buffer[s->buf_ptr + 1] = (val >> 8) & 0xFF;
            s->page_buffer[s->buf_ptr + 2] = (val >> 16) & 0xFF;
            s->page_buffer[s->buf_ptr + 3] = (val >> 24) & 0xFF;
            s->buf_ptr += 4;
        } else if (s->buf_ptr < NAND_PAGE_TOTAL_SIZE) {
            s->page_buffer[s->buf_ptr++]   = val & 0xFF;
        }
        break;

    case SPRD_SFC_CTL:
        s->ctl = (uint32_t)val;
        break;

    case SPRD_SFC_BUF_PTR:
        s->buf_ptr = (uint32_t)val % NAND_PAGE_TOTAL_SIZE;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                      "sprd_sfc: write at unhandled offset 0x%" HWADDR_PRIx
                      " val 0x%" PRIx64 "\n", offset, val);
        break;
    }
}

static const MemoryRegionOps sprd_sfc_ops = {
    .read = sprd_sfc_read,
    .write = sprd_sfc_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = {
        .min_access_size = 4,
        .max_access_size = 4,
    },
};

static uint64_t sprd_flash_direct_read(void *opaque, hwaddr offset, unsigned size)
{
    SprdSfcState *s = opaque;
    uint32_t val = 0xFFFFFFFF;

    if (offset + size <= SPRD_FLASH_SIZE && s->storage) {
        memcpy(&val, &s->storage[offset], size);
    }

    return val;
}

static void sprd_flash_direct_write(void *opaque, hwaddr offset, uint64_t val, unsigned size)
{
    qemu_log_mask(LOG_GUEST_ERROR,
                  "sprd_sfc: Direct write to read-only flash window at offset 0x%" HWADDR_PRIx "\n",
                  offset);
}

static const MemoryRegionOps sprd_flash_direct_ops = {
    .read = sprd_flash_direct_read,
    .write = sprd_flash_direct_write,
    .endianness = DEVICE_NATIVE_ENDIAN,
    .valid = {
        .min_access_size = 1,
        .max_access_size = 4,
    },
};

static void sprd_sfc_reset(DeviceState *dev)
{
    SprdSfcState *s = SPRD_SFC(dev);

    s->cmd = 0;
    s->addr = 0;
    s->ctl = 0;
    s->sts = 0x02; /* Done / Ready */
    s->buf_ptr = 0;
    s->int_en = 0;
    s->write_enabled = false;
    s->status_reg1 = 0x00;
    s->status_reg2 = 0x18; /* Default ECC enabled, buffer mode */
    s->status_reg3 = 0x00;

    memset(s->page_buffer, 0xFF, sizeof(s->page_buffer));
}

static void sprd_sfc_instance_init(Object *obj)
{
    SprdSfcState *s = SPRD_SFC(obj);
    SysBusDevice *sbd = SYS_BUS_DEVICE(obj);

    s->storage = g_malloc0(SPRD_FLASH_SIZE);
    memset(s->storage, 0xFF, SPRD_FLASH_SIZE);

    memory_region_init_io(&s->iomem, obj, &sprd_sfc_ops, s,
                          "sprd-sfc-regs", SPRD_SFC_REGS_SIZE);
    sysbus_init_mmio(sbd, &s->iomem);

    memory_region_init_io(&s->direct_mem, obj, &sprd_flash_direct_ops, s,
                          "sprd-flash-direct", SPRD_FLASH_SIZE);
    sysbus_init_mmio(sbd, &s->direct_mem);

    sysbus_init_irq(sbd, &s->irq);
}

static void sprd_sfc_class_init(ObjectClass *klass, void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);
    dc->reset = sprd_sfc_reset;
    dc->desc = "Unisoc SPRD Serial Flash Controller (SFC) & SPI NAND";
}

static const TypeInfo sprd_sfc_info = {
    .name          = TYPE_SPRD_SFC,
    .parent        = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(SprdSfcState),
    .instance_init = sprd_sfc_instance_init,
    .class_init    = sprd_sfc_class_init,
};

static void sprd_sfc_register_types(void)
{
    type_register_static(&sprd_sfc_info);
}

type_init(sprd_sfc_register_types)
