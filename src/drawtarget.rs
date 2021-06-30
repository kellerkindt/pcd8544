use core::convert::TryInto;

use embedded_graphics_core::{pixelcolor::BinaryColor, prelude::*};
use embedded_hal::digital::v2::OutputPin;

use crate::{backend::PCD8544Backend, HEIGHT, PCD8544, ROWS, WIDTH};

const MAX_X: u32 = WIDTH as u32 - 1;
const MAX_Y: u32 = HEIGHT as u32 - 1;

impl<Backend, LIGHT, RST> PCD8544<Backend, LIGHT, RST>
where
    Backend: PCD8544Backend,
    LIGHT: OutputPin<Error = Backend::Error>,
    RST: OutputPin<Error = Backend::Error>,
{
    /// Transfers internal framebuffer data to PCD8544.
    ///
    /// This will ignore things drawn using the text interface
    pub fn flush(&mut self) -> Result<(), Backend::Error> {
        for row in self.framebuffer.clone().iter() {
            for byte in row.iter() {
                self.write_data(*byte)?;
            }
        }
        Ok(())
    }
}

impl<Backend, LIGHT, RST> DrawTarget for PCD8544<Backend, LIGHT, RST>
where
    Backend: PCD8544Backend,
    LIGHT: OutputPin<Error = Backend::Error>,
    RST: OutputPin<Error = Backend::Error>,
{
    type Error = Backend::Error;
    type Color = BinaryColor;

    fn clear(&mut self, color: BinaryColor) -> Result<(), Backend::Error> {
        let byte: u8 = match color {
            BinaryColor::On => 0xff,
            BinaryColor::Off => 0x00,
        };
        self.framebuffer = [[byte; WIDTH as usize]; ROWS as usize];
        Ok(())
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels {
            let Pixel(coord, color) = pixel;
            if let Ok((x @ 0..=MAX_X, y @ 0..=MAX_Y)) = coord.try_into() {
                let byte: &mut u8 = &mut self.framebuffer[(y / 8) as usize][x as usize];
                let mask: u8 = 1 << (y % 8);
                match color {
                    BinaryColor::On => *byte |= mask,
                    BinaryColor::Off => *byte &= !mask,
                };
            }
        }
        Ok(())
    }
}

impl<Backend, LIGHT, RST> OriginDimensions for PCD8544<Backend, LIGHT, RST>
where
    Backend: PCD8544Backend,
    LIGHT: OutputPin<Error = Backend::Error>,
    RST: OutputPin<Error = Backend::Error>,
{
    fn size(&self) -> Size {
        Size::new(WIDTH.into(), HEIGHT.into())
    }
}
