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
use embedded_time::rate::*;                 // Embed the `Hz` function/trait
use core::fmt::Write;                       // for writeln!() macro
use rp_pico as bsp;                         // Provide an alias for our BSP so we can switch targets quickly.
use bsp::hal::{
    prelude::*,                             // pull in any important traits
    pac,                                    // Peripheral Access Crate; low-level registers
    sio::Sio,                               // Single-clock IO, takes care of all GPIO, SPI et al. stuff.
    watchdog::Watchdog,                     // we need to regularly call the watchdog or it shuts down our Pico
};

// Min and max for PWM driver
// Change this if you have a backlight which is active on pin high.
// to for example:
// const LOW: u16 = 32000;
// const HIGH: u16 = 0;
//
// (my) reference board for this example gives full backlight when pin is low.
const LOW: u16 = 0;
const HIGH: u16 = 65535;

// Even for examples it's necessary to import the library
extern crate pcd8544;
use pcd8544::PCD8544;
use pcd8544::dummypins::DummyOutputPin;
use pcd8544::textmode::TextMode;

#[entry]
fn main() -> ! {

 // --------------------------------------------------------------------------
 //  First part is "boilerplate" setup stuff for Raspberry Pico 
 // --------------------------------------------------------------------------

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

 // --------------------------------------------------------------------------
 //  Now we configure all our pins to do the proper thing. 
 //  We need some GPIO's, a functional SPI and the onboard LED
 // --------------------------------------------------------------------------

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
        8_000_000u32.Hz(),  // 8mHz should result in max supported 4Mbit/s
        &embedded_hal::spi::MODE_0,
    );

    // configure all the necessary other pins
    let pcd_rst = pins.gpio6.into_push_pull_output();
    let pcd_ce = pins.gpio7.into_push_pull_output();
    let pcd_dc = pins.gpio8.into_push_pull_output();
    let pcd_unused = DummyOutputPin; 

 // --------------------------------------------------------------------------
 //  Next part is only for the backlight of the PCD8544
 //  You only need this if you want fancy dimmable backlight stuff
 // --------------------------------------------------------------------------

    // configure PWM for dimmable backlight
    // gpio21 is on pwm2
    let mut pwm_slices = bsp::hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);
    let pwm = &mut pwm_slices.pwm2;
    pwm.set_ph_correct();
    pwm.enable();

 // --------------------------------------------------------------------------
 //  End Boilerplate and Setup, let's initialize the screen
 //  and some demo stuff, like some fine lyrics of a famous song
 // --------------------------------------------------------------------------

    // Output channel B on PWM4 to the LED pin
    let pcd_light = &mut pwm.channel_b;
    pcd_light.output_to(pins.gpio21);

    // set led pin to output
    let mut pico_led = pins.led.into_push_pull_output();

    // Setup a delay for the LED blink signals:
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    // Setting up the LCD display
    let mut pcd = PCD8544::new(pcd_spi, pcd_dc, pcd_ce, pcd_rst, pcd_unused).unwrap();

    // We need the lyrics for the song. This is a constant array of static strings. 
    const LYRICS: &[& str] = &[
       "give you up",
       "let you down",
       "run around",
       "and",
       "desert",
       "you",
       "make you cry",
       "say goodbye",
       "tell a lie",
       "and",
       "hurt",
       "you",
    ];

    let text_delay = 1600;
    
    pcd.cls().unwrap();

 // --------------------------------------------------------------------------
 //  And loop forever performing like a star.
 // --------------------------------------------------------------------------

    // Do some stuff, keep the board busy 
    loop {

        // Set backlight ON with a nice fade in
        for i in (LOW..=HIGH).rev().skip(50) {
            delay.delay_us(10);
            pcd_light.set_duty(i);
        }

        // Lets activate the onboard LED, we are at work now
        pico_led.set_high().unwrap();

        // Sing! like a bird in the sky.
        for i in 0..3 {
            delay.delay_ms(text_delay);
            pcd.cls().unwrap();
            writeln!(pcd, "Never gonna\n{}", LYRICS[i]).unwrap();
        }

        // lets put in some emphasized words
        for i in 3..6 {
            delay.delay_ms(text_delay/4);
            writeln!(pcd, "{}", LYRICS[i]).unwrap();
        }

        // The corus continues
        for i in 6..9 {
            delay.delay_ms(text_delay);
            pcd.cls().unwrap();
            writeln!(pcd, "Never gonna\n{}", LYRICS[i]).unwrap();
        }

        // final words
        for i in 9..12 {
            delay.delay_ms(text_delay/4);
            writeln!(pcd, "{}", LYRICS[i]).unwrap();
        }

        // finally, wait a bit and clear the screen for the last time
        delay.delay_ms(text_delay);
        pcd.cls().unwrap();

        // Disengage the onboard LED, work is over
        pico_led.set_low().unwrap();
   
        // Set backlight off with a nice fade out 
        for i in (LOW..=HIGH).skip(50) {
            delay.delay_us(10);
            pcd_light.set_duty(i);
        }

        // just wait a moment before the next run
        delay.delay_ms(1500);
    }
}

// End of file
