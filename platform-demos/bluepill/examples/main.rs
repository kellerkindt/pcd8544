#![deny(unsafe_code)]
#![allow(clippy::empty_loop)]
#![deny(warnings)]
#![no_std]
#![no_main]
extern crate cortex_m;
extern crate embedded_hal;
extern crate stm32f1xx_hal as hal;

use panic_halt as _;

use cortex_m_rt::entry;
use hal::{pac, prelude::*};
use core::fmt::Write;

use pcd8544::PCD8544;
use pcd8544::bitbang::BitBangSpi;

#[entry]
fn main() -> ! {

    // Get access to core and device peripherals and raw flash and rcc
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();

    // Freeze clocks
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    // Configure the syst timer to trigger an update every second
    let mut timer = cp.SYST.counter_hz(&clocks);
    timer.start(1.Hz()).unwrap();

    // Acquire the GPIO peripherals
    let mut gpioa = dp.GPIOA.split();
    let mut gpiob = dp.GPIOB.split();
    
    let mut pcd_gnd   = gpiob.pb12.into_push_pull_output(&mut gpiob.crh);
    let mut pcd_light = gpiob.pb13.into_push_pull_output(&mut gpiob.crh);
    let mut pcd_vcc   = gpiob.pb14.into_push_pull_output(&mut gpiob.crh);
    let pcd_clk   = gpiob.pb15.into_push_pull_output(&mut gpiob.crh);
    let pcd_din   = gpioa.pa8 .into_push_pull_output(&mut gpioa.crh);
    let pcd_dc    = gpioa.pa9 .into_push_pull_output(&mut gpioa.crh);
    let pcd_ce    = gpioa.pa10.into_push_pull_output(&mut gpioa.crh);
    let pcd_rst   = gpioa.pa11.into_push_pull_output(&mut gpioa.crh);

    let pcd_spi = BitBangSpi::new(pcd_clk, pcd_din).unwrap();

    pcd_gnd  .set_low();
    pcd_light.set_high();
    pcd_vcc  .set_high();

    let mut display = PCD8544::new(
        pcd_spi,
        pcd_dc,
        pcd_ce,
        pcd_rst,
        pcd_light,
    ).expect("Infallible cannot fail");

    display.reset().expect("Infallible cannot fail");
    writeln!(display, "Hello World").expect("Infallible cannot fail");
    
    loop {}
}
