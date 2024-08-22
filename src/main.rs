#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Input, Io, Level, Output, NO_PIN},
    peripherals::Peripherals,
    prelude::*,
    spi::{master::Spi, SpiMode},
    system::SystemControl,
    time
};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_graphics::{
    text::{Alignment, Text, TextStyle, TextStyleBuilder},
    geometry::Point,
    mono_font::MonoTextStyle,
    Drawable,
};
use profont::PROFONT_24_POINT;
use weact_studio_epd::{graphics::Display290BlackWhite, Color};
use weact_studio_epd::{
    graphics::{DisplayBlackWhite, DisplayRotation, buffer_len},
    WeActStudio290BlackWhiteDriver,
};
use display_interface_spi::SPIInterface;
use heapless::String;
use core::fmt::Write;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let clocks = ClockControl::max(system.clock_control).freeze();
    let delay = Delay::new(&clocks);

    esp_println::logger::init_logger_from_env();

    log::info!("Intializing SPI Bus...");

    let sclk = io.pins.gpio19; // D8 / GPIO19
    let mosi = io.pins.gpio18; // D10 / GPIO18
    let cs = io.pins.gpio20; // D9 / GPIO20
    let dc = io.pins.gpio21; // D3 / GPIO21
    let rst = io.pins.gpio22; // D4 / GPIO22
    let busy = io.pins.gpio23; // D5 / GPIO23

    let spi_bus = Spi::new(peripherals.SPI2, 100.kHz(), SpiMode::Mode0, &clocks).with_pins(
        Some(sclk),
        Some(mosi),
        NO_PIN,
        NO_PIN, // cs is handled by the exclusive device?
    );

    // Convert pins into InputPins and OutputPins
    /*
        CS: OutputPin,
        BUSY: InputPin,
        DC: OutputPin,
        RST: OutputPin,
    */
    let cs = Output::new(cs, Level::High);
    let busy = Input::new(busy, esp_hal::gpio::Pull::Up);
    let dc = Output::new(dc, Level::Low);
    let rst = Output::new(rst, Level::High);

    log::info!("Intializing SPI Device...");
    let spi_device = ExclusiveDevice::new(spi_bus, cs, delay).expect("SPI device initialize error");
    let spi_interface = SPIInterface::new(spi_device, dc);

    // Setup EPD
    log::info!("Intializing EPD...");
    let mut driver = WeActStudio290BlackWhiteDriver::new(spi_interface, busy, rst, delay);
    let mut display = Display290BlackWhite::new();
    display.set_rotation(DisplayRotation::Rotate90);

    let mut partial_display_bw =
        DisplayBlackWhite::<64, 128, { buffer_len::<Color>(64, 128) }>::new();
    partial_display_bw.set_rotation(DisplayRotation::Rotate90);

    let mut now = 999;
    driver.init().unwrap();

    let style = MonoTextStyle::new(&PROFONT_24_POINT, Color::Black);

    let mut string_buf = String::<30>::new();
    write!(string_buf, "Time:\nInterval:").unwrap();
    let _ = Text::with_text_style(&string_buf, Point::new(8, 40), style, TextStyle::default())
        .draw(&mut display)
        .unwrap();
    string_buf.clear();

    driver.full_update(&display).unwrap();

    let text_style = TextStyleBuilder::new().alignment(Alignment::Right).build();
    loop {
        let elapsed = now;
        now = elapsed + 1;
        let _ = write!(
            string_buf,
            "{:8.0}ms\n{}ms",
            now,
            elapsed
        );

        let _ = Text::with_text_style(&string_buf, Point::new(128, 32), style, text_style)
            .draw(&mut partial_display_bw);
        string_buf.clear();

        driver
            .fast_partial_update(&partial_display_bw, 56, 156)
            .unwrap();

        partial_display_bw.clear(Color::White);

        if now > 10 {
            break;
        }
    }

    // put the display to sleep
    driver.sleep().unwrap();

    loop {
        continue;
    }
}
