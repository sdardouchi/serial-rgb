#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#include "pico/stdlib.h"
#include "pico/error.h"
#include <tusb.h>

#include <PicoRGB.h>

#define WRITE_INTERVAL (2000)
#define MAX_LEN	(1024)

void clear_string(char *str) {
	int i = 0;
	while (str[i] != 0) {
		str[i] = 0;
		i += 1;
	}
}

int main() {
	stdio_init_all();
	strip_init();

	strip_set_led_color(0, 0, 0x00, 0);
	strip_update();
	
	while (!tud_cdc_connected()) {
		sleep_ms(100);
	}

	char buffer[MAX_LEN];
	char color[7];
	uint8_t r = 0, g = 0, b = 0;
	uint32_t int_color = 0;

	for (;;) {
		if (tud_cdc_available()) {
			tud_cdc_read(buffer, MAX_LEN);
			tud_cdc_read_flush();
			
			strncpy(color, buffer, 7);

			int_color = (uint32_t)strtol(color, NULL, 16);
			clear_string(color);

			b = int_color & 0xFF;
			g = int_color >> 8 & 0xFF;
			r = int_color >> 16 & 0xFF;
			
			strip_set_led_color(0, r, g, b);
			strip_update();
		}
	}
	return 0;
}
