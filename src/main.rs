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
};

use embedded_graphics::{
    pixelcolor::BinaryColor::On as Black,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
use epd_waveshare::{epd2in9bc::*, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;

// epaper connections:
// DC: 21, RST: 22, BUSY: 23, CS/SS: 15, SCK: 6, MISO: -1, MOSI: 7

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

    esp_println::logger::init_logger_from_env();

    log::info!("Intializing SPI Bus...");

    let sclk = io.pins.gpio6;
    let mosi = io.pins.gpio7;
    let cs = io.pins.gpio15;
    let dc = io.pins.gpio21;
    let rst = io.pins.gpio22;
    let busy = io.pins.gpio23;

    let mut spi_bus = Spi::new(peripherals.SPI2, 100.kHz(), SpiMode::Mode0, &clocks).with_pins(
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
    let mut spi_device = ExclusiveDevice::new(spi_bus, cs, delay).expect("SPI device initialize error");

    // Setup EPD
    log::info!("Intializing EPD...");
    let mut epd = Epd2in9bc::new(&mut spi_device, busy, dc, rst, &mut delay, None).expect("eink initialize error");
    log::info!("Initialized EPD.");

    // Use display graphics from embedded-graphics
    // This display is for the black/white pixels
    let mut mono_display = Display2in9bc::default();

    // Use embedded graphics for drawing
    // A black line
    let _ = Line::new(Point::new(0, 120), Point::new(0, 200))
        .into_styled(PrimitiveStyle::with_stroke(Black, 1))
        .draw(&mut mono_display.color_converted());

    // Use a second display for red/yellow
    let mut chromatic_display = Display2in9bc::default();

    // We use `Black` but it will be shown as red/yellow
    let _ = Line::new(Point::new(15, 120), Point::new(15, 200))
        .into_styled(PrimitiveStyle::with_stroke(Black, 1))
        .draw(&mut chromatic_display.color_converted());

    // Display updated frame
    epd.update_color_frame(
        &mut spi_device,
        &mut delay,
        &mono_display.buffer(),
        &chromatic_display.buffer()
    ).unwrap();
    log::info!("Display...");
    epd.display_frame(&mut spi_device, &mut delay).unwrap();

    loop {
        log::info!("Hello world!");
        delay.delay(500.millis());
    }
}
