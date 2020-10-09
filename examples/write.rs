use embedded_hal::digital::v2;
use pcd8544::{spi::BitBangSpi, PCD8544};
use std::convert::Infallible;
use std::fmt::Write;

pub struct DummyOutputPin;

impl v2::OutputPin for DummyOutputPin {
    type Error = Infallible;
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() {
    let pcd_light = DummyOutputPin;
    let pcd_clk = DummyOutputPin;
    let pcd_din = DummyOutputPin;
    let pcd_dc = DummyOutputPin;
    let pcd_ce = DummyOutputPin;
    let pcd_rst = DummyOutputPin;

    let spi = BitBangSpi::new(pcd_clk, pcd_din).expect("Infallible cannot fail");
    let mut display =
        PCD8544::new(spi, pcd_dc, pcd_ce, pcd_rst, pcd_light).expect("Infallible cannot fail");

    display.reset().expect("Infallible cannot fail");
    writeln!(display, "Hello World").unwrap();
}
