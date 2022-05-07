//! Small example program for PCD8544 crate
//!
//! we first create a (fake) SPI from some dummy pins
//! and then create a display from the PCD8544 struct
//! with even more fake pins.
//!
//! Fake it till you make it! But this example gives a fair idea
//! how to use the library

use pcd8544::{bitbang::BitBangSpi, dummypins::DummyOutputPin, PCD8544};
use std::fmt::Write;

fn main() {
    let pcd_clk = DummyOutputPin;
    let pcd_din = DummyOutputPin;
    let pcd_spi = BitBangSpi::new(pcd_clk, pcd_din).expect("Infallible cannot fail");

    let pcd_dc = DummyOutputPin;
    let pcd_ce = DummyOutputPin;
    let pcd_rst = DummyOutputPin;
    let pcd_light = DummyOutputPin;

    let mut display =
        PCD8544::new(pcd_spi, pcd_dc, pcd_ce, pcd_rst, pcd_light).expect("Infallible cannot fail");

    display.reset().expect("Infallible cannot fail");
    writeln!(display, "Hello World").unwrap();
}
