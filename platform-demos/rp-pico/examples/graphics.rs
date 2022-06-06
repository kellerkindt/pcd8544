//! Displays graphical stuff on a Philips PCD8544 driven Nokia 5110/3310 screen
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

// Some constants for the animation
const BALLSIZE: usize = 10;
const STEEPNESS: usize = 8;  // you can change the "gravity" of the bounce here. 8 is nice.
const ENERGY_LOSS: usize = 2; // how much energy is lost with each bounce
const BALLHEIGHT: usize = HEIGHT as usize - BALLSIZE; 

use embedded_graphics::{
    prelude::*,
    pixelcolor::BinaryColor,
    primitives::{Circle, PrimitiveStyle},
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    text::Text,
};

// Even for examples it's necessary to import the library
extern crate pcd8544;
use pcd8544::PCD8544;
use pcd8544::{WIDTH, HEIGHT};
use pcd8544::graphics::GraphicsMode;
use pcd8544::dummypins::DummyOutputPin;

// small function to precalculate the slope of the falling ball.
//
// It's more common practise to have a separate program generating the LUT
// and just hardcopy it into the project with a static array.
// It's here to let this be a complete example.
fn slope_lut(width: usize, height: usize, steepness: usize) -> ([usize; WIDTH as usize], usize) {

    let mut slope = [0_usize; WIDTH as usize];
    let mut x: usize = 1;
    
    loop {
       // falling is quadratic, so we create a parabola.
       let y = (x * x) / steepness;

       // stop if we hit the bottom of the screen, minus ballsize
       if y > height as usize - 1 { break };

       // put our freshly discovered knowledge of location into the LUT
       slope[x as usize] = y;

       // stop if we hit the right side of the screen 
       if x < width as usize {x += 1} else { break };
    }

    // return the slope LUT
    (slope, x - 1) 
}


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
        4_000_000u32.Hz(),          // this should result at 4MBit/s which is the max of pcd8544
        &embedded_hal::spi::MODE_0,
    );

    // configure all the necessary other pins
    // rst = reset, ce = chip enable, dc = data connect. pcd_unused is a dummy for the backlight
    let pcd_rst = pins.gpio6.into_push_pull_output();
    let pcd_ce = pins.gpio7.into_push_pull_output();
    let pcd_dc = pins.gpio8.into_push_pull_output();
    let pcd_unused = DummyOutputPin; // I use PWM so dummy, replace with GPIO for just on/off

    // set led pin to output
    let mut pico_led = pins.led.into_push_pull_output();

    // Setup a delay for the LED blink signals:
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

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

    // Output channel B on PWM4 to the LED pin
    let pcd_light = &mut pwm.channel_b;
    pcd_light.output_to(pins.gpio21);
    pcd_light.set_duty(HIGH);

 // --------------------------------------------------------------------------
 //  End Boilerplate and Setup, let's initialize the screen
 //  and some demo stuff like the text style and a lookup table for the bounce
 // --------------------------------------------------------------------------

    // Setting up the LCD display
    let mut pcd = PCD8544::new(pcd_spi, pcd_dc, pcd_ce, pcd_rst, pcd_unused).unwrap();

    // Set onboard LED to on to show we are in business
    pico_led.set_high().unwrap();

    // fade in backlight
    for i in (LOW..=HIGH).rev().skip(50) {
        delay.delay_us(20);
        pcd_light.set_duty(i);
    }

    // Create a new character style for some text
    let style = MonoTextStyle::new(&FONT_6X9, BinaryColor::On);

    // we need to keep track of the direction in the slope
    enum Direction {
        Up,
        Down,
    }

    // set up the state variables for the bouncing ball
    let (slope, max_x) = slope_lut(WIDTH as usize, BALLHEIGHT, STEEPNESS); 
    let mut slope_max = max_x;
    let mut slope_cursor: usize = 0;
    let mut slope_direction = Direction::Down;
    let mut height_loss: usize = 0;
    let mut y: i32;

 // --------------------------------------------------------------------------
 //  And loop forever bouncing balls.
 // --------------------------------------------------------------------------
 
    loop {

        // Let's walk the width of the screen.
        // the height of the ball comes from our precalculated LUT
        for x in 0..WIDTH {

            // we now know the height, let's put our knowledge in our variable
            y = (slope[slope_cursor] + height_loss) as i32;
            
            // we are at the top, ball must go down 
            if slope_cursor == 0 {
                slope_direction = Direction::Down;
                slope_cursor += 1;

            // we are at the bottom, ball bounces back up, but with a lower slope.
            } else if slope_cursor == slope_max {
                slope_max -= ENERGY_LOSS; 
                height_loss = BALLHEIGHT - slope[slope_max];
                slope_cursor = slope_max - 1;
                slope_direction = Direction::Up;

            // we are somewehere in the middle, falling down or bouncing up.
            } else {
                match slope_direction {
                   Direction::Down => slope_cursor += 1,
                   Direction::Up => slope_cursor -= 1,
                }
            }

            // clear the framebuffer (not the screen)
            pcd.clear(BinaryColor::Off).unwrap();
    
            // draw a circle into the framebuffer (this is invisible for now!)
            Circle::new(Point::new(x as i32, y), BALLSIZE as u32)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(&mut pcd).unwrap(); 

            // Put some text in as well
            Text::new("Balls...", Point::new(25, 5), style).draw(&mut pcd).unwrap();
        
            // copy the in-memory framebuffer to the PCD8544 chip
            pcd.flush().unwrap();
    
            // wait a bit
            delay.delay_ms(20);
        }

        // reset state for the next run
        slope_max = max_x;
        slope_cursor = 0;
        slope_direction = Direction::Down;
        height_loss = 0;

    }
}

// End of file
