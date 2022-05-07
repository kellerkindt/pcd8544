//! "Bit bang" half duplex SPI implementation
//! perfect for PCD8544 which doesn't have/need full duplex. 
//! This code is much more efficient for the PCD8544 than the bitbang-hal crate
//!
//! Use BitBangSpi.new() for slow boards who do not need a delay 
//! and BitBangSpi.new_with_delay() for fast(er) boards.
//!
//! Created by andreyk0
//! https://github.com/andreyk0/pcd8544/blob/master/src/spi.rs
//!
//! Only added some documentation to this

use core::marker::PhantomData;

use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::blocking::spi::Write as SpiWrite;
use embedded_hal::digital::v2::OutputPin;

/// "Bit bang" SPI implementation.
/// Use when you don't want to sacrifice a SPI port
/// or you want your PCD8544 to line up perfectly with your board
/// and SPI doesn't line up that nice
/// see example picture in README, with a Blue Pill.
pub struct BitBangSpi<ERR, CLK, DIN, DELAY> {
    clk: CLK,
    din: DIN,
    delay: DELAY,
    _phantom: PhantomData<ERR>,
}

/// Used to run without delay on a slow enough clock speed (below 8Mhz)
pub struct NoDelay {}

impl DelayUs<u8> for NoDelay {
    #[inline]
    fn delay_us(&mut self, _us: u8) {}
}

impl<ERR, CLK, DIN> BitBangSpi<ERR, CLK, DIN, NoDelay>
where
    CLK: OutputPin<Error = ERR>,
    DIN: OutputPin<Error = ERR>,
{
    /// Constructs a "bit bang" SPI implementation from "data in" and "clock" pins.
    /// If your clock frequency is higher than 8Mhz please consider `new_with_delay`,
    /// otherwise device won't work.
    pub fn new(mut clk: CLK, din: DIN) -> Result<BitBangSpi<ERR, CLK, DIN, NoDelay>, ERR> {
        clk.set_low()?;
        Ok(BitBangSpi {
            clk,
            din,
            delay: NoDelay {},
            _phantom: PhantomData::default(),
        })
    }
}

impl<ERR, CLK, DIN, DELAY> BitBangSpi<ERR, CLK, DIN, DELAY>
where
    CLK: OutputPin<Error = ERR>,
    DIN: OutputPin<Error = ERR>,
    DELAY: DelayUs<u8>,
{
    /// Constructs a "bit bang" SPI implementation from "data in" and "clock" pins
    /// with a clock delay. Please use this variant for clock speeds higher than 8Mhz.
    pub fn new_with_delay(
        mut clk: CLK,
        din: DIN,
        delay: DELAY,
    ) -> Result<BitBangSpi<ERR, CLK, DIN, DELAY>, ERR> {
        clk.set_low()?;
        Ok(BitBangSpi {
            clk,
            din,
            delay,
            _phantom: PhantomData::default(),
        })
    }

    #[inline]
    fn write_bit(&mut self, high: bool) -> Result<(), ERR> {
        if high {
            self.din.set_high()?;
        } else {
            self.din.set_low()?;
        }
        self.clk.set_high()?;
        self.delay.delay_us(1);
        self.clk.set_low()?;
        self.delay.delay_us(1);
        Ok(())
    }
}

impl<ERR, CLK, DIN, DELAY> SpiWrite<u8> for BitBangSpi<ERR, CLK, DIN, DELAY>
where
    CLK: OutputPin<Error = ERR>,
    DIN: OutputPin<Error = ERR>,
    DELAY: DelayUs<u8>,
{
    type Error = ERR;

    #[inline]
    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        for value in words {
            let mut v = *value;
            for _ in 0..8 {
                self.write_bit((v & 0x80) == 0x80)?;
                v <<= 1;
            }
        }
        Ok(())
    }
}
