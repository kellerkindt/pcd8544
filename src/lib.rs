#![no_std]
use core::fmt::Error as FmtError;
use core::fmt::Result as FmtResult;
use core::fmt::Write;
use embedded_hal::blocking::spi::Write as SpiWrite;
use embedded_hal::digital::v2::OutputPin;

pub mod dummypins;
pub mod bitbang;

mod font;
use crate::font::char_to_bytes;

pub const WIDTH: u8 = 84;
pub const HEIGHT: u8 = 48;
pub const ROWS: u8 = HEIGHT / 8;

#[repr(u8)]
pub enum TemperatureCoefficient {
    TC0 = 0b00,  // 0 
    TC1 = 0b01,  // 1
    TC2 = 0b10,  // 2
    TC3 = 0b11,  // 3
}

use TemperatureCoefficient::*;

#[repr(u8)]
pub enum MuxRate {
    Bias1To100 = 0,
    Bias1To80 = 1,
    Bias1To65 = 2,
    Bias1To48 = 3,
    Bias1To40 = 4,
    Bias1To24 = 5,
    Bias1To18 = 6,
    Bias1To10 = 7,
}

use MuxRate::*;


#[repr(u8)]
pub enum DisplayMode {
    DisplayBlank = 0b000,
    Normal = 0b100,
    AllSegmentsOn = 0b001,
    InverseVideo = 0b101,
}

use DisplayMode::*;

// PCD8544 instruction set, both basic and "extended" instructions, see data sheet
// Set differend modes via the lowest 3 bits in the FunctionSet command
// PD -> 0=active 1=power down | V -> 0=horizontal 1=vertical adressing | H -> 0=basic 1=extended instruction
pub enum Instruction {
    NOP,                                         // 0x00,
    FunctionSet{ pd: bool, v: bool, h: bool },   // 0x10 OR with lowest three bits PD, V, H (this is Pw
    DisplayMode(DisplayMode),                    // 0x08 OR with three bit DisplayMode, see DisplayMode enum  
    Yaddress(u8),                                // 0x20 OR with 0 <= Y <= 5 to set Y RAM Address
    Xaddress(u8),                                // 0x40 OR with 0 <= X <= 83 to set X RAM Address
    SetTempCoefficient(TemperatureCoefficient),  // 0x04 OR with TC0 t/m TC3 (TemperatureCoefficient)
    SetBiasMode(MuxRate),                        // 0x10 OR with MuxRate
    SetVop(u8),                                  // 0x80 OR with 7 bits contrast value (0-127), written to Vop (Set Vop)
}

// Use full binary presentation instead of hex, to j
impl Instruction {
    fn byte(self) -> u8 {
        match self {
            NOP => 0b0000_0000,
            FunctionSet{pd, v, h} => {
                0b0010_0000 | ((pd as u8) << 2) | ((v as u8) << 1) | h as u8
            },
            DisplayMode(mode) => 0b0000_1000 | mode as u8,
            Yaddress(y) => {
                assert!(y < 6);
                0b0100_0000 | y as u8
            },
            Xaddress(x) => {
                assert!(x <= 84);
                0b1000_0000 | x as u8
            },
            SetTempCoefficient(coeff) => 0b0000_0100 | coeff as u8,
            SetBiasMode(mux) => 0b0001_0000 | mux as u8,
            SetVop(contrast) => 0b1000_0000 | contrast as u8,
        }
    }

    // return `false` for a basic command and `true` for an extended command
    // NOP and FunctionSet work in both modes and should never use this
    fn mode(&self) -> bool {
        match &*self {
            NOP => false,
            FunctionSet{pd:_, v:_, h:_} => false,
            DisplayMode(_) => false,
            Yaddress(_) => false,
            Xaddress(_) => false,
            SetTempCoefficient(_) => true,
            SetBiasMode(_) => true,
            SetVop(_) => true,
        }
    }
}

use Instruction::*;

pub struct PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8>,
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
    x: u8,
    y: u8,
}

impl<SPI, DC, CE, RST, LIGHT, ERR> PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8, Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
{
    pub fn new(
        spi: SPI,
        dc: DC,
        ce: CE,
        rst: RST,
        light: LIGHT,
    ) -> Result<PCD8544<SPI, DC, CE, RST, LIGHT>, ERR> {

        let mut pcd = PCD8544 {
            spi, 
            dc,
            ce,
            rst,
            light,
            power_down_control: false,
            entry_mode: false,
            x: 0,
            y: 0,
        };

        pcd.reset()?;
        pcd.init()?;

        Ok(pcd)
    }

    fn init_sequence(&mut self) -> [u8; 6] {
        [ 
            FunctionSet{pd: false, v: false, h: true}.byte(),   // PD=0 (chip active); V=0 (horizontal addressing mode); H=1 (extended instruction set)
            SetVop(65).byte(),                                  // set contrast, try 49 (for 3.3V red SparkFun) or 56 (for 3.3V blue SparkFun), range is 0 to 127
            SetTempCoefficient(TC2).byte(),
            SetBiasMode(Bias1To48).byte(),                      // LCD bias mode 1:48, else try 1:40 (Bias1To40)
            FunctionSet{pd: false, v: false, h: false}.byte(),  // PD=1 (chip active); V=0 (horizontal addressing mode); H=0 (basic instruction set)
            DisplayMode(Normal).byte(),
        ]
    }

    pub fn reset(&mut self) -> Result<(), ERR> {
        self.x = 0;
        self.y = 0;
        self.init()
    }

    // Send a bunch of bytes using SPI
    // change this function if you want to bitbang
    fn send(&mut self, bytes: &[u8]) -> Result<(), ERR> {
        self.ce.set_low()?;
        self.spi.write(bytes)?;
        self.ce.set_high()?;
        Ok(())
    }

    // send array of commands (bytes)
    // DC low means "Send commands"
    fn write_commands(&mut self, commands: &[u8]) -> Result<(), ERR> {
        self.dc.set_low()?;
        self.send(commands)?;
        Ok(())
    }

    // send one command, preseeded by an basic/extended set command
    // DC low means "Send commands"
    fn write_command(&mut self, instruction: Instruction) -> Result<(), ERR> {
        self.write_commands(&[ 
            FunctionSet {
                pd: self.power_down_control,
                v: self.entry_mode,
                h: instruction.mode(),
            }.byte(),
            instruction.byte(),
        ])
    }

    // send array of data (bytes)
    // DC high means "Send data"
    fn write_data(&mut self, data: &[u8]) -> Result<(), ERR> {
        self.dc.set_high()?;
        self.send(data)?;
        self.increase_position();
        self.dc.set_low()?; // unnecessary, but just to be consistent
        Ok(())
    }

    pub fn init(&mut self) -> Result<(), ERR> {
        // reset the display
        self.rst.set_low()?;
        self.rst.set_high()?;

        // reset state variables
        self.power_down_control = false;
        self.entry_mode = false;

        // write init configuration
        let opcodes = self.init_sequence();
        self.write_commands(&opcodes)?;

        // clear display data
        self.clear()?;
        Ok(())
    }
    
    pub fn clear(&mut self) -> Result<(), ERR> {
        for _ in 0..(WIDTH as u16 * ROWS as u16) {
            self.write_data(&[0x00])?;
        }
        self.set_x_position(0)?;
        self.set_y_position(0)
    }

    pub fn set_power_down(&mut self, power_down: bool) -> Result<(), ERR> {
        self.power_down_control = power_down;
        self.write_commands(&[ 
            FunctionSet {
                pd: self.power_down_control,
                v: self.entry_mode,
                h: false,
            }.byte(),
        ])
    }

    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }

    pub fn set_x_position(&mut self, x: u8) -> Result<(), ERR> {
        let x = x % WIDTH;
        self.x = x;
        self.write_command(Xaddress(x))
    }

    pub fn set_y_position(&mut self, y: u8) -> Result<(), ERR> {
        let y = y % ROWS;
        self.y = y;
        self.write_command(Yaddress(y))
    }

    pub fn set_light(&mut self, enabled: bool) -> Result<(), ERR> {
        if enabled {
            self.light.set_low()
        } else {
            self.light.set_high()
        }
    }

    pub fn set_display_mode(&mut self, mode: DisplayMode) -> Result<(), ERR> {
        self.write_command(DisplayMode(mode))
    }

    pub fn set_bias_mode(&mut self, bias: MuxRate) -> Result<(), ERR> {
        self.write_command(SetBiasMode(bias))
    }

    pub fn set_temperature_coefficient(
        &mut self,
        coefficient: TemperatureCoefficient,
    ) -> Result<(), ERR> {
        self.write_command(SetTempCoefficient(coefficient))
    }

    // contrast in range of 0..128, automatically capped in the Enum
    pub fn set_contrast(&mut self, contrast: u8) -> Result<(), ERR> {
        self.write_command(SetVop(contrast))
    }

    fn increase_position(&mut self) {
        self.x = (self.x + 1) % WIDTH;
        if self.x == 0 {
            self.y = (self.y + 1) & ROWS;
        }
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
    fn write_str(&mut self, s: &str) -> FmtResult {
        for c in s.chars() {
            match c {
                '\r' => {
                    self.set_x_position(0).map_err(|_| FmtError)?;
                }
                '\n' => {
                    for _ in 0..(WIDTH - self.x) {
                        self.write_data(&[0x00]).map_err(|_| FmtError)?;
                    }
                }
                _ => {
                    self.write_data(char_to_bytes(c)).map_err(|_| FmtError)?;
                    self.write_data(&[0x00]).map_err(|_| FmtError)?;
                }
            }
        }
        Ok(())
    }
}
