#![no_std]
use core::fmt::Error as FmtError;
use core::fmt::Result as FmtResult;
use core::fmt::Write;

use embedded_hal::blocking::spi::Write as SpiWrite;
use embedded_hal::digital::v2::OutputPin;

pub mod spi;

pub mod error;
use crate::error::*;

mod font;
use crate::font::*;

pub const WIDTH: u8 = 84;
pub const HEIGHT: u8 = 48;
pub const ROWS: u8 = HEIGHT / 8;

#[repr(u8)]
pub enum TemperatureCoefficient {
    TC0 = 0,
    TC1 = 1,
    TC2 = 2,
    TC3 = 3,
}

#[repr(u8)]
pub enum BiasMode {
    Bias1To100 = 0,
    Bias1To80 = 1,
    Bias1To65 = 2,
    Bias1To48 = 3,
    Bias1To40 = 4,
    Bias1To24 = 5,
    Bias1To18 = 6,
    Bias1To10 = 7,
}

#[repr(u8)]
pub enum DisplayMode {
    DisplayBlank = 0b000,
    NormalMode = 0b100,
    AllSegmentsOn = 0b001,
    InverseVideoMode = 0b101,
}

pub struct PCD8544<SPI, DC, CE, RST, LIGHT>
where
    DC: OutputPin,
    CE: OutputPin,
    RST: OutputPin,
    LIGHT: OutputPin,
{
    spi: SPI,
    dc: DC,
    ce: CE,
    rst: RST,
    light: LIGHT,
    power_down_control: bool,
    entry_mode: bool,
    extended_instruction_set: bool,
    x: u8,
    y: u8,
}

impl<SPI, DC, CE, RST, LIGHT, PE> PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8>,
    DC: OutputPin<Error = PE>,
    CE: OutputPin<Error = PE>,
    RST: OutputPin<Error = PE>,
    LIGHT: OutputPin<Error = PE>,
{
    /// Constructs new PCD8544 with SPI and pin config.
    /// Note that if you use hardware SPI port it must be configured
    /// for SPI MODE0 and MSBFIRST.
    /// Please use BitBangSpi if you don't want to sacrifice a hardware SPI port.
    pub fn new(
        spi: SPI,
        dc: DC,
        mut ce: CE,
        mut rst: RST,
        light: LIGHT,
    ) -> Result<PCD8544<SPI, DC, CE, RST, LIGHT>, PCDError> {
        PCDError::pin(rst.set_low())?;
        PCDError::pin(ce.set_high())?;
        Ok(PCD8544 {
            spi,
            dc,
            ce,
            rst,
            light,
            power_down_control: false,
            entry_mode: false,
            extended_instruction_set: false,
            x: 0,
            y: 0,
        })
    }

    pub fn reset(&mut self) -> Result<(), PCDError> {
        PCDError::pin(self.rst.set_low())?;
        self.x = 0;
        self.y = 0;
        self.init()
    }

    pub fn init(&mut self) -> Result<(), PCDError> {
        // reset the display
        PCDError::pin(self.rst.set_low())?;
        PCDError::pin(self.rst.set_high())?;

        // reset state variables
        self.power_down_control = false;
        self.entry_mode = false;
        self.extended_instruction_set = false;

        // write init configuration
        self.enable_extended_commands(true)?;
        self.set_contrast(56_u8)?;
        self.set_temperature_coefficient(TemperatureCoefficient::TC3)?;
        self.set_bias_mode(BiasMode::Bias1To40)?;
        self.enable_extended_commands(false)?;
        self.set_display_mode(DisplayMode::NormalMode)?;

        // clear display data
        self.clear()
    }

    pub fn clear(&mut self) -> Result<(), PCDError> {
        for _ in 0..(WIDTH as u16 * ROWS as u16) {
            self.write_data(0x00)?;
        }
        self.set_x_position(0)?;
        self.set_y_position(0)
    }

    pub fn set_power_down(&mut self, power_down: bool) -> Result<(), PCDError> {
        self.power_down_control = power_down;
        self.write_current_function_set()
    }

    pub fn set_entry_mode(&mut self, entry_mode: bool) -> Result<(), PCDError> {
        self.entry_mode = entry_mode;
        self.write_current_function_set()
    }

    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }

    pub fn set_x_position(&mut self, x: u8) -> Result<(), PCDError> {
        let x = x % WIDTH;
        self.x = x;
        self.write_command(0x80 | x)
    }

    pub fn set_y_position(&mut self, y: u8) -> Result<(), PCDError> {
        let y = y % ROWS;
        self.y = y;
        self.write_command(0x40 | y)
    }

    pub fn set_light(&mut self, enabled: bool) -> Result<(), PCDError> {
        if enabled {
            PCDError::pin(self.light.set_low())
        } else {
            PCDError::pin(self.light.set_high())
        }
    }

    pub fn set_display_mode(&mut self, mode: DisplayMode) -> Result<(), PCDError> {
        self.write_command(0x08 | mode as u8)
    }

    pub fn set_bias_mode(&mut self, bias: BiasMode) -> Result<(), PCDError> {
        self.write_command(0x10 | bias as u8)
    }

    pub fn set_temperature_coefficient(
        &mut self,
        coefficient: TemperatureCoefficient,
    ) -> Result<(), PCDError> {
        self.write_command(0x04 | coefficient as u8)
    }

    /// contrast in range of 0..128
    pub fn set_contrast(&mut self, contrast: u8) -> Result<(), PCDError> {
        self.write_command(0x80 | contrast)
    }

    pub fn enable_extended_commands(&mut self, enable: bool) -> Result<(), PCDError> {
        self.extended_instruction_set = enable;
        self.write_current_function_set()
    }

    fn write_current_function_set(&mut self) -> Result<(), PCDError> {
        let power = self.power_down_control;
        let entry = self.entry_mode;
        let extended = self.extended_instruction_set;
        self.write_function_set(power, entry, extended)
    }

    fn write_function_set(
        &mut self,
        power_down_control: bool,
        entry_mode: bool,
        extended_instruction_set: bool,
    ) -> Result<(), PCDError> {
        let mut val = 0x20;
        if power_down_control {
            val |= 0x04;
        }
        if entry_mode {
            val |= 0x02;
        }
        if extended_instruction_set {
            val |= 0x01;
        }
        self.write_command(val)
    }

    pub fn write_command(&mut self, value: u8) -> Result<(), PCDError> {
        PCDError::pin(self.dc.set_low())?;
        self.write_byte(value)
    }

    pub fn write_data(&mut self, value: u8) -> Result<(), PCDError> {
        PCDError::pin(self.dc.set_high())?;
        self.increase_position();
        self.write_byte(value)
    }

    #[inline]
    fn write_byte(&mut self, value: u8) -> Result<(), PCDError> {
        PCDError::pin(self.ce.set_low())?;
        PCDError::spi(self.spi.write(&[value]))?;
        PCDError::pin(self.ce.set_high())
    }

    #[inline]
    fn increase_position(&mut self) {
        self.x = (self.x + 1) % WIDTH;
        if self.x == 0 {
            self.y = (self.y + 1) & ROWS;
        }
    }
}

impl<SPI, DC, CE, RST, LIGHT, ERR> Write for PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
{
    fn write_str(&mut self, s: &str) -> FmtResult {
        for char in s.chars() {
            match char {
                '\r' => {
                    self.set_x_position(0).map_err(|_| FmtError)?;
                }
                '\n' => {
                    for _ in 0..(WIDTH - self.x) {
                        self.write_data(0x00).map_err(|_| FmtError)?;
                    }
                }
                _ => {
                    for b in char_to_bytes(char) {
                        self.write_data(*b).map_err(|_| FmtError)?;
                    }
                    self.write_data(0x00).map_err(|_| FmtError)?;
                }
            }
        }
        Ok(())
    }
}
