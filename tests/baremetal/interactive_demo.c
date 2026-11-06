/*
 * Nokia 2660 Flip Interactive Firmware Demonstration
 * Implements real-time UI, keypad input, audio feedback, flip state handling,
 * and interrupt-driven events.
 *
 * Copyright (c) 2026 N2660Emu Developers
 * SPDX-License-Identifier: MIT
 */

#include <stdint.h>
#include "sprd_uart.h"
#include "gc9307.h"
#include "st7735.h"
#include "kpd.h"
#include "audio.h"
#include "intc.h"
#include "font5x7.h"

typedef enum {
    MODE_HOME = 0,
    MODE_DIALING,
    MODE_CALLING,
    MODE_MENU
} PhoneMode;

static volatile PhoneMode current_mode = MODE_HOME;
static char dial_buffer[32] = {0};
static int dial_len = 0;
static int menu_index = 0;
static int last_flip_state = 1;
static volatile uint32_t key_event_count = 0;
static volatile uint32_t last_s0 = 0;
static volatile uint32_t last_s1 = 0;

#define IS_PRESSED(k) (((k) < 16) ? (last_s0 & (1U << (k))) : (last_s1 & (1U << ((k) - 16))))

const char *menu_items[] = {
    "1. Call Log",
    "2. Contacts",
    "3. Messages",
    "4. Settings",
    "5. Snake Game"
};
#define MENU_ITEM_COUNT 5

static void draw_home_screen(void)
{
    // Deep Nokia blue background
    gc9307_fill_rect(0, 0, GC9307_WIDTH, GC9307_HEIGHT, COLOR_NOKIA_BLUE);

    // Status Bar (Top 24px)
    gc9307_fill_rect(0, 0, GC9307_WIDTH, 24, COLOR_BLACK);
    // Signal bars
    gc9307_fill_rect(8, 16, 3, 5, COLOR_GREEN);
    gc9307_fill_rect(13, 13, 3, 8, COLOR_GREEN);
    gc9307_fill_rect(18, 10, 3, 11, COLOR_GREEN);
    gc9307_fill_rect(23, 7, 3, 14, COLOR_GREEN);
    gc9307_draw_string(32, 8, "Nokia 4G", COLOR_WHITE, COLOR_BLACK, 1);

    // Battery (Top right)
    gc9307_draw_string(170, 8, "85%", COLOR_CYAN, COLOR_BLACK, 1);
    gc9307_fill_rect(198, 7, 24, 12, COLOR_WHITE);
    gc9307_fill_rect(200, 9, 16, 8, COLOR_GREEN);
    gc9307_fill_rect(222, 10, 2, 6, COLOR_WHITE);

    // Digital Clock
    gc9307_draw_string(72, 60, "12:34", COLOR_WHITE, COLOR_NOKIA_BLUE, 4);
    gc9307_draw_string(60, 105, "Friday 06 Nov", COLOR_CYAN, COLOR_NOKIA_BLUE, 2);

    // Brand Banner
    gc9307_fill_rect(20, 150, 200, 2, COLOR_CYAN);
    gc9307_draw_string(45, 165, "NOKIA 2660 FLIP", COLOR_WHITE, COLOR_NOKIA_BLUE, 2);
    gc9307_draw_string(50, 190, "Model TA-1480", COLOR_YELLOW, COLOR_NOKIA_BLUE, 2);
    gc9307_fill_rect(20, 215, 200, 2, COLOR_CYAN);

    gc9307_draw_string(30, 240, "Press any key to dial", COLOR_WHITE, COLOR_NOKIA_BLUE, 1);

    // Softkey Bar (Bottom 24px)
    gc9307_fill_rect(0, 296, GC9307_WIDTH, 24, COLOR_BLACK);
    gc9307_draw_string(10, 302, "Menu", COLOR_CYAN, COLOR_BLACK, 1);
    gc9307_draw_string(98, 302, "Select", COLOR_WHITE, COLOR_BLACK, 1);
    gc9307_draw_string(180, 302, "Names", COLOR_CYAN, COLOR_BLACK, 1);
}

static void draw_dialing_screen(void)
{
    gc9307_fill_rect(0, 0, GC9307_WIDTH, GC9307_HEIGHT, COLOR_BLACK);

    // Header
    gc9307_draw_string(10, 10, "Dialing Number", COLOR_YELLOW, COLOR_BLACK, 2);
    gc9307_fill_rect(0, 35, GC9307_WIDTH, 2, COLOR_YELLOW);

    // Large Dialed Number
    gc9307_fill_rect(10, 80, 220, 60, 0x18E3); // Dark slate box
    gc9307_draw_string(20, 95, dial_buffer, COLOR_WHITE, 0x18E3, 3);

    gc9307_draw_string(20, 170, "[CALL] to Call", COLOR_GREEN, COLOR_BLACK, 2);
    gc9307_draw_string(20, 200, "[END]  to Clear", COLOR_RED, COLOR_BLACK, 2);

    // Softkeys
    gc9307_fill_rect(0, 296, GC9307_WIDTH, 24, 0x18E3);
    gc9307_draw_string(10, 302, "Call", COLOR_GREEN, 0x18E3, 1);
    gc9307_draw_string(180, 302, "Clear", COLOR_RED, 0x18E3, 1);
}

static void draw_calling_screen(void)
{
    gc9307_fill_rect(0, 0, GC9307_WIDTH, GC9307_HEIGHT, 0x01E0); // Forest green

    gc9307_draw_string(30, 40, "CONNECTED", COLOR_WHITE, 0x01E0, 3);
    gc9307_draw_string(20, 100, "Calling:", COLOR_YELLOW, 0x01E0, 2);
    gc9307_draw_string(20, 130, dial_buffer, COLOR_WHITE, 0x01E0, 3);

    gc9307_draw_string(50, 190, "Duration: 00:07", COLOR_WHITE, 0x01E0, 2);
    gc9307_draw_string(35, 240, "Press END to hang up", COLOR_YELLOW, 0x01E0, 1);

    // Softkey
    gc9307_fill_rect(0, 296, GC9307_WIDTH, 24, COLOR_BLACK);
    gc9307_draw_string(180, 302, "End", COLOR_RED, COLOR_BLACK, 1);
}

static void draw_menu_screen(void)
{
    gc9307_fill_rect(0, 0, GC9307_WIDTH, GC9307_HEIGHT, 0x10A2);

    gc9307_draw_string(15, 10, "MAIN MENU", COLOR_YELLOW, 0x10A2, 2);
    gc9307_fill_rect(0, 32, GC9307_WIDTH, 2, COLOR_YELLOW);

    for (int i = 0; i < MENU_ITEM_COUNT; i++) {
        int y = 50 + i * 45;
        if (i == menu_index) {
            gc9307_fill_rect(10, y, 220, 35, COLOR_YELLOW);
            gc9307_draw_string(20, y + 10, menu_items[i], COLOR_BLACK, COLOR_YELLOW, 2);
        } else {
            gc9307_fill_rect(10, y, 220, 35, 0x2124);
            gc9307_draw_string(20, y + 10, menu_items[i], COLOR_WHITE, 0x2124, 2);
        }
    }

    gc9307_fill_rect(0, 296, GC9307_WIDTH, 24, COLOR_BLACK);
    gc9307_draw_string(10, 302, "Select", COLOR_CYAN, COLOR_BLACK, 1);
    gc9307_draw_string(180, 302, "Back", COLOR_CYAN, COLOR_BLACK, 1);
}

static void draw_sub_screen(void)
{
    // Outer 120x160 Display
    st7735_fill_rect(0, 0, ST7735_WIDTH, ST7735_HEIGHT, COLOR_SUB_BLACK);

    // Sub Status Bar
    st7735_fill_rect(5, 5, 12, 6, COLOR_SUB_GREEN);
    st7735_draw_string(22, 5, "Nokia", COLOR_SUB_WHITE, COLOR_SUB_BLACK, 1);
    st7735_draw_string(88, 5, "85%", COLOR_SUB_CYAN, COLOR_SUB_BLACK, 1);

    // Big Clock
    st7735_draw_string(15, 45, "12:34", COLOR_SUB_WHITE, COLOR_SUB_BLACK, 3);
    st7735_draw_string(20, 80, "Fri 06 Nov", COLOR_SUB_CYAN, COLOR_SUB_BLACK, 1);

    st7735_fill_rect(10, 105, 100, 1, COLOR_SUB_DARK_GREY);
    st7735_draw_string(25, 120, "FLIP CLOSED", COLOR_SUB_YELLOW, COLOR_SUB_BLACK, 1);
}

static void kpd_isr_handler(void)
{
    uint32_t s0 = 0, s1 = 0;
    kpd_read_status(&s0, &s1);
    kpd_clear_interrupt();

    if (s0 != 0 || s1 != 0) {
        last_s0 = s0;
        last_s1 = s1;
        key_event_count++;
    }
}

int main(void)
{
    uart_init();
    uart_puts("\n========================================================\n");
    uart_puts("   Nokia 2660 Flip (TA-1480) Interactive OS Shell        \n");
    uart_puts("========================================================\n");

    // Initialize display hardware
    gc9307_init();
    st7735_init();
    audio_init(1); // Stereo mode

    // Initialize Interrupts
    intc_init();
    kpd_init();
    kpd_enable_interrupt();
    intc_register_handler(SPRD_IRQ_KPD, kpd_isr_handler);
    intc_enable_irq(SPRD_IRQ_KPD);

    // Enable IRQs in CPU CPSR
    cpu_enable_irqs();

    uart_puts("[OS] Peripherals, INTC and Keypad IRQs initialized.\n");
    uart_puts("[OS] Entering interactive event loop...\n");

    draw_home_screen();
    draw_sub_screen();

    uint32_t prev_key_event = 0;

    while (1) {
        // 1. Check Flip Sensor State
        int flip = kpd_read_flip_sensor();
        if (flip != last_flip_state) {
            last_flip_state = flip;
            if (flip == 0) {
                uart_puts("[OS] Flip Closed: outer display active.\n");
                draw_sub_screen();
            } else {
                uart_puts("[OS] Flip Opened: main display active.\n");
                if (current_mode == MODE_HOME) draw_home_screen();
                else if (current_mode == MODE_DIALING) draw_dialing_screen();
                else if (current_mode == MODE_CALLING) draw_calling_screen();
                else if (current_mode == MODE_MENU) draw_menu_screen();
            }
        }

        // 2. Process Keypad Events
        if (key_event_count != prev_key_event) {
            prev_key_event = key_event_count;

            audio_play_tone(1200, 20, 80); // Key click tone

            // Check numeric keys 0-9
            char typed_char = 0;
            if (IS_PRESSED(KEY_0)) typed_char = '0';
            else if (IS_PRESSED(KEY_1)) typed_char = '1';
            else if (IS_PRESSED(KEY_2)) typed_char = '2';
            else if (IS_PRESSED(KEY_3)) typed_char = '3';
            else if (IS_PRESSED(KEY_4)) typed_char = '4';
            else if (IS_PRESSED(KEY_5)) typed_char = '5';
            else if (IS_PRESSED(KEY_6)) typed_char = '6';
            else if (IS_PRESSED(KEY_7)) typed_char = '7';
            else if (IS_PRESSED(KEY_8)) typed_char = '8';
            else if (IS_PRESSED(KEY_9)) typed_char = '9';
            else if (IS_PRESSED(KEY_STAR)) typed_char = '*';
            else if (IS_PRESSED(KEY_HASH)) typed_char = '#';

            if (typed_char) {
                if (dial_len < 15) {
                    dial_buffer[dial_len++] = typed_char;
                    dial_buffer[dial_len] = '\0';
                }
                current_mode = MODE_DIALING;
                uart_puts("[OS] Key Pressed: '");
                uart_putc(typed_char);
                uart_puts("' | Number: ");
                uart_puts(dial_buffer);
                uart_puts("\n");
                draw_dialing_screen();
            }
            // CALL Key
            else if (IS_PRESSED(KEY_CALL)) {
                if (dial_len > 0) {
                    current_mode = MODE_CALLING;
                    audio_play_tone(440, 200, 85);
                    uart_puts("[OS] CALL button: connecting call to ");
                    uart_puts(dial_buffer);
                    uart_puts("\n");
                    draw_calling_screen();
                }
            }
            // END Key
            else if (IS_PRESSED(KEY_END)) {
                current_mode = MODE_HOME;
                dial_len = 0;
                dial_buffer[0] = '\0';
                audio_play_tone(350, 60, 80);
                uart_puts("[OS] END button: call ended, returning to Home\n");
                draw_home_screen();
            }
            // LSK (Menu)
            else if (IS_PRESSED(KEY_LSK)) {
                if (current_mode == MODE_HOME) {
                    current_mode = MODE_MENU;
                    uart_puts("[OS] Entering Main Menu\n");
                    draw_menu_screen();
                }
            }
            // RSK (Back / Clear)
            else if (IS_PRESSED(KEY_RSK)) {
                if (current_mode == MODE_MENU) {
                    current_mode = MODE_HOME;
                    draw_home_screen();
                } else if (current_mode == MODE_DIALING) {
                    if (dial_len > 0) {
                        dial_buffer[--dial_len] = '\0';
                        if (dial_len == 0) {
                            current_mode = MODE_HOME;
                            draw_home_screen();
                        } else {
                            draw_dialing_screen();
                        }
                    }
                }
            }
            // Navigation Keys
            else if (IS_PRESSED(KEY_UP)) {
                if (current_mode == MODE_MENU) {
                    menu_index = (menu_index - 1 + MENU_ITEM_COUNT) % MENU_ITEM_COUNT;
                    draw_menu_screen();
                }
            }
            else if (IS_PRESSED(KEY_DOWN)) {
                if (current_mode == MODE_MENU) {
                    menu_index = (menu_index + 1) % MENU_ITEM_COUNT;
                    draw_menu_screen();
                }
            }
        }
    }

    return 0;
}
