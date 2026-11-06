# Makefile for N2660Emu baremetal verification test payload
# SPDX-License-Identifier: MIT

CROSS_COMPILE ?= arm-none-eabi-
CC      := $(CROSS_COMPILE)gcc
AS      := $(CROSS_COMPILE)as
LD      := $(CROSS_COMPILE)ld
OBJCOPY := $(CROSS_COMPILE)objcopy
OBJDUMP := $(CROSS_COMPILE)objdump

CFLAGS  := -mcpu=cortex-a7 -marm -mfpu=vfpv4 -mfloat-abi=hard \
           -O2 -Wall -Wextra -ffreestanding -nostdlib -I.
LDFLAGS := -T linker.ld -nostdlib

BUILD_DIR := build
SRCS_COMMON := sprd_uart.c gc9307.c kpd.c st7735.c sfc.c bsl.c audio.c bootrom.c atags.c intc.c font5x7.c
OBJS_COMMON := $(addprefix $(BUILD_DIR)/, $(SRCS_COMMON:.c=.o) startup.o)

TARGET_TEST := test_firmware
TARGET_DEMO := interactive_demo

all: $(BUILD_DIR) $(TARGET_TEST).bin $(TARGET_DEMO).bin $(BUILD_DIR)/$(TARGET_TEST).dis $(BUILD_DIR)/$(TARGET_DEMO).dis

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/$(TARGET_TEST).elf: $(BUILD_DIR)/main.o $(OBJS_COMMON) linker.ld | $(BUILD_DIR)
	$(CC) $(CFLAGS) $(BUILD_DIR)/main.o $(OBJS_COMMON) $(LDFLAGS) -o $@

$(BUILD_DIR)/$(TARGET_DEMO).elf: $(BUILD_DIR)/interactive_demo.o $(OBJS_COMMON) linker.ld | $(BUILD_DIR)
	$(CC) $(CFLAGS) $(BUILD_DIR)/interactive_demo.o $(OBJS_COMMON) $(LDFLAGS) -o $@

$(BUILD_DIR)/%.o: %.c | $(BUILD_DIR)
	$(CC) $(CFLAGS) -c $< -o $@

$(BUILD_DIR)/%.o: %.s | $(BUILD_DIR)
	$(CC) $(CFLAGS) -c $< -o $@

%.bin: $(BUILD_DIR)/%.elf
	$(OBJCOPY) -O binary $< $@

$(BUILD_DIR)/%.dis: $(BUILD_DIR)/%.elf
	$(OBJDUMP) -d -S $< > $@

clean:
	rm -rf $(BUILD_DIR) $(TARGET_TEST).bin $(TARGET_DEMO).bin *.o *.elf *.dis

.PHONY: all clean
