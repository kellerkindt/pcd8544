# pcd8544 - Display driver crate

This crate implements the `Write` trait so that one can write text to the display.


[![Build Status](https://github.com/kellerkindt/pcd8544/workflows/Rust/badge.svg)](https://github.com/kellerkindt/pcd8544/actions?query=workflow%3ARust)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/kellerkindt/pcd8544)
[![Crates.io](https://img.shields.io/crates/v/pcd8544.svg)](https://crates.io/crates/pcd8544)
[![Documentation](https://docs.rs/pcd8544/badge.svg)](https://docs.rs/pcd8544)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/kellerkindt/pcd8544/issues/new)

# How to use
Below is an example how to create a new PCD8544 instance, initialize and write "Hello World" onto it.

```rust
fn main() -> ! {
    let mut cp: cortex_m::Peripherals = cortex_m::Peripherals::take().unwrap();
    let mut peripherals = stm32f103xx::Peripherals::take().unwrap();
    let mut rcc = peripherals.RCC.constrain();
    
    let mut gpioa = peripherals.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = peripherals.GPIOB.split(&mut rcc.apb2);
    
    let mut pcd_gnd   = gpiob.pb12.into_push_pull_output(&mut gpiob.crh);
    let mut pcd_light = gpiob.pb13.into_push_pull_output(&mut gpiob.crh);
    let mut pcd_vcc   = gpiob.pb14.into_push_pull_output(&mut gpiob.crh);
    let mut pcd_clk   = gpiob.pb15.into_push_pull_output(&mut gpiob.crh);
    let mut pcd_din   = gpioa.pa8 .into_push_pull_output(&mut gpioa.crh);
    let mut pcd_dc    = gpioa.pa9 .into_push_pull_output(&mut gpioa.crh);
    let mut pcd_ce    = gpioa.pa10.into_push_pull_output(&mut gpioa.crh);
    let mut pcd_rst   = gpioa.pa11.into_push_pull_output(&mut gpioa.crh);


    pcd_gnd  .set_low();
    pcd_light.set_high();
    pcd_vcc  .set_high();

    let mut display = PCD8544::new(
        pcd_clk,
        pcd_din,
        pcd_dc,
        pcd_ce,
        pcd_rst,
        pcd_light,
    ).expect("Infallible cannot fail");

    display.reset().expect("Infallible cannot fail");;
    writeln!(display, "Hello World");
    
    loop {}
}
```
The code from the example is copy&pasted from a working project, but not tested in this specific combination.
#### In action
The picture below shows the display to showing the temperature from the [onewire](https://github.com/kellerkindt/onewire/) [ds18b20](https://datasheets.maximintegrated.com/en/ds/DS18B20.pdf) sensor.
 
![](pcd8544.jpg) 