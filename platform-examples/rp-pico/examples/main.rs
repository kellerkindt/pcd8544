//! Displays stuff on a Philips PCD8544 driven Nokia 5110/3310 screen
//! This example is for the Raspberry Pico with the rp2040 chip
//!
//! Pin layout and connection for this example
//!
//! display | Pico pin |  Pico function       | purpose
//!  1 RST     pin 9      GP6 (gpio6)           reset: active low to reset display
//!  2 CE      pin 10     GP7 (gpio7)           Chip Enable: active low allowes data  
//!  3 DC      pin 11     GP8 (gpio8)           Data/Command (1 = Data, 0 = Command)
//!  4 DIN     pin 5      GP3 (gpio3)           Serial data line 
//!  5 CLK     pin 4      GP2 (gpio2)           Serial clock, should be in range 0 - 4.0 Mbit/s
//!  6 VCC     pin 36     3V3(OUT)              Power to the display (lcd and chip) and leds
//!  7 LIGHT   pin 27     GP21 (gpio21)         turn on/off backlights, can be PWM-ed
//!  8 GND     pin 38     GND                   Ground for display (lcd and chip) but not leds
//!
//! This particular example was made for, and tested on a no-name display with a red pcb, 
//! probably direcltly imported from China.
//! https://hackerstore.nl/Artikel/78
//! It has blue backlights and has a LIGHT pin which need to be LOW to put backlight ON.
//! 
//! PLEASE: Change backlight code to reflect the setup on your particulair PCD8544 board!!
//! Adafruit and Sparkfun use LIGHT pin HIGH to turn backlight ON
//! See README for more info

#![deny(unsafe_code)]
#![deny(warnings)]
#![no_std]
#![no_main]

// Imports

use panic_halt as _;                        // well. halt on panic..
use cortex_m_rt::entry;                     // the macro for our startup function
use embedded_hal::PwmPin;                   // PWM output pin trait
use embedded_hal::digital::v2::OutputPin;   // GPIO output pin trait
use embedded_time::rate::*;                 // Embed the `Hz` function/trait:
use core::fmt::Write;                       // for writeln!() macro
use rp_pico as bsp;                         // Provide an alias for our BSP so we can switch targets quickly.
use bsp::hal::{
    prelude::*,                             // pull in any important traits
    pac,                                    // Peripheral Access Crate; low-level registers
    sio::Sio,                               // the SIO manages al the pins and their modes
    watchdog::Watchdog,                     // we need to regularly call the watchdog or it shuts down our Pico
};

// Min and max for PWM driver
// Change this if you have a backlight which is active on pin high.
// (my) reference board for this example was active on pin low!
const LOW: u16 = 0;
const HIGH: u16 = 65535;

// Even for examples it's necessary to import the library
extern crate pcd8544;
use pcd8544::PCD8544;
use pcd8544::dummypins::DummyOutputPin;


#[entry]
fn main() -> ! {

    // basic Raspberry Pico boiler plate setup stuff 
    let mut pac = pac::Peripherals::take().unwrap();   // grab singleton objects
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);    // set up watchdog timer

    let clocks = bsp::hal::clocks::init_clocks_and_plls(   // configure clocks
        bsp::XOSC_CRYSTAL_FREQ,                            // default is 125mHz system clock
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);       // the single-cycle I/O block (SIO) controls our GPIO pins
    let pins = bsp::Pins::new(         // first set up the pins to their default state
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // The spi driver picks up this pins automatically if they are in the correct mode
    let _spi_sclk = pins.gpio2.into_mode::<bsp::hal::gpio::FunctionSpi>();
    let _spi_mosi = pins.gpio3.into_mode::<bsp::hal::gpio::FunctionSpi>();
    let spi = bsp::hal::Spi::<_, _, 8>::new(pac.SPI0);

    // Exchange the uninitialised SPI driver for an initialised one
    // needs to be MODE_0
    let pcd_spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        4_000_000u32.Hz(),
        &embedded_hal::spi::MODE_0,
    );

    // configure all the necessary other pins
    let pcd_rst = pins.gpio6.into_push_pull_output();
    let pcd_ce = pins.gpio7.into_push_pull_output();
    let pcd_dc = pins.gpio8.into_push_pull_output();
    let pcd_unused = DummyOutputPin; 

    // configure PWM for dimmable backlight
    // gpio21 is on pwm2
    let mut pwm_slices = bsp::hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);
    let pwm = &mut pwm_slices.pwm2;
    pwm.set_ph_correct();
    pwm.enable();

    // Output channel B on PWM4 to the LED pin
    let pcd_light = &mut pwm.channel_b;
    pcd_light.output_to(pins.gpio21);

    // set led pin to output
    let mut pico_led = pins.led.into_push_pull_output();

    // Setup a delay for the LED blink signals:
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    // Setting up the LCD display
    let mut pcd = PCD8544::new(pcd_spi, pcd_dc, pcd_ce, pcd_rst, pcd_unused).unwrap();

    // Do some stuff, keep the board busy 
    loop {

        // Engage onboard Led, and fade in backlight
        // put in some important texts
        pico_led.set_high().unwrap();
        pcd.clear().unwrap();
        writeln!(pcd, "Hello World!!\n\rWhats up?").unwrap();
        for i in (LOW..=HIGH).rev().skip(50) {
            delay.delay_us(10);
            pcd_light.set_duty(i);
        }
        delay.delay_ms(800);

        // Disengage onboard Led, and fade out backlight
        // and politely say goodbye
        pico_led.set_low().unwrap();
        pcd.clear().unwrap();
        writeln!(pcd, "Bye!!\n\rSee you later").unwrap();
        for i in (LOW..=HIGH).skip(50) {
            delay.delay_us(10);
            pcd_light.set_duty(i);
        }
        delay.delay_ms(800);
    }
}

// End of file
