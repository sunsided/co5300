//! Async driver on [`AsyncInterface`].
//!
//! Mirrors the blocking [`Co5300`](crate::Co5300); see there for the datasheet references.

use crate::interface::AsyncInterface;
use crate::{
    AclMode, Command, Config, CtrlDisplay, EnhancementLevel, Error, Madctl, PanelCommand,
    PixelFormat, SLEEP_OUT_TO_DISPLAY_ON_MS, SLEEP_WAIT_MS, SOFTWARE_RESET_WAIT_MS, TearingEffect,
};
use co5300_registers::prelude::*;
use co5300_registers::{
    AclControl, Brightness, Colmod, ColorEnhance, CommandPage, DeepStandby, HbmBrightness,
    HbmControl, Page, SpiMode, TearingEffectOn,
};
use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;

/// Async version of [`reset`](crate::reset).
pub async fn reset<P, D>(rst: &mut P, delay: &mut D) -> Result<(), P::Error>
where
    P: OutputPin,
    D: DelayNs,
{
    rst.set_low()?;
    delay.delay_ms(crate::RESET_LOW_MS).await;
    rst.set_high()?;
    delay.delay_ms(crate::RESET_WAIT_MS).await;
    Ok(())
}

/// Async CO5300 driver.
#[derive(Debug)]
pub struct Co5300<IFACE> {
    iface: IFACE,
    config: Config,
    format: PixelFormat,
}

impl<IFACE> Co5300<IFACE> {
    /// Creates a driver on the specified interface. Performs no bus access.
    pub fn new(iface: IFACE, config: Config) -> Self {
        Self {
            iface,
            config,
            format: PixelFormat::Rgb888,
        }
    }

    /// Consumes the driver and returns the interface.
    pub fn release(self) -> IFACE {
        self.iface
    }

    /// The interface, for commands this driver does not cover.
    pub fn interface(&mut self) -> &mut IFACE {
        &mut self.iface
    }

    /// The configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The pixel format last set through this driver.
    pub fn pixel_format(&self) -> PixelFormat {
        self.format
    }
}

impl<IFACE: AsyncInterface> Co5300<IFACE> {
    /// See [`crate::Co5300::write_command`].
    pub async fn write_command(
        &mut self,
        command: u8,
        params: &[u8],
    ) -> Result<(), Error<IFACE::Error>> {
        Ok(self.iface.write_command(command, params).await?)
    }

    /// See [`crate::Co5300::write_register`].
    pub async fn write_register<R: WritableRegister>(
        &mut self,
        reg: R,
    ) -> Result<(), Error<IFACE::Error>> {
        self.write_command(R::COMMAND, &[reg.to_bits()]).await
    }

    /// See [`crate::Co5300::init`].
    pub async fn init<D: DelayNs>(
        &mut self,
        delay: &mut D,
        panel: &[PanelCommand<'_>],
    ) -> Result<(), Error<IFACE::Error>> {
        for &(command, params) in panel {
            self.write_command(command, params).await?;
        }

        self.write_register(CommandPage::new().with_page(Page::User))
            .await?;

        if IFACE::SPI_RAM_WRITE {
            self.write_register(SpiMode::new().with_spi_wram(true))
                .await?;
        }

        self.write_command(Command::SLPOUT.code(), &[]).await?;
        delay.delay_ms(SLEEP_OUT_TO_DISPLAY_ON_MS).await;
        self.set_pixel_format(self.config.pixel_format).await?;
        self.set_brightness(self.config.brightness).await?;
        self.display_on().await
    }

    /// See [`crate::Co5300::software_reset`].
    pub async fn software_reset<D: DelayNs>(
        &mut self,
        delay: &mut D,
    ) -> Result<(), Error<IFACE::Error>> {
        self.write_command(Command::SWRESET.code(), &[]).await?;
        self.format = PixelFormat::Rgb888;
        delay.delay_ms(SOFTWARE_RESET_WAIT_MS).await;
        Ok(())
    }

    /// See [`crate::Co5300::sleep_in`].
    pub async fn sleep_in<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), Error<IFACE::Error>> {
        self.write_command(Command::SLPIN.code(), &[]).await?;
        delay.delay_ms(SLEEP_WAIT_MS).await;
        Ok(())
    }

    /// See [`crate::Co5300::sleep_out`].
    pub async fn sleep_out<D: DelayNs>(
        &mut self,
        delay: &mut D,
    ) -> Result<(), Error<IFACE::Error>> {
        self.write_command(Command::SLPOUT.code(), &[]).await?;
        delay.delay_ms(SLEEP_OUT_TO_DISPLAY_ON_MS).await;
        Ok(())
    }

    /// See [`crate::Co5300::display_on`].
    pub async fn display_on(&mut self) -> Result<(), Error<IFACE::Error>> {
        self.write_command(Command::DISPON.code(), &[]).await
    }

    /// See [`crate::Co5300::display_off`].
    pub async fn display_off(&mut self) -> Result<(), Error<IFACE::Error>> {
        self.write_command(Command::DISPOFF.code(), &[]).await
    }

    /// See [`crate::Co5300::set_brightness`].
    pub async fn set_brightness(&mut self, brightness: u8) -> Result<(), Error<IFACE::Error>> {
        self.write_register(Brightness::new().with_dbv(brightness))
            .await
    }

    /// See [`crate::Co5300::set_ctrl_display`].
    pub async fn set_ctrl_display(&mut self, ctrl: CtrlDisplay) -> Result<(), Error<IFACE::Error>> {
        self.write_register(ctrl).await
    }

    /// See [`crate::Co5300::set_hbm_brightness`].
    pub async fn set_hbm_brightness(&mut self, brightness: u8) -> Result<(), Error<IFACE::Error>> {
        self.write_register(HbmBrightness::new().with_dbv(brightness))
            .await
    }

    /// See [`crate::Co5300::set_hbm`].
    pub async fn set_hbm(&mut self, enabled: bool) -> Result<(), Error<IFACE::Error>> {
        self.write_register(HbmControl::new().with_hbm_en(enabled))
            .await
    }

    /// See [`crate::Co5300::set_acl`].
    pub async fn set_acl(&mut self, enabled: bool) -> Result<(), Error<IFACE::Error>> {
        let mode = if enabled { AclMode::On } else { AclMode::Off };
        self.write_register(AclControl::new().with_acl(mode)).await
    }

    /// See [`crate::Co5300::set_sunlight_enhancement`].
    pub async fn set_sunlight_enhancement(
        &mut self,
        level: Option<EnhancementLevel>,
    ) -> Result<(), Error<IFACE::Error>> {
        let reg = match level {
            Some(level) => ColorEnhance::new().with_enable(true).with_level(level),
            None => ColorEnhance::new(),
        };
        self.write_register(reg).await
    }

    /// See [`crate::Co5300::set_pixel_format`].
    pub async fn set_pixel_format(
        &mut self,
        format: PixelFormat,
    ) -> Result<(), Error<IFACE::Error>> {
        self.write_register(Colmod::uniform(format)).await?;
        self.format = format;
        Ok(())
    }

    /// See [`crate::Co5300::set_madctl`].
    pub async fn set_madctl(&mut self, madctl: Madctl) -> Result<(), Error<IFACE::Error>> {
        self.write_register(madctl).await
    }

    /// See [`crate::Co5300::set_inverted`].
    pub async fn set_inverted(&mut self, inverted: bool) -> Result<(), Error<IFACE::Error>> {
        let command = if inverted {
            Command::INVON
        } else {
            Command::INVOFF
        };
        self.write_command(command.code(), &[]).await
    }

    /// See [`crate::Co5300::set_tearing_effect`].
    pub async fn set_tearing_effect(
        &mut self,
        mode: Option<TearingEffect>,
    ) -> Result<(), Error<IFACE::Error>> {
        match mode {
            Some(mode) => {
                self.write_register(
                    TearingEffectOn::new().with_m(mode == TearingEffect::VAndHBlank),
                )
                .await
            }
            None => self.write_command(Command::TEOFF.code(), &[]).await,
        }
    }

    /// See [`crate::Co5300::set_tear_scanline`].
    pub async fn set_tear_scanline(&mut self, line: u16) -> Result<(), Error<IFACE::Error>> {
        self.write_command(Command::STESL.code(), &line.to_be_bytes())
            .await
    }

    /// See [`crate::Co5300::set_idle`].
    pub async fn set_idle(&mut self, idle: bool) -> Result<(), Error<IFACE::Error>> {
        let command = if idle {
            Command::IDMON
        } else {
            Command::IDMOFF
        };
        self.write_command(command.code(), &[]).await
    }

    /// See [`crate::Co5300::enter_deep_standby`].
    pub async fn enter_deep_standby(&mut self) -> Result<(), Error<IFACE::Error>> {
        self.write_register(DeepStandby::new().with_dstb(true))
            .await
    }

    /// See [`crate::Co5300::set_window`].
    pub async fn set_window(
        &mut self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> Result<(), Error<IFACE::Error>> {
        let (columns, rows) = self
            .config
            .window(x, y, width, height)
            .ok_or(Error::InvalidWindow)?;
        self.write_command(Command::CASET.code(), &columns.to_params())
            .await?;
        self.write_command(Command::RASET.code(), &rows.to_params())
            .await
    }

    /// See [`crate::Co5300::write_pixels`].
    pub async fn write_pixels(&mut self, data: &[u8]) -> Result<(), Error<IFACE::Error>> {
        Ok(self.iface.write_pixels(Command::RAMWR.code(), data).await?)
    }

    /// See [`crate::Co5300::write_pixels_continue`].
    pub async fn write_pixels_continue(&mut self, data: &[u8]) -> Result<(), Error<IFACE::Error>> {
        Ok(self
            .iface
            .write_pixels(Command::RAMWRC.code(), data)
            .await?)
    }
}
