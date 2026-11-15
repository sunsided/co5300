//! The transport interface and the shipped adapters: 4-wire SPI with a D/C pin, and
//! single-line SPI with the datasheet's Quad-SPI framing.

use crate::qspi::{Frame, PixelMode};
use embedded_hal::digital::OutputPin;

/// A transport that carries CO5300 commands and pixel data.
///
/// The driver only ever needs two operations, both of which every CO5300 host interface
/// offers in some form:
///
/// - [`write_command`](Self::write_command): a command code followed by its parameter bytes;
/// - [`write_pixels`](Self::write_pixels): a memory write command, `RAMWR` (2Ch) or `RAMWRC`
///   (3Ch), followed by pixel bytes, possibly many kilobytes.
///
/// The split mirrors ESP-IDF's `esp_lcd_panel_io_tx_param` and `esp_lcd_panel_io_tx_color`,
/// so a MIPI-DSI or Quad-SPI panel IO handle maps onto it one to one. Each call is one
/// complete transfer; the driver streams large images as one `RAMWR` followed by `RAMWRC`
/// calls, which continue where the previous write stopped (datasheet section 7.5.33).
///
/// Shipped implementations: [`SpiInterface`] (4-wire SPI with a D/C pin) and
/// [`FramedSpiInterface`] (single-line SPI with the instruction/address framing of datasheet
/// section 5.2.2). For dual or quad data lines, build an adapter on the [`qspi`](crate::qspi)
/// framing helper and your HAL's half-duplex transfer.
pub trait Interface {
    /// The transport error type.
    type Error;

    /// Whether [`init`](crate::Co5300::init) sends `SetSPIMode` (C4h) with `SPI_WRAM` set,
    /// enabling SPI writes to the frame memory (datasheet section 7.5.57). `true` for SPI
    /// and Quad-SPI transports, `false` (the default) for MIPI-DSI.
    const SPI_RAM_WRITE: bool = false;

    /// Sends a command code followed by its parameter bytes (none for most commands).
    fn write_command(&mut self, command: u8, params: &[u8]) -> Result<(), Self::Error>;

    /// Sends a memory write command, `RAMWR` (2Ch) or `RAMWRC` (3Ch), followed by pixel data.
    fn write_pixels(&mut self, command: u8, data: &[u8]) -> Result<(), Self::Error>;
}

/// The async twin of [`Interface`].
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
// The trait is meant for single-threaded executors; no `Send` bound on the futures.
#[allow(async_fn_in_trait)]
pub trait AsyncInterface {
    /// The transport error type.
    type Error;

    /// See [`Interface::SPI_RAM_WRITE`].
    const SPI_RAM_WRITE: bool = false;

    /// Sends a command code followed by its parameter bytes (none for most commands).
    async fn write_command(&mut self, command: u8, params: &[u8]) -> Result<(), Self::Error>;

    /// Sends a memory write command, `RAMWR` (2Ch) or `RAMWRC` (3Ch), followed by pixel data.
    async fn write_pixels(&mut self, command: u8, data: &[u8]) -> Result<(), Self::Error>;
}

/// An error of [`SpiInterface`]: either the SPI device or the D/C pin failed.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SpiError<S, P> {
    /// The SPI device failed.
    Spi(S),
    /// The D/C pin failed.
    Dc(P),
}

/// The CO5300 on a 4-wire SPI bus: `CSX`, `SCL`, `SDI` and the `D/CX` pin.
///
/// The datasheet names the `D/CX` input "display data / command selection" (low: command,
/// high: parameter or display data; section 4.2) and gives its setup and hold times
/// (section 6.4), but draws no waveform for this mode. The byte layout follows Espressif's
/// `esp_lcd_co5300` SPI configuration: one 8-bit command byte with `D/CX` low, then the
/// parameter or pixel bytes with `D/CX` high, MSB first.
///
/// The command byte and its data are two `SpiDevice` transactions, so `CSX` goes high in
/// between. The chip samples `SDI` on the rising edge of `SCL` (section 5.2); configure the
/// bus for SPI mode 0 or 3. Writes are limited to 50 MHz (`TSCYC` ≥ 20 ns, section 6.4).
#[derive(Debug)]
pub struct SpiInterface<SPI, DC> {
    spi: SPI,
    dc: DC,
}

impl<SPI, DC> SpiInterface<SPI, DC> {
    /// Creates an interface on the specified SPI device (bus plus chip select) and D/C pin.
    pub fn new(spi: SPI, dc: DC) -> Self {
        Self { spi, dc }
    }

    /// Returns the SPI device and the D/C pin.
    pub fn release(self) -> (SPI, DC) {
        (self.spi, self.dc)
    }
}

impl<SPI, DC> Interface for SpiInterface<SPI, DC>
where
    SPI: embedded_hal::spi::SpiDevice,
    DC: OutputPin,
{
    type Error = SpiError<SPI::Error, DC::Error>;
    const SPI_RAM_WRITE: bool = true;

    fn write_command(&mut self, command: u8, params: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(SpiError::Dc)?;
        self.spi.write(&[command]).map_err(SpiError::Spi)?;

        if !params.is_empty() {
            self.dc.set_high().map_err(SpiError::Dc)?;
            self.spi.write(params).map_err(SpiError::Spi)?;
        }

        Ok(())
    }

    fn write_pixels(&mut self, command: u8, data: &[u8]) -> Result<(), Self::Error> {
        self.write_command(command, data)
    }
}

#[cfg(feature = "async")]
impl<SPI, DC> AsyncInterface for SpiInterface<SPI, DC>
where
    SPI: embedded_hal_async::spi::SpiDevice,
    DC: OutputPin,
{
    type Error = SpiError<SPI::Error, DC::Error>;
    const SPI_RAM_WRITE: bool = true;

    async fn write_command(&mut self, command: u8, params: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(SpiError::Dc)?;
        self.spi.write(&[command]).await.map_err(SpiError::Spi)?;

        if !params.is_empty() {
            self.dc.set_high().map_err(SpiError::Dc)?;
            self.spi.write(params).await.map_err(SpiError::Spi)?;
        }

        Ok(())
    }

    async fn write_pixels(&mut self, command: u8, data: &[u8]) -> Result<(), Self::Error> {
        AsyncInterface::write_command(self, command, data).await
    }
}

/// The CO5300 on a single-line SPI bus (`CSX`, `SCL`, `SDI`), using the Quad-SPI
/// interface's Standard SPI framing (datasheet section 5.2.2, "QSPI 1-wire write").
///
/// Every transfer is one chip-select cycle: instruction `02h`, the 24-bit address
/// `{00h, CMD, 00h}`, then the parameter or pixel bytes. No D/C pin is needed. This is the
/// interface's mode after reset; pixel writes on two or four lines need a HAL with
/// half-duplex multi-line transfers, see [`qspi`](crate::qspi).
#[derive(Debug)]
pub struct FramedSpiInterface<SPI> {
    spi: SPI,
}

impl<SPI> FramedSpiInterface<SPI> {
    /// Creates an interface on the specified SPI device (bus plus chip select).
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    /// Returns the SPI device.
    pub fn release(self) -> SPI {
        self.spi
    }
}

impl<SPI> Interface for FramedSpiInterface<SPI>
where
    SPI: embedded_hal::spi::SpiDevice,
{
    type Error = SPI::Error;
    const SPI_RAM_WRITE: bool = true;

    fn write_command(&mut self, command: u8, params: &[u8]) -> Result<(), Self::Error> {
        use embedded_hal::spi::Operation;
        let header = Frame::command(command).header();
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Write(params)])
    }

    fn write_pixels(&mut self, command: u8, data: &[u8]) -> Result<(), Self::Error> {
        use embedded_hal::spi::Operation;
        let header = Frame::pixels(command, PixelMode::Single).header();
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Write(data)])
    }
}

#[cfg(feature = "async")]
impl<SPI> AsyncInterface for FramedSpiInterface<SPI>
where
    SPI: embedded_hal_async::spi::SpiDevice,
{
    type Error = SPI::Error;
    const SPI_RAM_WRITE: bool = true;

    async fn write_command(&mut self, command: u8, params: &[u8]) -> Result<(), Self::Error> {
        use embedded_hal_async::spi::Operation;
        let header = Frame::command(command).header();
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Write(params)])
            .await
    }

    async fn write_pixels(&mut self, command: u8, data: &[u8]) -> Result<(), Self::Error> {
        use embedded_hal_async::spi::Operation;
        let header = Frame::pixels(command, PixelMode::Single).header();
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Write(data)])
            .await
    }
}
