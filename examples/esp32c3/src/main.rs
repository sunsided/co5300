//! Draws a test pattern on a CO5300 over 4-wire SPI, with the frame memory written directly
//! through the `embedded-graphics` target.
//!
//! Wiring: SCL = GPIO4, SDI = GPIO5, CSX = GPIO6, DCX = GPIO7, RESX = GPIO10.
//! The panel is 410 × 502, as on the Kode Dot.

#![no_std]
#![no_main]

use co5300::{Co5300, Config, SpiInterface};
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_graphics_core::geometry::{Point, Size};
use embedded_graphics_core::pixelcolor::{Rgb565, RgbColor};
use embedded_graphics_core::primitives::Rectangle;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::main;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::time::Rate;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

const WIDTH: u16 = 410;
const HEIGHT: u16 = 502;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(Mode::_0),
    )
    .expect("valid SPI config")
    .with_sck(peripherals.GPIO4)
    .with_mosi(peripherals.GPIO5);
    let cs = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default());
    let mut rst = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let spi = ExclusiveDevice::new_no_delay(spi, cs).unwrap();

    let mut display = Co5300::new(SpiInterface::new(spi, dc), Config::new(WIDTH, HEIGHT));
    co5300::reset(&mut rst, &mut delay).unwrap();
    display.init(&mut delay, &[]).expect("CO5300 init");
    println!("CO5300 init ok");

    display.clear(Rgb565::BLACK).unwrap();

    // Four color bars on even coordinates, so the 2-pixel grid draws them exactly.
    let bars = [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE, Rgb565::WHITE];
    for (i, color) in (0..).zip(bars) {
        let bar = Rectangle::new(Point::new(20 + i * 96, 40), Size::new(80, 200));
        display.fill_solid(&bar, color).unwrap();
    }

    // A red-to-blue gradient, streamed pixel by pixel.
    let gradient = Rectangle::new(Point::new(20, 280), Size::new(370, 64));
    let colors = (0..64u32).flat_map(|y| {
        (0..370u32).map(move |x| {
            let red = (31 * (369 - x) / 369) as u8;
            let blue = (31 * x / 369) as u8;
            Rgb565::new(red, y as u8, blue)
        })
    });
    display.fill_contiguous(&gradient, colors).unwrap();

    println!("CO5300 frame drawn");

    loop {
        delay.delay_millis(1000);
    }
}
