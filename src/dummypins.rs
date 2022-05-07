//! Small module to provide "fake" pins
//! This is used when for example we need to use a SPI
//! crate of some sort and don't want to use all pins
//! or if we want to implement the backlight code ourselves
//! in our main.rs

use embedded_hal::digital::v2::{InputPin, OutputPin};

/// Provides a dummy for the LIGHT parameter in PCD8544::init() if not used.
/// and can also be used for other things
///
/// PCD8544 needs a light pin in its init() function.
/// Some want to use their own backlight management,
/// for example use PWM. Or they use a board with pin-low is on.
/// Therefore we provide a dummy pin to feed to the init() function.

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
