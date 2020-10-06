//! "Bit bang" SPI implementation

use core::marker::PhantomData;

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::spi::Write as SpiWrite;
use embedded_hal::blocking::delay::DelayUs;


/// "Bit bang" SPI implementation.
/// Use when you don't want to sacrifice a SPI port
pub struct BitBangSpi<ERR, CLK, DIN, DELAY> {
    clk: CLK,
    din: DIN,
    delay: DELAY,
    _phantom: PhantomData<ERR>
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
    pub fn new(
        mut clk: CLK,
        din: DIN,
    ) -> Result<BitBangSpi<ERR, CLK, DIN, NoDelay>, ERR> {
        clk.set_low()?;
        Ok(
            BitBangSpi { clk, din, delay: NoDelay{}, _phantom: PhantomData::default() }
        )
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
    ///
    /// ```rust
    /// let cp = cortex_m::peripheral::Peripherals::take().unwrap();
    /// let dwt = cp.DWT.constrain(cp.DCB, clocks);
    /// let delay = dwt.delay();
    ///
    /// let pcd_spi = BitBangSpi::new_with_delay(
    ///     pcd_clk,
    ///     pcd_din,
    ///     delay.clone(),
    /// ).unwrap();
    /// ```
    pub fn new_with_delay(
        mut clk: CLK,
        din: DIN,
        delay: DELAY
    ) -> Result<BitBangSpi<ERR, CLK, DIN, DELAY>, ERR> {
        clk.set_low()?;
        Ok(
            BitBangSpi { clk, din, delay, _phantom: PhantomData::default() }
        )
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
