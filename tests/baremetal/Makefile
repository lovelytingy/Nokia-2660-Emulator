# Makefile for N2660Emu baremetal verification test payload

CROSS_COMPILE ?= arm-none-eabi-
CC      := $(CROSS_COMPILE)gcc
AS      := $(CROSS_COMPILE)as
LD      := $(CROSS_COMPILE)ld
OBJCOPY := $(CROSS_COMPILE)objcopy
OBJDUMP := $(CROSS_COMPILE)objdump

CFLAGS  := -mcpu=cortex-a7 -marm -mfpu=vfpv4 -mfloat-abi=hard \
           -O2 -Wall -Wextra -ffreestanding -nostdlib -I.
LDFLAGS := -T linker.ld -nostdlib

SRCS_COMMON := sprd_uart.c gc9307.c kpd.c st7735.c sfc.c bsl.c audio.c bootrom.c atags.c intc.c font5x7.c
OBJS_COMMON := $(SRCS_COMMON:.c=.o) startup.o

TARGET_TEST := test_firmware
TARGET_DEMO := interactive_demo

all: $(TARGET_TEST).bin $(TARGET_DEMO).bin $(TARGET_TEST).dis $(TARGET_DEMO).dis

$(TARGET_TEST).elf: main.o $(OBJS_COMMON) linker.ld
	$(CC) $(CFLAGS) main.o $(OBJS_COMMON) $(LDFLAGS) -o $@

$(TARGET_DEMO).elf: interactive_demo.o $(OBJS_COMMON) linker.ld
	$(CC) $(CFLAGS) interactive_demo.o $(OBJS_COMMON) $(LDFLAGS) -o $@

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.s
	$(CC) $(CFLAGS) -c $< -o $@

%.bin: %.elf
	$(OBJCOPY) -O binary $< $@

%.dis: %.elf
	$(OBJDUMP) -d -S $< > $@

clean:
	rm -f *.o $(TARGET_TEST).elf $(TARGET_TEST).bin $(TARGET_TEST).dis $(TARGET_DEMO).elf $(TARGET_DEMO).bin $(TARGET_DEMO).dis

.PHONY: all clean
