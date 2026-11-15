//! # Chipone CO5300 AMOLED Display Controller Driver
//!
//! A `no_std` driver for the CO5300, following the CO5300 datasheet V0.01 (Chipone, 24 July
//! 2023). The driver speaks commands; how they reach the chip is up to an [`Interface`]
//! implementation, so the same driver runs over:
//!
//! - 4-wire SPI with a D/C pin: [`SpiInterface`], on `embedded-hal` [`SpiDevice`] and
//!   [`OutputPin`];
//! - single-line SPI with the datasheet's instruction/address framing:
//!   [`FramedSpiInterface`];
//! - Quad-SPI, through your HAL's half-duplex transfer and the [`qspi`] framing helper;
//! - a MIPI-DSI panel IO, such as ESP-IDF's `esp_lcd_panel_io_tx_param`/`tx_color`
//!   (see [below](#mipi-dsi-esp-idf-panel-io)).
//!
//! Enable the `async` feature for an async driver ([`asynch::Co5300`]) on [`AsyncInterface`],
//! and the `embedded-graphics` feature for a [`DrawTarget`](embedded_graphics_core::draw_target::DrawTarget)
//! that writes straight to the chip's frame memory, without a frame buffer in the host.
//!
//! ```
//! # use core::cell::RefCell;
//! # use co5300_sim::{Co5300Model, FakeDc, FakeDelay, FakeSpi};
//! use co5300::{Co5300, Config, SpiInterface};
//!
//! # let model = RefCell::new(Co5300Model::new(410, 502));
//! # let (spi, dc, mut delay) = (FakeSpi::new(&model), FakeDc::new(&model), FakeDelay::new(&model));
//! let iface = SpiInterface::new(spi, dc);
//! let mut display = Co5300::new(iface, Config::new(410, 502));
//!
//! // After a hardware reset (`co5300::reset`): sleep out, RGB565, full brightness, display on.
//! display.init(&mut delay, &[])?;
//!
//! // Fill the top-left 4 x 2 pixels with red (RGB565, MSB first).
//! display.set_window(0, 0, 4, 2)?;
//! display.write_pixels(&[0xF8, 0x00].repeat(8))?;
//! # assert_eq!(model.borrow().pixel(3, 1), 0xFF0000);
//! # Ok::<(), co5300::Error<co5300::SpiError<embedded_hal::spi::ErrorKind, core::convert::Infallible>>>(())
//! ```
//!
//! ## Panel initialization
//!
//! The CO5300 drives many different AMOLED panels; their power, gamma and timing settings
//! live in the manufacturer command pages and come from the panel vendor, not from the chip
//! datasheet. [`Co5300::init`] therefore only sends the chip-level commands the datasheet
//! requires and accepts a panel-specific command list, sent first:
//!
//! ```rust,ignore
//! const PANEL: &[PanelCommand] = &[
//!     (0xFE, &[0x20]), // vendor page, unlock, settings ...
//!     (0xFE, &[0x00]),
//! ];
//! display.init(&mut delay, PANEL)?;
//! ```
//!
//! [`panel::ESPRESSIF_DEFAULT`] holds the default sequence of Espressif's `esp_lcd_co5300`
//! component.
//!
//! ## Window alignment
//!
//! `CASET` and `RASET` require an even start and an even length (datasheet sections 7.5.21
//! and 7.5.22). [`Co5300::set_window`] rejects other windows with [`Error::InvalidWindow`];
//! the `embedded-graphics` target widens them, see [its documentation](crate::graphics).
//!
//! ## MIPI-DSI: ESP-IDF panel IO
//!
//! On a board where the CO5300 hangs off MIPI-DSI (such as the Kode Dot below), the ESP-IDF
//! DSI bus stays in charge; an adapter in the board crate forwards the two calls:
//!
//! ```rust,ignore
//! struct PanelIo(esp_lcd_panel_io_handle_t);
//!
//! impl co5300::Interface for PanelIo {
//!     type Error = EspError;
//!     // MIPI-DSI: no SetSPIMode (C4h) in `init`.
//!     const SPI_RAM_WRITE: bool = false;
//!
//!     fn write_command(&mut self, command: u8, params: &[u8]) -> Result<(), EspError> {
//!         esp!(unsafe {
//!             esp_lcd_panel_io_tx_param(self.0, command.into(), params.as_ptr().cast(), params.len())
//!         })
//!     }
//!
//!     fn write_pixels(&mut self, command: u8, data: &[u8]) -> Result<(), EspError> {
//!         esp!(unsafe {
//!             esp_lcd_panel_io_tx_color(self.0, command.into(), data.as_ptr().cast(), data.len())
//!         })
//!     }
//! }
//! ```
//!
//! When the pixels come from a DPI frame buffer (video mode), only the command path is used.
//!
//! ## Board: Kode Dot
//!
//! The Kode Dot (ESP32-P4) drives the CO5300 over MIPI-DSI; ESP-IDF owns the DSI bus, the
//! DPI interface and the frame buffer. The panel is 410 × 502 pixels (`KODE_DOT` in the
//! handheld screen constants), so use `Config::new(410, 502)`. Before `init`:
//!
//! 1. The panel supply `VCI_EN` comes from TCA6408A pin P7: latch P7 high, then make it an
//!    output, then wait 20 ms.
//! 2. Reset is shared with the touch controller on GPIO7: low for 10 ms, high, then wait
//!    120 ms ([`reset`] does exactly this).
//! 3. The DSI PHY LDO runs at 2.5 V.
//!
//! The datasheet lists resolutions up to 480 lines (and 192 × 960); 502 lines is the
//! board's value, not one of the datasheet's.
//!
//! [`SpiDevice`]: embedded_hal::spi::SpiDevice
//! [`OutputPin`]: embedded_hal::digital::OutputPin

#![deny(missing_docs)]
#![no_std]
#![forbid(unsafe_code)]
// Enables the `doc_cfg` feature when the `docsrs` configuration attribute is defined.
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub mod asynch;
mod driver;
#[cfg(feature = "embedded-graphics")]
#[cfg_attr(docsrs, doc(cfg(feature = "embedded-graphics")))]
pub mod graphics;
mod interface;
pub mod panel;
pub mod qspi;

#[cfg(feature = "async")]
pub use interface::AsyncInterface;
pub use interface::{FramedSpiInterface, Interface, SpiError, SpiInterface};

pub use blocking::{Co5300, reset};
pub use co5300_registers as registers;
pub use co5300_registers::{
    AclMode, AddressRange, Command, CtrlDisplay, EnhancementLevel, Madctl, PixelFormat,
};

/// A panel-specific command: the command code and its parameter bytes.
pub type PanelCommand<'a> = (u8, &'a [u8]);

/// How long [`reset`] holds `RESX` low, in milliseconds.
///
/// The datasheet requires at least 10 µs (`tRESW`, section 6.6) and more than 3 ms to leave
/// deep standby (section 7.5.37); 10 ms covers both.
pub const RESET_LOW_MS: u32 = 10;

/// How long [`reset`] waits after releasing `RESX`, in milliseconds.
///
/// The reset completes within 120 ms when applied in Sleep Out mode (`tRESET`), and
/// `SLPOUT` may not be sent earlier (section 6.6, note 5).
pub const RESET_WAIT_MS: u32 = 120;

/// The wait after `SLPIN` or `SLPOUT` before the next command, in milliseconds
/// (sections 7.5.11 and 7.5.12).
pub const SLEEP_WAIT_MS: u32 = 5;

/// The wait between `SLPOUT` and `DISPON`, in milliseconds (`T4`, section 5.6.2).
pub const SLEEP_OUT_TO_DISPLAY_ON_MS: u32 = 60;

/// The wait after `SWRESET` before `SLPOUT` when reset in Sleep Out mode, in milliseconds
/// (section 7.5.2). Other commands may follow after 5 ms.
pub const SOFTWARE_RESET_WAIT_MS: u32 = 120;

/// A driver error.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    /// The interface failed.
    Interface(E),
    /// The window is empty, outside the display, or not 2-pixel aligned (`CASET`/`RASET`
    /// need an even start and length).
    InvalidWindow,
    /// The operation needs a different pixel format, such as RGB565 for drawing.
    UnsupportedFormat,
}

impl<E> From<E> for Error<E> {
    fn from(value: E) -> Self {
        Self::Interface(value)
    }
}

/// The tearing effect output mode (`TEON`, 35h).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TearingEffect {
    /// Mode 1: V-blanking only.
    VBlank,
    /// Mode 2: V-blanking and H-blanking.
    VAndHBlank,
}

/// Display geometry and the settings [`Co5300::init`] applies.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// The visible width in pixels.
    pub width: u16,
    /// The visible height in pixels.
    pub height: u16,
    /// The frame memory column of the first visible column.
    pub column_offset: u16,
    /// The frame memory row of the first visible row.
    pub row_offset: u16,
    /// The pixel format set by `init`.
    pub pixel_format: PixelFormat,
    /// The brightness set by `init`, `0x00` (0 %) to `0xFF` (100 %). The chip's reset
    /// default is `0x00`, a dark panel.
    pub brightness: u8,
}

impl Config {
    /// A `width` × `height` panel at frame memory origin, RGB565, full brightness.
    pub const fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            column_offset: 0,
            row_offset: 0,
            pixel_format: PixelFormat::Rgb565,
            brightness: 0xFF,
        }
    }

    /// Sets the frame memory position of the visible area.
    pub const fn with_offset(mut self, column: u16, row: u16) -> Self {
        self.column_offset = column;
        self.row_offset = row;
        self
    }

    /// Sets the pixel format `init` selects.
    pub const fn with_pixel_format(mut self, format: PixelFormat) -> Self {
        self.pixel_format = format;
        self
    }

    /// Sets the brightness `init` selects.
    pub const fn with_brightness(mut self, brightness: u8) -> Self {
        self.brightness = brightness;
        self
    }

    /// Converts a window in display coordinates into frame memory ranges, checking bounds
    /// and the 2-pixel alignment.
    pub const fn window(
        &self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> Option<(AddressRange, AddressRange)> {
        if width == 0
            || height == 0
            || x as u32 + width as u32 > self.width as u32
            || y as u32 + height as u32 > self.height as u32
        {
            return None;
        }

        let x = x + self.column_offset;
        let y = y + self.row_offset;
        let columns = AddressRange::new(x, x + width - 1);
        let rows = AddressRange::new(y, y + height - 1);

        if columns.is_valid() && rows.is_valid() {
            Some((columns, rows))
        } else {
            None
        }
    }
}

/// The blocking driver.
mod blocking {
    crate::driver::define_driver!(
        crate::Interface,
        embedded_hal::delay::DelayNs,
        "Blocking CO5300 driver.",
        [],
        []
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_checks_bounds_and_alignment() {
        let config = Config::new(410, 502);
        assert_eq!(
            config.window(0, 0, 410, 502),
            Some((AddressRange::new(0, 409), AddressRange::new(0, 501)))
        );
        assert_eq!(config.window(1, 0, 2, 2), None, "odd start");
        assert_eq!(config.window(0, 0, 3, 2), None, "odd width");
        assert_eq!(config.window(0, 0, 0, 2), None, "empty");
        assert_eq!(config.window(408, 0, 4, 2), None, "out of bounds");

        let offset = config.with_offset(6, 0);
        assert_eq!(
            offset.window(2, 4, 2, 2),
            Some((AddressRange::new(8, 9), AddressRange::new(4, 5)))
        );
    }
}
