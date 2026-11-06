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

SRCS_C  := main.c sprd_uart.c gc9307.c
SRCS_S  := startup.s

OBJS    := $(SRCS_C:.c=.o) $(SRCS_S:.s=.o)

TARGET  := test_firmware

all: $(TARGET).elf $(TARGET).bin $(TARGET).dis

$(TARGET).elf: $(OBJS) linker.ld
	$(CC) $(CFLAGS) $(OBJS) $(LDFLAGS) -o $@

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.s
	$(CC) $(CFLAGS) -c $< -o $@

$(TARGET).bin: $(TARGET).elf
	$(OBJCOPY) -O binary $< $@

$(TARGET).dis: $(TARGET).elf
	$(OBJDUMP) -d -S $< > $@

clean:
	rm -f $(OBJS) $(TARGET).elf $(TARGET).bin $(TARGET).dis

.PHONY: all clean
