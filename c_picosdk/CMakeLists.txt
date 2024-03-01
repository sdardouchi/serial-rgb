cmake_minimum_required(VERSION 3.13)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(PICO_BOARD waveshare_rp2040_zero)

# initialize the SDK based on PICO_SDK_PATH
# note: this must happen before project()
include(pico_sdk_import.cmake)

project(usb_serial_rp2040 C CXX ASM)

# initialize the Raspberry Pi Pico SDK
pico_sdk_init()

include_directories(usb_serial_rp2040
	include/
)

add_executable(usb_serial_rp2040
	src/main.c
	src/PicoRGB.c
)

# Generate PIO for WS2812B
pico_generate_pio_header(usb_serial_rp2040 ${CMAKE_CURRENT_LIST_DIR}/pio/ws2812.pio)

# Enable USB UART
pico_enable_stdio_usb(usb_serial_rp2040 1)
pico_enable_stdio_uart(usb_serial_rp2040 0)

# Set program name
pico_set_program_name(usb_serial_rp2040 "FuzzingLabs USB Serial Test")
pico_set_program_version(usb_serial_rp2040 "1.0")

# Add pico_stdlib library which aggregates commonly used features
target_link_libraries(usb_serial_rp2040 pico_stdlib hardware_pio)

# create map/bin/hex/uf2 file in addition to ELF.
pico_add_extra_outputs(usb_serial_rp2040)
