/*
 * Baremetal UART driver implementation for N2660Emu
 * Supports Unisoc SPRD UART (0x50000000) and QEMU PL011 (0x09000000).
 */

#include "sprd_uart.h"

#define REG32(addr) (*(volatile uint32_t *)(addr))

static int g_use_sprd = 1;

void uart_init(void)
{
    /* In emulation, UART controllers are ready upon reset. */
    g_use_sprd = 1;
}

void uart_putc(char c)
{
    if (c == '\n') {
        uart_putc('\r');
    }

    /* 1. Write to Unisoc SPRD UART (Base 0x50000000) */
    /* Check TX FIFO empty / room available */
    volatile uint32_t *sprd_txd = (volatile uint32_t *)(SPRD_UART0_BASE + SPRD_TXD_REG);
    *sprd_txd = (uint32_t)(uint8_t)c;

    /* 2. Also mirror to standard ARM PL011 UART (Base 0x09000000) for virt fallback */
    volatile uint32_t *pl011_dr = (volatile uint32_t *)(PL011_UART_BASE + PL011_DR);
    *pl011_dr = (uint32_t)(uint8_t)c;
}

void uart_puts(const char *str)
{
    while (*str) {
        uart_putc(*str++);
    }
}

static const char hex_digits[] = "0123456789ABCDEF";

void uart_puthex32(uint32_t val)
{
    uart_puts("0x");
    for (int i = 28; i >= 0; i -= 4) {
        uart_putc(hex_digits[(val >> i) & 0xF]);
    }
}

void uart_putdec(uint32_t val)
{
    char buf[12];
    int idx = 0;

    if (val == 0) {
        uart_putc('0');
        return;
    }

    while (val > 0) {
        buf[idx++] = '0' + (val % 10);
        val /= 10;
    }

    for (int i = idx - 1; i >= 0; i--) {
        uart_putc(buf[i]);
    }
}
