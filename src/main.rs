#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Input, Io, Level, Output, Pull, NO_PIN},
    peripherals::Peripherals,
    prelude::*,
    spi::{master::Spi, SpiMode},
    system::SystemControl,
};

use embedded_graphics::{
    geometry::Point, mono_font::MonoTextStyle, text::{Text, TextStyle}, Drawable,
};
use profont::PROFONT_24_POINT;
use epd_waveshare::{epd2in9_v2::*, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

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
    let busy = Input::new(busy, Pull::Down);
    let dc = Output::new(dc, Level::Low);
    let rst = Output::new(rst, Level::High);

    log::info!("Intializing SPI Device...");
    let mut spi_device = ExclusiveDevice::new(spi_bus, cs, delay).expect("SPI device initialize error");

    // Setup EPD
    log::info!("Intializing EPD...");
    let mut epd = Epd2in9::new(&mut spi_device, busy, dc, rst, &mut delay, None).expect("eink initialize error");
    log::info!("Initialized EPD.");

    // Use display graphics from embedded-graphics
    let mut display = Display2in9::default();

    // Write hello world
    let black_style = MonoTextStyle::new(&PROFONT_24_POINT, Color::Black);
    let _ = Text::with_text_style("Hello World!", Point::new(8, 40), black_style, TextStyle::default()).draw(&mut display);


    // Display updated frame
    log::info!("Display...");
    epd.update_frame(&mut spi_device, &display.buffer(), &mut delay).unwrap();
    epd.display_frame(&mut spi_device, &mut delay).unwrap();

    // Sleep for 2 seconds
    delay.delay(2_000.millis());

    // Set the EPD to sleep
    log::info!("Sleeping EPD...");
    epd.sleep(&mut spi_device, &mut delay).unwrap();

    loop {
        log::info!("Hello world!");
        delay.delay(500.millis());
    }
}
