//! # Module defining the instruction set of the PCD8544
//!
//! All instructions are implemented as enums and most arguments as well.
//! The source code is written to resemble the data sheet as close as possible.
//! The data sheet can be found here:
//! <https://www.sparkfun.com/datasheets/LCD/Monochrome/Nokia5110.pdf>
//! 
//! There's a lot of registers, settings and tweakings that can be done to the PCD8544 chip. That's
//! because it was designed to be able to drive all kinds of LCD's with different form factors and
//! different setups.
//!
//! It's probably wise to elaborate a bit on the settings.
//!
//! ## Contrast (SetVop)
//! This is the easiest one to grasp and the most logical one to change when somebody uses this
//! library. It's easy, just set the contrast with a value of 0..127. More is more contrast
//! (pixels are more black). It's very much possible to have a physical button, dial nob or menu
//! item where the contrast can be changed at runtime. 
//!
//! Most common values are around the middle. 
//! There is rumour that 49 works very nicely on the red pcb-colored Sparkfuns and 56 for the blue one. 
//! I find that somwewhat dim and use 65 as the go to value (right in the middle).
//!
//! To increase contrast the PCD8544 simply increase the operational voltage (Vops) to the LCD segments.
//! A LCD segment gets darker when applied more voltage. It's that simple. Until it breaks, that
//! is. I believe the actual "breaking voltage" is around 11V. Be carefull to not set the contrast
//! too high in lower temperatures, because the PCD8544 also adds voltage for temperature
//! compensation. Do not set Vops higher than 8.5V in very cold climates (-25 degrees Celsius).
//! 
//! The contrast can be easily changed with the SetVops() function within your projects code.
//!
//! ## Temperature coefficient
//! Liquid Crystal Displays are temperature sensitive, the viscosity of the LCD fluid changes, and
//! so the electrical conductancy.
//! The operational voltage needs to be increased with dropping temperatures.
//! Fortunately the PCD8544 has a temperature sensor build in so it can compensate automatically.
//! In order to tell the PCD8544 with how much it should increase voltage with decreasing
//! temperature, it needs a temperature coefficient set in its registers.
//! The temp coefficient just represents a lineair increase over temperature (getting higher with
//! lower temp). The crystals actually follow more of a curve of sensitivity, so it's an
//! approximation.
//! 
//! Unfortunately I couldn't find any information on the specific temperature characteristics of
//! the Nokia 5110 screen (the LCD itself, not the chip), so this library defaults to a coefficient
//! of TC2, which seems to be the consensus in all example code on the internet. Setting this
//! coefficient wrong can brick your screen but normally only gives problems at (very) low
//! temperatures. If the chrystals get too much voltage (above 8.1V I believe), they will be
//! damaged.
//!
//! Consider experimenting with this setting when constructing a project to be used for wintersports,
//! outdoors in cold climates or pole expeditions. Probably set it to TC1 and test in the real
//! world if the screen stays bright enough, and change to TC2 if not.
//! You can actually calculate (guestimate) the voltage by taking Vops (the contrast) and adding
//! the number of degrees colder than 27 degrees Celsius, times the coefficient voltage.
//!
//! ## Bias voltage and multiplex rate
//! The PCD8544 is designed for multiplex drived LCD screens. 
//! There are 4032 pixels on the screen implemented as LCD segments.
//! If it would be a static drive LCD, there would be 4032 control lines and a backplane line
//! running to the PCD8544, because every pixel would have a separate control line.
//! This is not practical so it's multiplex meaning the segments are on a grid, and the PCD8544
//! sends a somewhat complex waveform to this grid to turn on or off pixels on the intersections of
//! the grid. It needs to know the exact bias voltage for the grid of the screen to be able to
//! trigger the right amount of voltages to either cancel it out or add it up.
//! The standard Nokia 5110 screen has a multiplex ratio of 1:48.

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
/// The Temperature Coefficient.
///
/// This is an arbitrary number connected to a voltage (millivolt) which is added for every degree
/// Kelvin (K) that the temperature gets lower.
/// So TC1 adds 9 millivolt to the LCD driver voltage per every degree Kelvin the temperature gets
/// lower. The "zero temperature" at which there is no added voltage, is at 27 degrees Celsius or
/// 300.15 Kelvin, as I understand the datasheet correctly.
pub enum TemperatureCoefficient {

    /// Adds 1 mV/K 
    TC0 = 0b00,

    /// Adds 9 mV/K 
    TC1 = 0b01,

    /// Adds 17 mV/K
    TC2 = 0b10,

    /// Adds 24 mV/K
    TC3 = 0b11,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
/// The bias voltage for multiplexing the LCD glass.
///
/// We need to tell the PCD8544 how the specific LCD glass uses multiplexing.
/// The PCD8544 need a bias voltage value for that.
/// There is a tabie in the datasheet to correlate bias voltage with multiplex ratios.
/// The multiplex ratio of an LCD screen (glass) is fixed and is 1:48 on the common 5110 screen.
/// Change the bias voltage if you use the PCD8544 chip with other LCD's than the Nokia 5110.
pub enum MuxRate {
    /// bias voltage for LCD with multiplex ratio 1/100
    Bias1To100 = 0,
    /// bias voltage for LCD with multiplex ratio 1/80 
    Bias1To80 = 1,
    /// bias voltage for LCD with multiplex ratio 1/65 
    Bias1To65 = 2,
    /// bias voltage for LCD with multiplex ratio 1/48, this is the Nokia 5110.
    Bias1To48 = 3,
    /// bias voltage for LCD with multiplex ratio 1/40
    Bias1To40 = 4,
    /// bias voltage for LCD with multiplex ratio 1/24
    Bias1To24 = 5,
    /// bias voltage for LCD with multiplex ratio 1/18
    Bias1To18 = 6,
    /// bias voltage for LCD with multiplex ratio 1/10
    Bias1To10 = 7,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
/// The display mode (normal, inversed, all black or all blank).
pub enum DisplayMode {

    /// Show a blank screen (all pixels off)
    DisplayBlank = 0b000,

    /// Show image from DDRAM buffer, pixel "on/true" is a black pixel/dot
    Normal = 0b100, 

    /// Show a black screen (all pixels on)
    AllSegmentsOn = 0b001,

    /// Show reverse image from DDRAM buffer, pixel "off/false" is a black pixel/dot
    InverseVideo = 0b101,
}

/// PCD8544 instruction set, both basic and "extended" instructions, see data sheet
///
/// Set differend modes via the lowest 3 bits in the FunctionSet command.
/// PD -> 0=active 1=power down | V -> 0=horizontal 1=vertical adressing | H -> 0=basic 1=extended instruction
#[derive(Debug, Clone, Copy)]
pub enum Instruction {

    /// no operation, do nothing
    // translates to: 0x00
    NOP,

    /// set basic functions:  power down, entry mode (horizontal or vertical) and instruction set
    // translates to: 0x10 OR with lowest three bits PD, V, H
    FunctionSet{ 

        /// pd means Power Down, if true then the display will power off (standby mode)
        pd: bool,

        /// v means Vertical mode, if true bytes written to DDRAM will follow a column pattern 
        v: bool,

        /// h means Extended Instruction Set, if true PCD accepts extended instructions, else only
        /// basic instructions.
        h: bool },

    /// set display configuration
    // translates to: 0x08 OR with three bit DisplayMode, see DisplayMode enum  
    SetDisplayMode(DisplayMode),

    /// set Y address of DDRAM; 0 =< Y =< 5.
    // translates to: 0x20 OR with 0 <= Y <= 5 to set Y RAM Address
    Yaddress(u8),

    /// set X address of DDRAM; 0 =< X =< 83.
    // translates to: 0x40 OR with 0 <= X <= 83 to set X RAM Address
    Xaddress(u8),

    /// set which curve is used in LCD voltage regulation for temperature difference
    // translates to: 0x04 OR with TC0 t/m TC3 (TemperatureCoefficient)
    SetTempCoefficient(TemperatureCoefficient),

    /// configure the bias voltage level, using the bias multiplex ratio (muxratea).
    // translates to: 0x10 OR with MuxRate
    SetBiasMode(MuxRate),

    /// set contrast
    // translates to: 0x80 OR with 7 bits contrast value (0-127)
    SetVop(u8),
}

use Instruction::*;

// Use full binary presentation instead of hex, to look the same as data sheet
impl Instruction {

    /// Returns the specific instruction as a byte (u8) which can be send over a wire
    ///
    /// use this function to generate input for something that does a Write<u8>.
    pub fn byte(self) -> u8 {
        match self {
            NOP => 0b0000_0000,
            FunctionSet{pd, v, h} => {
                0b0010_0000 | ((pd as u8) << 2) | ((v as u8) << 1) | h as u8
            },
            SetDisplayMode(mode) => 0b0000_1000 | mode as u8,
            Yaddress(y) => {
                assert!(y < 6);
                0b0100_0000 | y
            },
            Xaddress(x) => {
                assert!(x <= 84);
                0b1000_0000 | x
            },
            SetTempCoefficient(coeff) => 0b0000_0100 | coeff as u8,
            SetBiasMode(mux) => 0b0001_0000 | mux as u8,
            SetVop(contrast) => 0b1000_0000 | contrast,
        }
    }

    /// Returns a boolean if this is a basic ('false') or extended ('true') command
    ///
    /// NOP and FunctionSet work in both modes and shouldn't need this function, but it is
    /// implemented anyway (as "false") to not get into gritty errors and incompatibilities.
    pub fn mode(&self) -> bool {
        match &*self {
            NOP => false,
            FunctionSet{pd:_, v:_, h:_} => false,
            SetDisplayMode(_) => false,
            Yaddress(_) => false,
            Xaddress(_) => false,
            SetTempCoefficient(_) => true,
            SetBiasMode(_) => true,
            SetVop(_) => true,
        }
    }
}

/// A prelude for convenience, it pulls all enums and traits into scope, for convenience.
pub mod prelude {
    pub use super::{
        TemperatureCoefficient, TemperatureCoefficient::*,
        MuxRate, MuxRate::*,
        DisplayMode, DisplayMode::*,
        Instruction, Instruction::*,
    };
}
