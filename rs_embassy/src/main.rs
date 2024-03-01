//! This example shows powerful PIO module in the RP2040 chip to communicate with WS2812 LED modules.
//! See (https://www.sparkfun.com/categories/tags/ws2812)

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::pio::Pio;
use embassy_futures::join::join;

use smart_leds::RGB8;
use {defmt_rtt as _, panic_probe as _};

mod ws2812;
use crate::ws2812::Ws2812;

use embassy_rp::{
    bind_interrupts,
    pio::InterruptHandler,
    peripherals::{
        PIO0,
        USB
    },
    usb::{
        Driver,
        Instance,
    }
};

use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    driver::EndpointError,
    Builder, Config
};


bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

fn hex_char_to_int(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c as u8 - b'0'),
        b'a'..=b'f' => Some(c as u8 - b'a' + 10),
        b'A'..=b'F' => Some(c as u8 - b'A' + 10),
        _ => None,
    }
}

fn parse_hex(hex: &[u8]) -> Option<u8> {
    let mut result: u8 = 0;
    for c in hex.iter().rev() {
        let value = hex_char_to_int(*c)?;
        result = result * 16 + value;
    }
    Some(result)
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Create the driver, from the HAL.
    let driver = Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("sdardouchi");
    config.product = Some("Serial RGB");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Required for windows compatibility.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut device_descriptor = [0; 256];
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no msos descriptors
        &mut control_buf,
    );

    // Create classes on the builder.
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let mut ws2812 = Ws2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16);
    
    let get_color = async{
        loop {
            class.wait_connection().await;
            match serial_color(&mut class).await {
                Ok(col) => { 
                    ws2812.write(&[col]).await;
                }
                Err(_) => {
                    let _ = class.write_packet(b"Didn't find color sorry !").await;
                }
            }
                
            
        }
    };

    join(usb_fut, get_color).await;
}

#[derive(Debug)]
struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => core::panic!("Buffer overflow !"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn serial_color<'d, T: Instance + 'd>(class: &mut CdcAcmClass<'d, Driver<'d, T>>) -> Result<RGB8, Disconnected> {
    let mut buf = [0; 64];
    let n = class.read_packet(&mut buf).await?;
    
    if n != 6 {
        return Err(Disconnected{});
    }

    let r_slice = buf.get(0..2).unwrap();
    let g_slice = buf.get(2..4).unwrap();
    let b_slice = buf.get(4..6).unwrap();

    let (r, g, b) = (
        parse_hex(r_slice).unwrap(),
        parse_hex(g_slice).unwrap(),
        parse_hex(b_slice).unwrap()
    );
    
    Ok(RGB8{ r:r, g:g, b:b })
}
