//! # Driver for embedded_hal and embedded_graphics for the populair PCD8544 display.
//!
//! The PCD8544 LCD driver chip is commonly found in Nokia black and white LCD displays of the 90's
//! era. There are a lot of (second-hand refurbished) screens of this type on the market, for
//! cheap, around the $5 mark. Most of them are soldered on a PCB with integrated backlights as a
//! "breadboard compatible screen" for testing and tinkering.
//!
//! The most commonly found variants come from China and are therefore based on the screen units
//! from the Nokia 5110 which was very populair in that are. Be aware, none of these screens are
//! new, they are all refurbished. The screens aren't produced any more. Only the PCB and LEDs
//! supporting the screen are new. Most big "tinkering" brands have such a variant in their
//! arsenal, like Adafruit and Sparkfun, but there are also a lot of brandless boards around.
//! This library can also be used to drive other Nokia screens, like the one on the Nokia 3310.
//!
//! ## Feature flags
//!
//! There are two feature flags, each one enabling an extra feature set. These are:
//!  - "textmode" -> enables text-only usage of the display by implementing Write<u8> so the
//!  writeln!() macro works as expected. It has a builtin font and can only produce text, no
//!  graphics.
//!  - "graphics" -> enables the embedded_graphics driver and makes the PCD8544 usable as a
//!  display. See the embedded_graphics crate documentation for the limitless possibilities this
//!  opens up.
//!
//! ## Usage
//!
//! See the example for basic usage. There are usually eight pins on the screen units. Two of them
//! are just 3.3V power and ground. There is one for the backlight and the other five pins are for
//! driving the PCD8544 chip. Here is an example of one board, with what to do with every pin:
//!
//! 1. RST     ->  Attach to GPIO OutputPin
//! 2. CE      ->  Attach to GPIO OutputPin
//! 3. DC      ->  Attach to GPIO OutputPin
//! 4. DIN     ->  Connect as MOSI in a SPI config
//! 5. CLK     ->  Connect als CLK in a SPI config
//! 6. VCC     ->  Directly connect to 3.3V power on the microcontroller or level converter
//! 7. LIGHT   ->  Attach to GPIO OutputPin
//! 8. GND     ->  Directly connect to ground on the microcontroller
//!
//! You just assign all the pins to the proper type of connection, and call PCD8544.new().
//! After that you can use all the methods associated with the PCD8544 struct and traits.
//!
//! Here is an example:
//!
//! ```rust
//! use pcd8544::{bitbang::BitBangSpi, dummypins::DummyOutputPin, PCD8544};
//! use std::fmt::Write;
//!
//! fn main() {
//!    let pcd_clk = DummyOutputPin;
//!    let pcd_din = DummyOutputPin;
//!    let pcd_spi = BitBangSpi::new(pcd_clk, pcd_din).expect("Infallible cannot fail");
//!
//!    let pcd_dc = DummyOutputPin;
//!    let pcd_ce = DummyOutputPin;
//!    let pcd_rst = DummyOutputPin;
//!    let pcd_light = DummyOutputPin;
//!
//!    let mut display =
//!        PCD8544::new(pcd_spi, pcd_dc, pcd_ce, pcd_rst, pcd_light).expect("Infallible cannot fail");
//!
//!    display.reset().expect("Infallible cannot fail");
//!    writeln!(display, "Hello World").unwrap();
//! }
//! ```
//!
//! ## Demos
//!
//! There are demos for a bluepill board (STM32something) and the Raspberry Pico (rp-pico) in the
//! folder "platform-demos". If you would like to compile such a demo, please git clone this repo, go
//! to the `platform-demos` directory, go to one of the boards directory (like `rp-pico`) and run
//! `cargo build --examples`. Then go to the `target/examples` dir to find the ELF, and
//! convert/upload it to your microcontroller.
//!
//! For the Raspberry Pico, if you already have a working toolchain with elf2uf2-rs and such, as
//! described in the [rp-hal](https://github.com/rp-rs/rp-hal) documentation, you could just
//! mount the Pico in disk mode (hold the button and attach USB) and just do a `cargo run ---example X`
//! in the `platform-demos/rp-pico` directory. X should be one of the demos in the `examples` dir.
//! The `run` command should then automatically upload and run the demo. See the Cargo.toml in the
//! `platform-demos/rp-pico` dir for more details, but it's basically the boilerplater rp-hal
//! template.
//!
//! The examples in the `platform-demos` directory are a bit elaborate and not really good
//! "examples" because a good example should be short and to the point. The demos have a lot of
//! "clutter" but portray a real-world implementation a bit better.
#![no_std]
#![forbid(deprecated)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(warnings)]
#![deny(missing_copy_implementations)]
#![deny(trivial_casts)]
#![deny(trivial_numeric_casts)]
#![deny(unsafe_code)]
#![deny(unstable_features)]
#![deny(unused_import_braces)]
#![deny(unused_qualifications)]

extern crate embedded_hal as hal;

// The PCD8544 LCD driver has a Graphic Display Data RAM (GDDRAM) of 504 bytes.
// The memory area is organized in 6 banks of 84 columns/segments.
// Each column can store 8 bits.
// so each byte per bank represents a vertical column of 8 pixels.

/// The width of the screen, in pixels
pub const WIDTH: u8 = 84;
/// The height of the screen, in pixels
pub const HEIGHT: u8 = 48;
/// Number of text rows, using an eight point font
pub const ROWS: u8 = HEIGHT / 8;
/// Number of memory banks in the PCD DDRAM (banks are one byte wide)
pub const DDRAM_BANKS: usize = 6;
/// Total memory available in DDRAM in bytes (should be 504)
pub const DDRAM_SIZE: usize = WIDTH as usize * DDRAM_BANKS;

pub mod dummypins;
pub mod bitbang;
pub mod instructions;
pub mod error;
mod display;

#[cfg(feature = "textmode")]
pub mod textmode;

#[cfg(feature = "graphics")]
pub mod graphics;

pub use crate::display::PCD8544;

/*

To be done:
------
- finish graphics mode:
  - fill -> let's make some more efficient algorithms
- make default profiles for known screens

*/
