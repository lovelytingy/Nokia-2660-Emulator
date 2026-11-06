/*
 * ARM Boot Tags (ATAGS) Baremetal Driver for Unisoc T107 (UMS9117)
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#ifndef BAREMETAL_ATAGS_H
#define BAREMETAL_ATAGS_H

#include <stdint.h>
#include <stddef.h>

#define ATAG_NONE      0x00000000
#define ATAG_CORE      0x54410001
#define ATAG_MEM       0x54410002
#define ATAG_VIDEOTEXT 0x54410003
#define ATAG_RAMDISK   0x54410004
#define ATAG_INITRD2   0x54420005
#define ATAG_SERIAL    0x54410006
#define ATAG_REVISION  0x54410007
#define ATAG_VIDEOLFB  0x54410008
#define ATAG_CMDLINE   0x54410009

#define T107_DEFAULT_BOARD_ID   0x9117
#define T107_DEFAULT_ATAGS_ADDR 0x80000100

struct tag_header {
    uint32_t size; /* Length of tag in 32-bit words, including header */
    uint32_t tag;  /* Tag ID */
};

struct tag_core {
    uint32_t flags;
    uint32_t pagesize;
    uint32_t rootdev;
};

struct tag_mem32 {
    uint32_t size;
    uint32_t start;
};

struct tag_cmdline {
    char cmdline[1]; /* Variable length null-terminated string */
};

struct tag_initrd2 {
    uint32_t start;
    uint32_t size;
};

struct tag_serialnr {
    uint32_t low;
    uint32_t high;
};

struct tag_revision {
    uint32_t rev;
};

struct tag {
    struct tag_header hdr;
    union {
        struct tag_core     core;
        struct tag_mem32    mem;
        struct tag_cmdline  cmdline;
        struct tag_initrd2  initrd2;
        struct tag_serialnr serialnr;
        struct tag_revision revision;
    } u;
};

#define tag_next(t) ((const struct tag *)((const uint32_t *)(t) + (t)->hdr.size))

/* Validate that tag list begins with ATAG_CORE and ends with ATAG_NONE */
int atags_validate(const struct tag *tags);

/* Find a tag by ID in the list */
const struct tag *atags_find(const struct tag *tags, uint32_t tag_id);

/* Extract primary DRAM base and size */
int atags_get_mem(const struct tag *tags, uint32_t *start, uint32_t *size);

/* Extract kernel command line string */
const char *atags_get_cmdline(const struct tag *tags);

#endif /* BAREMETAL_ATAGS_H */
