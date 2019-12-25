extern crate embedded_hal;
extern crate pcd8544;

use embedded_hal::digital::{v1_compat::OldOutputPin, v2};
use pcd8544::PCD8544;
use std::fmt::Write;

pub struct DummyOutputPin {}

impl DummyOutputPin {
    pub fn new() -> Self {
        DummyOutputPin {}
    }
}

impl v2::OutputPin for DummyOutputPin {
    type Error = ();
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() -> () {
    let pcd_light: OldOutputPin<_> = DummyOutputPin::new().into();
    let pcd_clk: OldOutputPin<_> = DummyOutputPin::new().into();
    let pcd_din: OldOutputPin<_> = DummyOutputPin::new().into();
    let pcd_dc: OldOutputPin<_> = DummyOutputPin::new().into();
    let pcd_ce: OldOutputPin<_> = DummyOutputPin::new().into();
    let pcd_rst: OldOutputPin<_> = DummyOutputPin::new().into();

    let mut display = PCD8544::new(pcd_clk, pcd_din, pcd_dc, pcd_ce, pcd_rst, pcd_light);

    display.reset();
    writeln!(display, "Hello World").unwrap();
}
