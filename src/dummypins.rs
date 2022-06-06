//! # Small module to provide "fake" pins
//!
//! This is used when for example we need to use a SPI
//! crate of some sort and don't want to use all pins
//! or if we want to implement the backlight code ourselves but we still
//! need to give a light pin to a new() function.
//!
//! Just assign a pin with a DummyPin and it wil behaive as a working pin,
//! but will actually do nothing.

use embedded_hal::digital::v2::{InputPin, OutputPin};

/// provides a dummy OutputPin.
///
/// This is very usable if a function requires a GPIO pin, but it's not necessary because there is
/// separate code to manage that function in the project.
/// This, for example, can be used for the LIGHT pin of the PCD8544.
#[derive(Debug, Clone, Copy)]
pub struct DummyOutputPin;

impl OutputPin for DummyOutputPin {
    type Error = core::convert::Infallible;
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Provides a dummy InputPin 
///
/// This is a very usable dummypin if you want to use
/// a pre-existing SPI crate on the PCD8544 which demands a
/// MISO pin. The PCD8544 has no MISO, only MOSI
/// So we feed it a pin which is allways low.
#[derive(Debug, Clone, Copy)]
pub struct DummyInputPin;

impl InputPin for DummyInputPin {
    type Error = core::convert::Infallible;
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }
}
