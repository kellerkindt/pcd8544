#![no_std]

mod backend;
mod font;

use core::fmt::Error as FmtError;
use core::fmt::Result as FmtResult;
use core::fmt::Write;

use embedded_hal::blocking;
use embedded_hal::digital::v2::OutputPin;

use crate::backend::*;
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

pub struct PCD8544<Backend, LIGHT, RST>
where
    Backend: PCD8544Backend,
    LIGHT: OutputPin,
    RST: OutputPin,
{
    backend: Backend,
    rst: RST,
    light: LIGHT,
    power_down_control: bool,
    entry_mode: bool,
    extended_instruction_set: bool,
    x: u8,
    y: u8,
}

impl<CLK, DIN, DC, CE, LIGHT, RST, ERR> PCD8544<PCD8544GpioBackend<CLK, DIN, DC, CE>, LIGHT, RST>
where
    CLK: OutputPin<Error = ERR>,
    DIN: OutputPin<Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
{
    pub fn new(clk: CLK, din: DIN, dc: DC, ce: CE, light: LIGHT, rst: RST) -> Result<Self, ERR> {
        Self::new_from_gpio(clk, din, dc, ce, light, rst)
    }

    pub fn new_from_gpio(
        clk: CLK,
        din: DIN,
        dc: DC,
        ce: CE,
        light: LIGHT,
        mut rst: RST,
    ) -> Result<Self, ERR> {
        let backend = PCD8544GpioBackend::new(clk, din, dc, ce)?;
        rst.set_low()?;
        Ok(PCD8544 {
            backend,
            rst,
            light,
            power_down_control: false,
            entry_mode: false,
            extended_instruction_set: false,
            x: 0,
            y: 0,
        })
    }
}

impl<SPI, DC, CE, LIGHT, RST, ERR, SPIERR> PCD8544<PCD8544SpiBackend<SPI, DC, CE>, LIGHT, RST>
where
    SPI: blocking::spi::Write<u8, Error = SPIERR>,

    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
{
    pub fn new_from_spi(spi: SPI, dc: DC, ce: CE, light: LIGHT, mut rst: RST) -> Result<Self, ERR> {
        let backend = PCD8544SpiBackend::new(spi, dc, ce)?;
        rst.set_low()?;
        Ok(PCD8544 {
            backend,
            rst,
            light,
            power_down_control: false,
            entry_mode: false,
            extended_instruction_set: false,
            x: 0,
            y: 0,
        })
    }
}

impl<Backend, LIGHT, RST> PCD8544<Backend, LIGHT, RST>
where
    Backend: PCD8544Backend,
    LIGHT: OutputPin<Error = Backend::Error>,
    RST: OutputPin<Error = Backend::Error>,
{
    pub fn reset(&mut self) -> Result<(), Backend::Error> {
        self.rst.set_low()?;
        self.x = 0;
        self.y = 0;
        self.init()
    }

    pub fn init(&mut self) -> Result<(), Backend::Error> {
        // reset the display
        self.rst.set_low()?;
        self.rst.set_high()?;

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

    pub fn clear(&mut self) -> Result<(), Backend::Error> {
        for _ in 0..(WIDTH as u16 * ROWS as u16) {
            self.write_data(0x00)?;
        }
        self.set_x_position(0)?;
        self.set_y_position(0)
    }

    pub fn set_power_down(&mut self, power_down: bool) -> Result<(), Backend::Error> {
        self.power_down_control = power_down;
        self.write_current_function_set()
    }

    pub fn set_entry_mode(&mut self, entry_mode: bool) -> Result<(), Backend::Error> {
        self.entry_mode = entry_mode;
        self.write_current_function_set()
    }

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

    pub fn set_light(&mut self, enabled: bool) -> Result<(), Backend::Error> {
        if enabled {
            self.light.set_low()
        } else {
            self.light.set_high()
        }
    }

    pub fn set_display_mode(&mut self, mode: DisplayMode) -> Result<(), Backend::Error> {
        self.write_command(0x08 | mode as u8)
    }

    pub fn set_bias_mode(&mut self, bias: BiasMode) -> Result<(), Backend::Error> {
        self.write_command(0x10 | bias as u8)
    }

    pub fn set_temperature_coefficient(
        &mut self,
        coefficient: TemperatureCoefficient,
    ) -> Result<(), Backend::Error> {
        self.write_command(0x04 | coefficient as u8)
    }

    /// contrast in range of 0..128
    pub fn set_contrast(&mut self, contrast: u8) -> Result<(), Backend::Error> {
        self.write_command(0x80 | contrast)
    }

    pub fn enable_extended_commands(&mut self, enable: bool) -> Result<(), Backend::Error> {
        self.extended_instruction_set = enable;
        self.write_current_function_set()
    }

    fn write_current_function_set(&mut self) -> Result<(), Backend::Error> {
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
    ) -> Result<(), Backend::Error> {
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

    fn write_command(&mut self, value: u8) -> Result<(), Backend::Error> {
        self.backend.write_byte(false, value)
    }

    fn write_data(&mut self, value: u8) -> Result<(), Backend::Error> {
        self.increase_position();

        self.backend.write_byte(true, value)
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
