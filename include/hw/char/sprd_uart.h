/*
 * Unisoc / Spreadtrum UART Emulation
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: GPL-2.0-or-later
 */

#ifndef HW_CHAR_SPRD_UART_H
#define HW_CHAR_SPRD_UART_H

#include "qemu/osdep.h"
#include "hw/sysbus.h"
#include "chardev/char-fe.h"
#include "qom/object.h"

#define TYPE_SPRD_UART "sprd-uart"
OBJECT_DECLARE_SIMPLE_TYPE(SprdUartState, SPRD_UART)

/* Register Offsets */
#define SPRD_TXD        0x0000  /* Transmit Data Register */
#define SPRD_RXD        0x0004  /* Receive Data Register */
#define SPRD_STS0       0x0008  /* Status Register 0 */
#define SPRD_STS1       0x000C  /* Status Register 1 (FIFO count & flags) */
#define SPRD_IEN        0x0010  /* Interrupt Enable Register */
#define SPRD_ICLR       0x0014  /* Interrupt Clear Register */
#define SPRD_CTL0       0x0018  /* Control Register 0 */
#define SPRD_CTL1       0x001C  /* Control Register 1 */
#define SPRD_CTL2       0x0020  /* Control Register 2 */
#define SPRD_CLKD0      0x0024  /* Clock Divider 0 */
#define SPRD_CLKD1      0x0028  /* Clock Divider 1 */

#define SPRD_UART_REGS_SIZE 0x1000

/* Status Bits */
#define SPRD_STS1_TX_FIFO_EMPTY (1 << 8)
#define SPRD_STS1_TX_FIFO_FULL  (1 << 9)
#define SPRD_STS1_RX_FIFO_EMPTY (1 << 10)
#define SPRD_STS1_RX_FIFO_FULL  (1 << 11)

struct SprdUartState {
    /*< private >*/
    SysBusDevice parent_obj;

    /*< public >*/
    MemoryRegion iomem;
    CharBackend chr;
    qemu_irq irq;

    uint32_t sts0;
    uint32_t sts1;
    uint32_t ien;
    uint32_t ctl0;
    uint32_t ctl1;
    uint32_t ctl2;
    uint32_t clkd0;
    uint32_t clkd1;

    uint8_t rx_fifo[64];
    uint32_t rx_count;
    uint32_t rx_head;
    uint32_t rx_tail;
};

DeviceState *sprd_uart_create(hwaddr addr, qemu_irq irq, Chardev *chr);

#endif /* HW_CHAR_SPRD_UART_H */
