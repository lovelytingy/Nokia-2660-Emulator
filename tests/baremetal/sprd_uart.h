/*
 * Baremetal UART driver interface for N2660Emu
 */

#ifndef TEST_SPRD_UART_H
#define TEST_SPRD_UART_H

#include <stdint.h>

#define SPRD_UART0_BASE   0x50000000
#define SPRD_TXD_REG      0x0000
#define SPRD_RXD_REG      0x0004
#define SPRD_STS1_REG     0x000C

/* Fallback PL011 UART for generic QEMU virt testing */
#define PL011_UART_BASE   0x09000000
#define PL011_DR          0x0000
#define PL011_FR          0x0018

void uart_init(void);
void uart_putc(char c);
void uart_puts(const char *str);
void uart_puthex32(uint32_t val);
void uart_putdec(uint32_t val);

#endif /* TEST_SPRD_UART_H */
