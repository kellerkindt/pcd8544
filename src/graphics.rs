//! Graphics driver for the PCD8544
//!
//! This module is behind a feature flag. Enable it in your Cargo.toml with feature flag
//! "graphics".
//! 
//! It implements all necessary functions and traits to be able to use the embedded_graphics
//! library, so all circle/ellipse/text/rectangle/bitmap functions are available on the screen.
//! It still is possible to directly manipulate the framebuffer with set_pixel() without using
//! embedded_graphics functions, if you want to write your own shaders. 
//!
//! It uses a framebuffer for efficiency. This is a trade-off between speed and low(er) processor
//! overhead, verses higher memory uses on te microcontroller. That's the main reason it is behind
//! a feature flag. If only some text is necessary on the display (like display of sensor data)
//! withhout fancy graphics, then please consider the "TextMode" feature flag with its functions,
//! it's much lighter on microprocessor resources, and easier to use.
//!
//! The typical workflow for (animated) graphics is:
//!  - clear the framebuffer with PCD8544.clear()
//!  - draw "stuff" into the framebuffer (Cirle's, Text's, embedded_graphics "stuff")
//!  - PCD8544.flush() the framebuffer to the screen's DDRAM, it now gets visible
//!  - rinse and repeat, so clear the framebuffer and draw and flush and clear and draw and...
//!
//!  The embedded_graphics library is well documented. Please look there for all the juicyness of
//!  graphics functions it provides.
//!  <https://docs.rs/embedded-graphics/latest/embedded_graphics/>
use hal::blocking::spi::Write as SpiWrite;
use hal::digital::v2::OutputPin;

use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::Size,
    geometry::{Dimensions, OriginDimensions},
    pixelcolor::BinaryColor,
    Pixel,
};

use crate::{ 
    WIDTH, HEIGHT, DDRAM_SIZE,
    display::PCD8544,
    error::*,
};

/// trait with extra functions needed for using graphics on the PCD8544.
///
/// this functions are behind a feature flag. Set "graphics" as a feature for this library in your
/// Cargo.toml
pub trait GraphicsMode<ERR> {

    /// Write the in-memory frambebuffer to the PCD8544 DDRAM using SPI
    fn flush(&mut self) -> Result<(), PCDError>;

    /// Set a pixel at x, y in the framebuffer to color "color"
    ///
    /// It is the main function used by draw_iter in the DrawTarget trait implementation for the
    /// PCD8544 driver. It can be used on itself to manipulate the framebuffer, if you want to
    /// implement a shader/effect directly without using embedded_graphics. Just clear() the
    /// framebuffer, set_pixel all you want with your own shader, and flush() it to the PCD8544.
    fn set_pixel(&mut self, x: u32, y: u32, color: BinaryColor);
}

impl<SPI, DC, CE, RST, LIGHT, ERR> GraphicsMode<ERR> for PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8, Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
{

    fn flush(&mut self) -> Result<(), PCDError> {
        let data = self.framebuffer;
        self.write_data(&data)?;
        Ok(())
    }

    fn set_pixel(&mut self, x: u32, y: u32, color: BinaryColor) {

        // bounds checking
        if (0..WIDTH as u32).contains(&x) && (0..HEIGHT as u32).contains(&y) {

            let mut buffer = self.framebuffer;            

            match color {
                BinaryColor::On => buffer[((y / 8) * WIDTH as u32 + x) as usize] |= 1 << (y % 8),
                BinaryColor::Off => buffer[((y / 8) * WIDTH as u32 + x) as usize] &= !(1 << (y % 8)),
            };

            self.framebuffer = buffer;
        }
    }
}

// Implementation of the embedded_graphics DrawTarget trait on our PCD8544 driver.
// By implementing these few functions, we get the complete power of the embedded_graphics library for free.
// We can then draw Circle's, Rectangle's, Text and even bitmaps and TrueType fonts.
impl<SPI, DC, CE, RST, LIGHT, ERR> DrawTarget for PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8, Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
{
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let bb = self.bounding_box();

        pixels
            .into_iter()
            .filter(|Pixel(pos, _color)| bb.contains(*pos))
            .for_each(|Pixel(pos, color)| {
                self.set_pixel(pos.x as u32, pos.y as u32, color);
            });

        Ok(())
    }

    fn clear(&mut self, color: BinaryColor) -> Result<(), Self::Error> {
       let byte: u8 = match color {
           BinaryColor::On => 0xff,
           BinaryColor::Off => 0x00,
        };
        self.framebuffer = [byte; DDRAM_SIZE]; 
        Ok(())
    }

}

impl<SPI, DC, CE, RST, LIGHT, ERR> OriginDimensions for PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8, Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
{
    // return the size of the screen in pixels, so embedded_graphics knows how to properly operate
    // on it.
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

