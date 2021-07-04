use embedded_hal::{blocking, digital::v2::OutputPin};

pub trait PCD8544Backend {
    type Error;
    fn write_byte(&mut self, data: bool, value: u8) -> Result<(), Self::Error>;
}

pub struct PCD8544GpioBackend<CLK, DIN, DC, CE>
where
    CLK: OutputPin,
    DIN: OutputPin,
    DC: OutputPin,
    CE: OutputPin,
{
    clk: CLK,
    din: DIN,
    dc: DC,
    ce: CE,
}

impl<CLK, DIN, DC, CE, ERR> PCD8544GpioBackend<CLK, DIN, DC, CE>
where
    CLK: OutputPin<Error = ERR>,
    DIN: OutputPin<Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
{
    pub fn new(
        clk: CLK,
        din: DIN,
        dc: DC,
        mut ce: CE,
    ) -> Result<PCD8544GpioBackend<CLK, DIN, DC, CE>, ERR> {
        ce.set_high()?;
        Ok(PCD8544GpioBackend { clk, din, dc, ce })
    }

    fn write_bit(&mut self, high: bool) -> Result<(), ERR> {
        if high {
            self.din.set_high()?;
        } else {
            self.din.set_low()?;
        }
        self.clk.set_high()?;
        self.clk.set_low()
    }
}

impl<CLK, DIN, DC, CE, ERR> PCD8544Backend for PCD8544GpioBackend<CLK, DIN, DC, CE>
where
    CLK: OutputPin<Error = ERR>,
    DIN: OutputPin<Error = ERR>,
    DC: OutputPin<Error = ERR>,
    CE: OutputPin<Error = ERR>,
{
    type Error = ERR;

    fn write_byte(&mut self, data: bool, mut value: u8) -> Result<(), ERR> {
        if data {
            self.dc.set_high()?;
        } else {
            self.dc.set_low()?;
        }
        self.ce.set_low()?;
        for _ in 0..8 {
            self.write_bit((value & 0x80) == 0x80)?;
            value <<= 1;
        }
        self.ce.set_high()
    }
}

pub enum SPIBackendError<PinErr, SpiErr> {
    Pin(PinErr),
    Spi(SpiErr),
}

pub struct PCD8544SpiBackend<SPI, DC, CE>
where
    SPI: blocking::spi::Write<u8>,
    DC: OutputPin,
    CE: OutputPin,
{
    spi: SPI,
    dc: DC,
    ce: CE,
}

impl<SPI, DC, CE, PinErr, SpiErr> PCD8544SpiBackend<SPI, DC, CE>
where
    SPI: blocking::spi::Write<u8, Error = SpiErr>,
    DC: OutputPin<Error = PinErr>,
    CE: OutputPin<Error = PinErr>,
{
    pub fn new(spi: SPI, dc: DC, mut ce: CE) -> Result<PCD8544SpiBackend<SPI, DC, CE>, PinErr> {
        ce.set_high()?;
        Ok(PCD8544SpiBackend { spi, dc, ce })
    }
}

impl<SPI, DC, CE, PinErr, SpiErr> PCD8544Backend for PCD8544SpiBackend<SPI, DC, CE>
where
    SPI: blocking::spi::Write<u8, Error = SpiErr>,
    DC: OutputPin<Error = PinErr>,
    CE: OutputPin<Error = PinErr>,
{
    type Error = SPIBackendError<PinErr, SpiErr>;

    fn write_byte(&mut self, data: bool, value: u8) -> Result<(), SPIBackendError<PinErr, SpiErr>> {
        if data {
            self.dc.set_high().map_err(SPIBackendError::Pin)?;
        } else {
            self.dc.set_low().map_err(SPIBackendError::Pin)?;
        }
        self.ce.set_low().map_err(SPIBackendError::Pin)?;
        self.spi.write(&[value]).map_err(SPIBackendError::Spi)?;
        self.ce.set_high().map_err(SPIBackendError::Pin)
    }
}
