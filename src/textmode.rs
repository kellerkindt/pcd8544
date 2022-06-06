//! # Textmode text only extensions for the PCD8544 driver
//!
//! This extension uses an embedded font of 6x8 and a separate text cursor
//! to write text on the display, like a teletype.
//! It implements the Write trait so the writeln!() macro "just works".
//!
//! The screen size is 14x6 characters with this font.
//!
//! TODO: small example
use core::fmt::Error as FmtError;
use core::fmt::Result as FmtResult;
use core::fmt::Write;

use embedded_hal::blocking::spi::Write as SpiWrite;
use embedded_hal::digital::v2::OutputPin;

use crate::{
    WIDTH, DDRAM_BANKS,
    instructions::Instruction::{Xaddress, Yaddress},
    display::PCD8544, 
    error::*,
};

const COLUMNS: u8 = WIDTH / 6;
const ROWS: u8 = DDRAM_BANKS as u8;

/// Main trait to implement the textmode (ascii as you will) driver extension
pub trait TextMode<PCDError> {

    /// return position of the text cursor in current row
    fn pos(&self) -> u8; 

    /// return row where the text cursor is 
    fn row(&self) -> u8; 

    /// set the position (column, row) of the text cursor
    fn set_position(&mut self, text_col: u8, text_row: u8) -> Result<(), PCDError>;

    /// set the column ("x value") of the text cursor. position in 0..14
    fn set_col(&mut self, text_col: u8) -> Result<(), PCDError>;

    /// set the row ("y value) of the text cursor. row in 0..6
    fn set_row(&mut self, text_row: u8) -> Result<(), PCDError>;

    /// test if we have a newline and adjust internal variables accordingly
    fn cr_lf(&mut self);

    /// increase the cursor position by one, and auto cr/lf at end of line
    fn increase_position(&mut self);
    
    /// clear the screen and set cursor to (0,0)
    fn cls(&mut self) -> Result<(), PCDError>;
}

impl<SPI, DC, CE, RST, LIGHT, ERR> TextMode<PCDError> for PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8, Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
{
    fn pos(&self) -> u8 {
        self.text_col
    }

    fn row(&self) -> u8 {
        self.text_row
    }
 
    fn set_position(&mut self, new_col: u8, new_row: u8) -> Result<(), PCDError> {

        self.set_col(new_col)?;
        self.set_row(new_row)?;
        
        Ok(())

    }

    fn set_col(&mut self, new_col: u8) -> Result<(), PCDError> {

        // bounds checking
        if (0..COLUMNS).contains(&new_col) {

            self.text_col = new_col;

            // Xaddress takes a column value between 0..84 (pixel-wide)
            // so we need to multiply to get the correct position
            self.write_command(Xaddress(new_col * 6))

        } else {
            // silently ignore out of bounds
            Ok(())
        }
    }

    fn set_row(&mut self, new_row: u8) -> Result<(), PCDError> {

        // bounds checking
        if (0..ROWS).contains(&new_row) {

            self.text_row = new_row;

            // PCD8544 takes "bank" as argument for Yaddress.
            // A DDRAM "bank" is 8 pixels high, so row equals bank.
            self.write_command(Yaddress(new_row))

        } else {
            // silently ignore out of bounds
            Ok(())
        }
    }

    fn cr_lf(&mut self) {

        if self.text_col == 14 {

            // Carriage return
            self.text_col = 0;

            // Line feed & wrap
            if self.text_row == 5 {
                self.text_row = 0;
            } else {
                self.text_row += 1;
            }
        }
    }

    fn increase_position(&mut self) {

        // The PCD8544 has auto-increasing of internal address registers.
        // PCD8544 has a one-way serial bus (SPI-like) so we can't read from it.
        // We have to do some "shadow-administration" with some vars which we need to increase.
        self.text_col += 1;
        self.cr_lf();
    }

    fn cls(&mut self) -> Result<(), PCDError> {

        // reset the text cursor position
        self.text_col = 0;
        self.text_row = 0;
 
        // clear the screen by clearing DDRAM in PCD8544
        // hw_cls() also resets hardware cursor to (0, 0)
        self.hw_cls()
  
    }

}

impl<SPI, DC, CE, RST, LIGHT, ERR> Write for PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8, Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
{
    // with this implementation of write_str, the PCD8544 becomes addressable with all the
    // standard "Write" fuctions. Especially the writeln!() macro works directly on the screen.
    fn write_str(&mut self, s: &str) -> FmtResult {
        for c in s.chars() {
            match c {
                '\r' => {
                    self.set_col(0).map_err(|_| FmtError)?;
                }
                '\n' => {
                    let padding = COLUMNS - self.text_col;
                    for _ in 0..padding {
                        self.write_data(&[0x00; 6]).map_err(|_| FmtError)?;
                    }
                    self.text_col += padding;
                    self.cr_lf();
                }
                _ => {
                    self.write_data(char_to_bytes(c)).map_err(|_| FmtError)?;
                    self.write_data(&[0x00]).map_err(|_| FmtError)?;
                    self.increase_position();
                }
            }
        }
        Ok(())
    }
}

// internal function which implements the "font". It's a 6x8 font.
// it returns a byte array containing a PCD8544 DDRAM "compatible" presentation of the different
// ascii characters.
//
// The last byte (a 0x00 "character separator") is added in the write_str() function, that's why
// there's 5 bytes per character. This trick conserves a bit of memory.
//
// PCD8544 uses a layout where every byte is a vertical line of 8 bits/pixels, starting at the top
// so bytes in this array are vertical lines of 8 pixels.
//
// no, it's not unicode, but an ascii subset. It gets the job done and looks nice.
fn char_to_bytes(c: char) -> &'static [u8] {
    match c {
        ' ' => &[0x00, 0x00, 0x00, 0x00, 0x00],
        '!' => &[0x00, 0x00, 0x5f, 0x00, 0x00],
        '"' => &[0x00, 0x07, 0x00, 0x07, 0x00],
        '#' => &[0x14, 0x7f, 0x14, 0x7f, 0x14],
        '$' => &[0x24, 0x2a, 0x7f, 0x2a, 0x12],
        '%' => &[0x23, 0x13, 0x08, 0x64, 0x62],
        '&' => &[0x36, 0x49, 0x55, 0x22, 0x50],
        '\'' => &[0x00, 0x05, 0x03, 0x00, 0x00],
        '(' => &[0x00, 0x1c, 0x22, 0x41, 0x00],
        ')' => &[0x00, 0x41, 0x22, 0x1c, 0x00],
        '*' => &[0x14, 0x08, 0x3e, 0x08, 0x14],
        '+' => &[0x08, 0x08, 0x3e, 0x08, 0x08],
        ',' => &[0x00, 0x50, 0x30, 0x00, 0x00],
        '-' => &[0x08, 0x08, 0x08, 0x08, 0x08],
        '.' => &[0x00, 0x60, 0x60, 0x00, 0x00],
        '/' => &[0x20, 0x10, 0x08, 0x04, 0x02],
        '0' => &[0x3e, 0x51, 0x49, 0x45, 0x3e],
        '1' => &[0x00, 0x42, 0x7f, 0x40, 0x00],
        '2' => &[0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => &[0x21, 0x41, 0x45, 0x4b, 0x31],
        '4' => &[0x18, 0x14, 0x12, 0x7f, 0x10],
        '5' => &[0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => &[0x3c, 0x4a, 0x49, 0x49, 0x30],
        '7' => &[0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => &[0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => &[0x06, 0x49, 0x49, 0x29, 0x1e],
        ':' => &[0x00, 0x36, 0x36, 0x00, 0x00],
        ';' => &[0x00, 0x56, 0x36, 0x00, 0x00],
        '<' => &[0x08, 0x14, 0x22, 0x41, 0x00],
        '=' => &[0x14, 0x14, 0x14, 0x14, 0x14],
        '>' => &[0x00, 0x41, 0x22, 0x14, 0x08],
        '?' => &[0x02, 0x01, 0x51, 0x09, 0x06],
        '@' => &[0x32, 0x49, 0x79, 0x41, 0x3e],
        'A' => &[0x7e, 0x11, 0x11, 0x11, 0x7e],
        'B' => &[0x7f, 0x49, 0x49, 0x49, 0x36],
        'C' => &[0x3e, 0x41, 0x41, 0x41, 0x22],
        'D' => &[0x7f, 0x41, 0x41, 0x22, 0x1c],
        'E' => &[0x7f, 0x49, 0x49, 0x49, 0x41],
        'F' => &[0x7f, 0x09, 0x09, 0x09, 0x01],
        'G' => &[0x3e, 0x41, 0x49, 0x49, 0x7a],
        'H' => &[0x7f, 0x08, 0x08, 0x08, 0x7f],
        'I' => &[0x00, 0x41, 0x7f, 0x41, 0x00],
        'J' => &[0x20, 0x40, 0x41, 0x3f, 0x01],
        'K' => &[0x7f, 0x08, 0x14, 0x22, 0x41],
        'L' => &[0x7f, 0x40, 0x40, 0x40, 0x40],
        'M' => &[0x7f, 0x02, 0x0c, 0x02, 0x7f],
        'N' => &[0x7f, 0x04, 0x08, 0x10, 0x7f],
        'O' => &[0x3e, 0x41, 0x41, 0x41, 0x3e],
        'P' => &[0x7f, 0x09, 0x09, 0x09, 0x06],
        'Q' => &[0x3e, 0x41, 0x51, 0x21, 0x5e],
        'R' => &[0x7f, 0x09, 0x19, 0x29, 0x46],
        'S' => &[0x46, 0x49, 0x49, 0x49, 0x31],
        'T' => &[0x01, 0x01, 0x7f, 0x01, 0x01],
        'U' => &[0x3f, 0x40, 0x40, 0x40, 0x3f],
        'V' => &[0x1f, 0x20, 0x40, 0x20, 0x1f],
        'W' => &[0x3f, 0x40, 0x38, 0x40, 0x3f],
        'X' => &[0x63, 0x14, 0x08, 0x14, 0x63],
        'Y' => &[0x07, 0x08, 0x70, 0x08, 0x07],
        'Z' => &[0x61, 0x51, 0x49, 0x45, 0x43],
        '[' => &[0x00, 0x7f, 0x41, 0x41, 0x00],
        '¥' => &[0x02, 0x04, 0x08, 0x10, 0x20],
        ']' => &[0x00, 0x41, 0x41, 0x7f, 0x00],
        '^' => &[0x04, 0x02, 0x01, 0x02, 0x04],
        '_' => &[0x40, 0x40, 0x40, 0x40, 0x40],
        '`' => &[0x00, 0x01, 0x02, 0x04, 0x00],
        'a' => &[0x20, 0x54, 0x54, 0x54, 0x78],
        'b' => &[0x7f, 0x48, 0x44, 0x44, 0x38],
        'c' => &[0x38, 0x44, 0x44, 0x44, 0x20],
        'd' => &[0x38, 0x44, 0x44, 0x48, 0x7f],
        'e' => &[0x38, 0x54, 0x54, 0x54, 0x18],
        'f' => &[0x08, 0x7e, 0x09, 0x01, 0x02],
        'g' => &[0x0c, 0x52, 0x52, 0x52, 0x3e],
        'h' => &[0x7f, 0x08, 0x04, 0x04, 0x78],
        'i' => &[0x00, 0x44, 0x7d, 0x40, 0x00],
        'j' => &[0x20, 0x40, 0x44, 0x3d, 0x00],
        'k' => &[0x7f, 0x10, 0x28, 0x44, 0x00],
        'l' => &[0x00, 0x41, 0x7f, 0x40, 0x00],
        'm' => &[0x7c, 0x04, 0x18, 0x04, 0x78],
        'n' => &[0x7c, 0x08, 0x04, 0x04, 0x78],
        'o' => &[0x38, 0x44, 0x44, 0x44, 0x38],
        'p' => &[0x7c, 0x14, 0x14, 0x14, 0x08],
        'q' => &[0x08, 0x14, 0x14, 0x18, 0x7c],
        'r' => &[0x7c, 0x08, 0x04, 0x04, 0x08],
        's' => &[0x48, 0x54, 0x54, 0x54, 0x20],
        't' => &[0x04, 0x3f, 0x44, 0x40, 0x20],
        'u' => &[0x3c, 0x40, 0x40, 0x20, 0x7c],
        'v' => &[0x1c, 0x20, 0x40, 0x20, 0x1c],
        'w' => &[0x3c, 0x40, 0x30, 0x40, 0x3c],
        'x' => &[0x44, 0x28, 0x10, 0x28, 0x44],
        'y' => &[0x0c, 0x50, 0x50, 0x50, 0x3c],
        'z' => &[0x44, 0x64, 0x54, 0x4c, 0x44],
        '{' => &[0x00, 0x08, 0x36, 0x41, 0x00],
        '|' => &[0x00, 0x00, 0x7f, 0x00, 0x00],
        '}' => &[0x00, 0x41, 0x36, 0x08, 0x00],
        '←' => &[0x10, 0x08, 0x08, 0x10, 0x08],
        '→' => &[0x78, 0x46, 0x41, 0x46, 0x78],
        '°' => &[0x00, 0x02, 0x05, 0x02, 0x00],
        _ => &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    }
}
