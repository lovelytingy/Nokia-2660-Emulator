# Makefile for Unisoc T107 Clean-Room BootROM

CROSS_COMPILE ?= arm-none-eabi-
CC      := $(CROSS_COMPILE)gcc
AS      := $(CROSS_COMPILE)as
LD      := $(CROSS_COMPILE)ld
OBJCOPY := $(CROSS_COMPILE)objcopy
OBJDUMP := $(CROSS_COMPILE)objdump

CFLAGS  := -mcpu=cortex-a7 -marm -mfpu=vfpv4 -mfloat-abi=hard \
           -O2 -Wall -Wextra -ffreestanding -nostdlib
LDFLAGS := -T bootrom.ld -nostdlib

TARGET  := bootrom

all: $(TARGET).elf $(TARGET).bin $(TARGET).dis

$(TARGET).elf: bootrom.o bootrom.ld
	$(CC) $(CFLAGS) bootrom.o $(LDFLAGS) -o $@

bootrom.o: bootrom.s
	$(CC) $(CFLAGS) -c $< -o $@

$(TARGET).bin: $(TARGET).elf
	$(OBJCOPY) -O binary $< $@

$(TARGET).dis: $(TARGET).elf
	$(OBJDUMP) -d -S $< > $@

clean:
	rm -f bootrom.o $(TARGET).elf $(TARGET).bin $(TARGET).dis

.PHONY: all clean
