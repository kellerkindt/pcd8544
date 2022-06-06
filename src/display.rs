//! # Main driver library for the PCD8544
//!
//! This contains shared code between the two feature-flag enabled modes, namely TextMode and
//! Graphics. See the documentation of the similarly named modules to get information on those
//! more advanced functions.
//! In here most of the data structures and functions care about state keeping of the display
//! status and sending raw bytes to the device.
//!
//! This module exports the main PCD8544 struct and traits.
//! To use the driver, make a PCD8544.new() and call the methods on that.
//! Users of this module typically only use the "set" commands to (re)configure the display. The
//! more advanced stuff is handles by TextMode and/or Graphics modules, which use the more
//! low-level functions of this driver.
//!
//! Typically you want to look into the following funcions:
//!  - PCD8544.new() to create a new driver instance
//!  - PCD8544.set_contrast() to change the contrast of the display
//!  - PCD8544.set_light() to enable or disable the backlight.
//! The other functions are more "advanced" and only used in specific cases. You can change de
//! display mode for example to do inverse colors, or put the screen in "sleep mode" with power
//! down control funcions.
use hal::blocking::spi::Write as SpiWrite;
use hal::digital::v2::OutputPin;

use crate::{
    DDRAM_SIZE,
    instructions::prelude::*,
    error::*,
};

#[derive(Debug)]
/// main struct for state keeping of the PCD8544 driver, and as a spine to hold all the traits
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
    #[cfg(feature = "textmode")]
    /// column (x-axis) of the text cursor. should be in range 0..84
    pub text_col: u8,
    #[cfg(feature = "textmode")]
    /// row (y-axis) of the text cursor. should be in range 0..6
    pub text_row: u8,
    #[cfg(feature = "graphics")]
    /// buffer for drawing graphics in memory with the same size as the PCD DDRAM.
    pub framebuffer: [u8; DDRAM_SIZE],
}

impl<SPI, DC, CE, RST, LIGHT, ERR> PCD8544<SPI, DC, CE, RST, LIGHT>
where
    SPI: SpiWrite<u8, Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
    RST: OutputPin<Error = ERR>,
    LIGHT: OutputPin<Error = ERR>,
{
    /// Create a new instance of the PCD8544 driver
    ///
    /// Arguments:
    ///
    /// - spi: a working SPI interface implementing Write<u8>. Can be a dummy driver
    /// - dc: Data/Command, a GPIO OutputPin connected to DC on the display
    /// - ce: Chip Enable, a GPIO OutputPin connected to CE on the display
    /// - rst: Reset, a GPIO OutputPin connected to RST on the display
    /// - Light: a GPIO OutputPin connected to the backlight connector of the display
    pub fn new(
        spi: SPI,
        dc: DC,
        ce: CE,
        rst: RST,
        light: LIGHT,
    ) -> Result<PCD8544<SPI, DC, CE, RST, LIGHT>, PCDError> {

        let mut pcd = PCD8544 {
            spi, 
            dc,
            ce,
            rst,
            light,
            power_down_control: false,
            entry_mode: false,
            #[cfg(feature = "textmode")]
            text_col: 0,
            #[cfg(feature = "textmode")]
            text_row: 0,
            #[cfg(feature = "graphics")]
            framebuffer: [0u8;  DDRAM_SIZE],
        };

        // resetting the display at initial startup is _mandatory_
        // because the PCD8544 is in an undefined state at power on of the chip
        // and you can actually break the display by not resetting and just start using.
        pcd.reset()?;

        Ok(pcd)
    }

    /// the reset() function hardware resets and initializes the PDC8544.
    ///
    /// this means hardware reset and initial configuration. It also clears the DDRAM.
    /// The PCD8544.new() function calls the reset() function, so it's not a function with a lot of
    /// use during projects. It can be used in error situations or to "restart" your project with a
    /// button or something.
    pub fn reset(&mut self) -> Result<(), PCDError> {

        // hardware reset the PCD8544 with the RST pin
        PCDError::pin(self.rst.set_low())?;

        // should there be a delay here?
        // datasheet says something like "within 100ms of Vdd high"
        // we could also just send NOPs for a while, misusing SPI timer...
        //delay.delay_ms(10);

        PCDError::pin(self.rst.set_high())?;

        // reset state variables
        self.power_down_control = false;
        self.entry_mode = false;

        // write initial configuration to PCD8544
        let opcodes = self.init_sequence();
        self.write_commands(&opcodes)?;

        // clear display data
        self.hw_cls()?;
        Ok(())
    }
    
    // this is the pre-programmed init sequence. It doesn't automatically switch from basic to
    // extended mode, like write_command() does, so take care to switch mode via FunctionSet{}.
    // TODO: We should definately parameterize this by using use some sort of Enum for different presets of
    // screens like Sparkfun and Adafruit, and a custom setting.
    fn init_sequence(&mut self) -> [u8; 6] {
        [ 
            FunctionSet{pd: false, v: false, h: true}.byte(),
            SetVop(65).byte(),
            SetTempCoefficient(TC2).byte(),
            SetBiasMode(Bias1To48).byte(),
            FunctionSet{pd: false, v: false, h: false}.byte(),
            SetDisplayMode(Normal).byte(),
        ]
    }
    
    /// clears the screen by zeroing the DDRAM in the PCD8544 chip
    ///
    /// This function sends directly to the DDRAM.
    /// It calculates the number of bytes to be written (width * banks) 
    /// which should be 504 bytes, and just sends 504 bytes of 0x00 to the DDRAM
    /// name of the function is an abbreviation of "hardware clearscreen".
    pub fn hw_cls(&mut self) -> Result<(), PCDError> {

        // fill the DDRAM with zeroes
        for _ in 0..(DDRAM_SIZE) {
            self.write_data(&[0x00])?;
        }

        // reset the hardware cursor
        self.write_command(Xaddress(0))?;
        self.write_command(Yaddress(0))?;
  
        Ok(())
    }

    // Send a bunch of bytes using SPI to the PCD8544
    // Chip Enable "activates" the PCD8544 to listen to SPI if pin is low.
    fn send(&mut self, bytes: &[u8]) -> Result<(), PCDError> {
        PCDError::pin(self.ce.set_low())?;
        PCDError::spi(self.spi.write(bytes))?;
        PCDError::pin(self.ce.set_high())?;
        Ok(())
    }

    // send array of commands (bytes)
    // DC low means "Send commands" and not data, so it gets to the registers and not the DDRAM.
    // It is only used in the init() function, so no need to "pub" it.
    // Users of the library typically use the write_command function to tweak specific settings.
    fn write_commands(&mut self, commands: &[u8]) -> Result<(), PCDError> {
        PCDError::pin(self.dc.set_low())?;
        self.send(commands)?;
        Ok(())
    }

    /// Send a single command to the PCD8544 chip.
    ///
    /// The sent command is automatically preseeded by the proper "mode" command
    /// as in basic or extended instruction, so you do not have to worry about changing
    /// modes.
    /// You feed it an Instruction from the Instruction Enum, with arguments.
    // DC low means "Send commands" so it ends up in the registers, not the DDRAM.
    pub fn write_command(&mut self, instruction: Instruction) -> Result<(), PCDError> {
        self.write_commands(&[ 
            FunctionSet {
                pd: self.power_down_control,
                v: self.entry_mode,
                h: instruction.mode(),
            }.byte(),
            instruction.byte(),
        ])
    }

    /// Send a buffer of data to the PCD8544 in "data mode".
    ///
    /// If the TextMode feature is on, it also increases the text cursor position.
    // DC high means "Send data" so it directly ends up in DDRAM.
    // This function should also increase te position/cursor, which it doesn't at the moment.
    // We probably need to make a special write function inside textmode.rs
    pub fn write_data(&mut self, data: &[u8]) -> Result<(), PCDError> {
        PCDError::pin(self.dc.set_high())?;
        self.send(data)?;
        PCDError::pin(self.dc.set_low())?; // unnecessary, but just to be consistent
        Ok(())
    }

    /// Power down or up the PCD8544 chip. the one argument is a boolean. true means screen off.
    ///
    /// This is a hardware power down function which was mainly used for energy preservation
    /// ("sleep mode") in mobile phones like the Nokia 3320/5110.
    /// This can be very usefull in a project which needs to conserve energy.
    pub fn set_power_down(&mut self, power_down: bool) -> Result<(), PCDError> {
        self.power_down_control = power_down;
        self.write_commands(&[
            FunctionSet {
                pd: self.power_down_control,
                v: self.entry_mode,
                h: false,
            }.byte(),
        ])
    }

    /// Change the display mode, talks directly to the PCD8544 hardware.
    ///
    /// Only arguments are DisplayBlank, Normal, AllSegmentsOn or InverseVideo.
    /// See the DisplayMode Enum for more explanation.
    pub fn set_display_mode(&mut self, mode: DisplayMode) -> Result<(), PCDError> {
        self.write_command(SetDisplayMode(mode))
    }

    /// Change the bias mode, talks directly to the PCD8544 hardware.
    ///
    /// It expects a MuxRate enum with the preferred bias rate.
    /// This should be fixed at 1:48 for the tipical Nokia 5110 screen, 
    /// so it probably is'nt a very much used function. 
    pub fn set_bias_mode(&mut self, bias: MuxRate) -> Result<(), PCDError> {
        self.write_command(SetBiasMode(bias))
    }

    /// Set the temperature coefficient from the range TC0 to TC4
    ///
    /// Use the TemperatureCoefficient enum as an argument. 
    /// See the documentation in the Instructions module for an explanation.
    pub fn set_temperature_coefficient(
        &mut self,
        coefficient: TemperatureCoefficient,
    ) -> Result<(), PCDError> {
        self.write_command(SetTempCoefficient(coefficient))
    }

    /// Set the Vop and therefore the contrast of the LCD
    ///
    /// feed the function an u8 contrast value in range of 0..128
    /// the value automatically gets capped to max value (127) if it is 128 or higher
    pub fn set_contrast(&mut self, contrast: u8) -> Result<(), PCDError> {
        self.write_command(SetVop(contrast))
    }

    /// Enable or disable the backlight
    ///
    /// this function assumes a display where you need to pull the light pin low (0V) to
    /// activate the backlight. This can be different per manufacturer unfortunately
    pub fn set_light(&mut self, enabled: bool) -> Result<(), PCDError> {
        if enabled {
            PCDError::pin(self.light.set_low())
        } else {
            PCDError::pin(self.light.set_high())
        }
    }

}
