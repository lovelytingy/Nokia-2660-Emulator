/*
 * ARM Boot Tags (ATAGS) Baremetal Driver for Unisoc T107 (UMS9117)
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include "atags.h"
#include "sprd_uart.h"

int atags_validate(const struct tag *tags)
{
    if (!tags) {
        return -1;
    }

    /* First tag MUST be ATAG_CORE */
    if (tags->hdr.tag != ATAG_CORE) {
        return -2;
    }

    const struct tag *t = tags;
    size_t count = 0;
    while (t->hdr.size > 0 && count < 64) {
        if (t->hdr.tag == ATAG_NONE) {
            return 0;
        }
        t = tag_next(t);
        count++;
    }

    /* Check if terminator ATAG_NONE was reached */
    if (t->hdr.tag == ATAG_NONE || t->hdr.size == 0) {
        return 0;
    }

    return -3;
}

const struct tag *atags_find(const struct tag *tags, uint32_t tag_id)
{
    if (!tags || tags->hdr.tag != ATAG_CORE) {
        return NULL;
    }

    const struct tag *t = tags;
    size_t count = 0;
    while (t->hdr.size > 0 && count < 64) {
        if (t->hdr.tag == tag_id) {
            return t;
        }
        if (t->hdr.tag == ATAG_NONE) {
            break;
        }
        t = tag_next(t);
        count++;
    }

    return NULL;
}

int atags_get_mem(const struct tag *tags, uint32_t *start, uint32_t *size)
{
    const struct tag *mem_tag = atags_find(tags, ATAG_MEM);
    if (!mem_tag) {
        return -1;
    }

    if (start) {
        *start = mem_tag->u.mem.start;
    }
    if (size) {
        *size = mem_tag->u.mem.size;
    }

    return 0;
}

const char *atags_get_cmdline(const struct tag *tags)
{
    const struct tag *cmd_tag = atags_find(tags, ATAG_CMDLINE);
    if (!cmd_tag) {
        return NULL;
    }

    return cmd_tag->u.cmdline.cmdline;
}
