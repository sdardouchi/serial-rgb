#![no_std]
#![no_main]

extern crate alloc;
use embedded_alloc::Heap;
use alloc::{
    format, 
    string::{
        ToString,
        String,
    },
    borrow::ToOwned
};

use panic_halt as _;
use core::iter::once;
use waveshare_rp2040_zero::{
    hal::{
        pac,
        timer::Timer,
        watchdog::Watchdog,
        pio::PIOExt,
        usb::UsbBus,
        clocks::{init_clocks_and_plls, Clock},
        Sio, 
    },
    entry, Pins, XOSC_CRYSTAL_FREQ
};
use embedded_hal::timer::CountDown;

use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

use fugit::ExtU32;
use nb;

use smart_leds::{brightness, SmartLedsWrite, RGB8};
use ws2812_pio::Ws2812;

use strtoint::strtoint;

pub fn decode_hex(s: &str) -> [u8;  3] {
    let mut result = [0u8;  3];
    let mut index =  0;

    for chunk in s.as_bytes().chunks(2) {
        let byte = u8::from_str_radix(core::str::from_utf8(chunk).unwrap(),  16).unwrap();
        result[index] = byte;
        index +=  1;
    }

    result
}

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn main() -> ! {
    //Initialize alloc
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE)
        }
    }
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut delay = timer.count_down();

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Set up the USB driver
    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    // Set up the USB Communications Class Device driver
    let mut serial = SerialPort::new(&usb_bus);
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut ws = Ws2812::new(
        // The onboard NeoPixel is attached to GPIO pin #16 on the Feather RP2040.
        pins.neopixel.into_function(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );

    // Create a USB device with a fake VID and PID
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x2e8a, 0x000a))
        .manufacturer("sdardouchi")
        .product("Serial RGB")
        .serial_number("TEST")
        .device_class(2) // from: https://www.usb.org/defined-class-codes
        .build();

    delay.start(2000.millis());
    let _ = nb::block!(delay.wait());

    loop {
        if usb_dev.poll(&mut [&mut serial]) {
            let mut buf = [0u8; 7];
            match serial.read(&mut buf) { 
                Err(_e) => {},
                Ok(0) => {},
                Ok(count) => {
                    if count == 6 {
                        let col = input_to_rgb8(buf, &mut serial);
                        ws.write(brightness(once(col), 255)).unwrap();
                    }
                }
            }
        }
    }
}

fn input_to_rgb8(buf: [u8; 7], serial: &mut SerialPort<UsbBus>) -> RGB8 {
    let r_slice = buf.get(0..2).unwrap();
    let g_slice = buf.get(2..4).unwrap();
    let b_slice = buf.get(4..6).unwrap();

    let mut r_str = "0x".to_string();
    r_str.push_str(
        &String::from_utf8(r_slice.to_owned())
        .unwrap()
    );
    
    
    let mut g_str = "0x".to_string();
    g_str.push_str(
        &String::from_utf8(g_slice.to_owned())
        .unwrap()
    );
    
    let mut b_str = "0x".to_string();
    b_str.push_str(
        &String::from_utf8(b_slice.to_owned())
        .unwrap()
    );

    let (r, g, b) = (
        strtoint::<u8>(r_str.as_str()).unwrap(),
        strtoint::<u8>(g_str.as_str()).unwrap(),
        strtoint::<u8>(b_str.as_str()).unwrap(),
    );
    
    let fmt = format!("R: {}; G: {}; B: {}\n", r, g, b);
    let _ = serial.write(fmt.as_bytes());
    return RGB8 { r: r, g: g, b: b }
}
