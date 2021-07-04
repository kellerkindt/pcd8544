use core::fmt::Error as FmtError;
use core::fmt::Result as FmtResult;
use core::fmt::Write;

use embedded_hal::digital::v2::OutputPin;

use crate::font::*;
use crate::{backend::PCD8544Backend, PCD8544, ROWS, WIDTH};

impl<Backend, LIGHT, RST> PCD8544<Backend, LIGHT, RST>
where
    Backend: PCD8544Backend,
    LIGHT: OutputPin<Error = Backend::Error>,
    RST: OutputPin<Error = Backend::Error>,
{
    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }

    pub fn set_x_position(&mut self, x: u8) -> Result<(), Backend::Error> {
        let x = x % WIDTH;
        self.x = x;
        self.write_command(0x80 | x)
    }

    pub fn set_y_position(&mut self, y: u8) -> Result<(), Backend::Error> {
        let y = y % ROWS;
        self.y = y;
        self.write_command(0x40 | y)
    }

    pub fn reset_position(&mut self) {
        self.x = 0;
        self.y = 0;
    }

    fn write_char(&mut self, value: u8) -> Result<(), Backend::Error> {
        self.increase_position();

        self.write_data(value)
    }

    fn increase_position(&mut self) {
        self.x = (self.x + 1) % WIDTH;
        if self.x == 0 {
            self.y = (self.y + 1) & ROWS;
        }
    }
}

impl<Backend, LIGHT, RST> Write for PCD8544<Backend, LIGHT, RST>
where
    Backend: PCD8544Backend,
    LIGHT: OutputPin<Error = Backend::Error>,
    RST: OutputPin<Error = Backend::Error>,
{
    fn write_str(&mut self, s: &str) -> FmtResult {
        for char in s.chars() {
            match char {
                '\r' => {
                    self.set_x_position(0).map_err(|_| FmtError)?;
                }
                '\n' => {
                    for _ in 0..(WIDTH - self.x) {
                        self.write_char(0x00).map_err(|_| FmtError)?;
                    }
                }
                _ => {
                    for b in char_to_bytes(char) {
                        self.write_char(*b).map_err(|_| FmtError)?;
                    }
                    self.write_char(0x00).map_err(|_| FmtError)?;
                }
            }
        }
        Ok(())
    }
}
