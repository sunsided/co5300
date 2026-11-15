//! The driver body, shared by the blocking and the async driver.
//!
//! [`define_driver!`] expands once with plain functions on [`Interface`](crate::Interface)
//! and once with `async` functions on `AsyncInterface`, inserting `.await` after every bus
//! access and delay.

/// Defines [`reset`](crate::reset) and the driver struct with its methods.
///
/// `$iface` is the interface trait, `$delay` the delay trait, `$doc` the struct
/// documentation, `$($async)?` is empty or `async`, and `$($await)*` is empty or `.await`.
macro_rules! define_driver {
    ($iface:path, $delay:path, $doc:literal, [$($async:tt)?], [$($await:tt)*]) => {
        use $crate::{
            AclMode, Command, Config, CtrlDisplay, EnhancementLevel, Error, Madctl, PanelCommand,
            PixelFormat, TearingEffect, RESET_LOW_MS, RESET_WAIT_MS, SLEEP_OUT_TO_DISPLAY_ON_MS,
            SLEEP_WAIT_MS, SOFTWARE_RESET_WAIT_MS,
        };
        use co5300_registers::prelude::*;
        use co5300_registers::{
            AclControl, Brightness, Colmod, ColorEnhance, CommandPage, DeepStandby, HbmBrightness,
            HbmControl, Page, SpiMode, TearingEffectOn,
        };
        use embedded_hal::digital::OutputPin;

        /// Resets the chip through its active-low `RESX` pin: low for [`RESET_LOW_MS`], high,
        /// then a wait of [`RESET_WAIT_MS`].
        ///
        /// Afterwards the chip is in Sleep In mode with its register defaults (datasheet
        /// section 6.6). This also leaves deep standby.
        pub $($async)? fn reset<P, D>(rst: &mut P, delay: &mut D) -> Result<(), P::Error>
        where
            P: OutputPin,
            D: $delay,
        {
            rst.set_low()?;
            delay.delay_ms(RESET_LOW_MS) $($await)*;
            rst.set_high()?;
            delay.delay_ms(RESET_WAIT_MS) $($await)*;
            Ok(())
        }

        #[doc = $doc]
        #[derive(Debug)]
        pub struct Co5300<IFACE> {
            iface: IFACE,
            pub(crate) config: Config,
            pub(crate) format: PixelFormat,
        }

        impl<IFACE> Co5300<IFACE> {
            /// Creates a driver on the specified interface. Performs no bus access.
            pub fn new(iface: IFACE, config: Config) -> Self {
                Self {
                    iface,
                    config,
                    // The chip's reset default (COLMOD 77h).
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

        impl<IFACE: $iface> Co5300<IFACE> {
            /// Sends a raw command with its parameter bytes.
            pub $($async)? fn write_command(&mut self, command: u8, params: &[u8]) -> Result<(), Error<IFACE::Error>> {
                Ok(self.iface.write_command(command, params) $($await)* ?)
            }

            /// Writes a single-parameter command, given as its register type.
            pub $($async)? fn write_register<R: WritableRegister>(
                &mut self,
                reg: R,
            ) -> Result<(), Error<IFACE::Error>> {
                self.write_command(R::COMMAND, &[reg.to_bits()]) $($await)*
            }

            /// Initializes the chip after a reset.
            ///
            /// Sends, in this order: the `panel` commands; `CMD Page Switch` (FEh) back to the
            /// user command set; `SetSPIMode` (C4h) with `SPI_WRAM` if the interface asks for it
            /// ([`Interface::SPI_RAM_WRITE`](crate::Interface::SPI_RAM_WRITE)); `SLPOUT`; a wait
            /// of [`SLEEP_OUT_TO_DISPLAY_ON_MS`]; `COLMOD` and `WRDISBV` from the [`Config`];
            /// `DISPON`.
            ///
            /// Call [`reset`] (or [`software_reset`](Self::software_reset)) first: `SLPOUT` may
            /// not follow a reset within 120 ms.
            pub $($async)? fn init<D: $delay>(
                &mut self,
                delay: &mut D,
                panel: &[PanelCommand<'_>],
            ) -> Result<(), Error<IFACE::Error>> {
                for &(command, params) in panel {
                    self.write_command(command, params) $($await)* ?;
                }

                self.write_register(CommandPage::new().with_page(Page::User)) $($await)* ?;

                if IFACE::SPI_RAM_WRITE {
                    self.write_register(SpiMode::new().with_spi_wram(true)) $($await)* ?;
                }

                self.write_command(Command::SLPOUT.code(), &[]) $($await)* ?;
                delay.delay_ms(SLEEP_OUT_TO_DISPLAY_ON_MS) $($await)*;
                self.set_pixel_format(self.config.pixel_format) $($await)* ?;
                self.set_brightness(self.config.brightness) $($await)* ?;
                self.display_on() $($await)*
            }

            /// Sends `SWRESET` (01h) and waits [`SOFTWARE_RESET_WAIT_MS`]. The chip returns to
            /// its defaults in Sleep In mode.
            pub $($async)? fn software_reset<D: $delay>(&mut self, delay: &mut D) -> Result<(), Error<IFACE::Error>> {
                self.write_command(Command::SWRESET.code(), &[]) $($await)* ?;
                self.format = PixelFormat::Rgb888;
                delay.delay_ms(SOFTWARE_RESET_WAIT_MS) $($await)*;
                Ok(())
            }

            /// Enters Sleep In mode (`SLPIN`, 10h) and waits [`SLEEP_WAIT_MS`].
            ///
            /// The datasheet requires 120 ms between `SLPOUT` and `SLPIN`, and 83 ms (at 60 Hz)
            /// between `SLPIN` and pulling `RESX` low or removing power (section 5.6.3).
            pub $($async)? fn sleep_in<D: $delay>(&mut self, delay: &mut D) -> Result<(), Error<IFACE::Error>> {
                self.write_command(Command::SLPIN.code(), &[]) $($await)* ?;
                delay.delay_ms(SLEEP_WAIT_MS) $($await)*;
                Ok(())
            }

            /// Leaves Sleep In mode (`SLPOUT`, 11h) and waits [`SLEEP_OUT_TO_DISPLAY_ON_MS`],
            /// after which `DISPON` may follow.
            pub $($async)? fn sleep_out<D: $delay>(&mut self, delay: &mut D) -> Result<(), Error<IFACE::Error>> {
                self.write_command(Command::SLPOUT.code(), &[]) $($await)* ?;
                delay.delay_ms(SLEEP_OUT_TO_DISPLAY_ON_MS) $($await)*;
                Ok(())
            }

            /// Turns the display on (`DISPON`, 29h).
            pub $($async)? fn display_on(&mut self) -> Result<(), Error<IFACE::Error>> {
                self.write_command(Command::DISPON.code(), &[]) $($await)*
            }

            /// Turns the display off (`DISPOFF`, 28h). The frame memory is kept.
            pub $($async)? fn display_off(&mut self) -> Result<(), Error<IFACE::Error>> {
                self.write_command(Command::DISPOFF.code(), &[]) $($await)*
            }

            /// Sets the display brightness (`WRDISBV`, 51h): `0x00` is 0 %, `0xFF` 100 %.
            pub $($async)? fn set_brightness(&mut self, brightness: u8) -> Result<(), Error<IFACE::Error>> {
                self.write_register(Brightness::new().with_dbv(brightness)) $($await)*
            }

            /// Writes the brightness control (`WRCTRLD`, 53h).
            pub $($async)? fn set_ctrl_display(&mut self, ctrl: CtrlDisplay) -> Result<(), Error<IFACE::Error>> {
                self.write_register(ctrl) $($await)*
            }

            /// Sets the brightness used in HBM mode (`WRHBMDISBV`, 63h).
            pub $($async)? fn set_hbm_brightness(&mut self, brightness: u8) -> Result<(), Error<IFACE::Error>> {
                self.write_register(HbmBrightness::new().with_dbv(brightness)) $($await)*
            }

            /// Enters or leaves HBM (high brightness) mode (66h).
            pub $($async)? fn set_hbm(&mut self, enabled: bool) -> Result<(), Error<IFACE::Error>> {
                self.write_register(HbmControl::new().with_hbm_en(enabled)) $($await)*
            }

            /// Enables or disables the auto current limit (`WRACL`, 55h).
            pub $($async)? fn set_acl(&mut self, enabled: bool) -> Result<(), Error<IFACE::Error>> {
                let mode = if enabled { AclMode::On } else { AclMode::Off };
                self.write_register(AclControl::new().with_acl(mode)) $($await)*
            }

            /// Sets sunlight readable enhancement (58h); `None` disables it.
            pub $($async)? fn set_sunlight_enhancement(
                &mut self,
                level: Option<EnhancementLevel>,
            ) -> Result<(), Error<IFACE::Error>> {
                let reg = match level {
                    Some(level) => ColorEnhance::new().with_enable(true).with_level(level),
                    None => ColorEnhance::new(),
                };
                self.write_register(reg) $($await)*
            }

            /// Sets the pixel format (`COLMOD`, 3Ah), in both the `IFPF` and `VIPF` fields.
            pub $($async)? fn set_pixel_format(&mut self, format: PixelFormat) -> Result<(), Error<IFACE::Error>> {
                self.write_register(Colmod::uniform(format)) $($await)* ?;
                self.format = format;
                Ok(())
            }

            /// Sets the memory access direction and color order (`MADCTL`, 36h).
            pub $($async)? fn set_madctl(&mut self, madctl: Madctl) -> Result<(), Error<IFACE::Error>> {
                self.write_register(madctl) $($await)*
            }

            /// Turns display inversion on (`INVON`, 21h) or off (`INVOFF`, 20h).
            pub $($async)? fn set_inverted(&mut self, inverted: bool) -> Result<(), Error<IFACE::Error>> {
                let command = if inverted {
                    Command::INVON
                } else {
                    Command::INVOFF
                };
                self.write_command(command.code(), &[]) $($await)*
            }

            /// Turns the tearing effect output on (`TEON`, 35h) in the specified mode, or off
            /// (`TEOFF`, 34h) with `None`.
            pub $($async)? fn set_tearing_effect(
                &mut self,
                mode: Option<TearingEffect>,
            ) -> Result<(), Error<IFACE::Error>> {
                match mode {
                    Some(mode) => {
                        self.write_register(
                            TearingEffectOn::new().with_m(mode == TearingEffect::VAndHBlank),
                        ) $($await)*
                    }
                    None => self.write_command(Command::TEOFF.code(), &[]) $($await)*,
                }
            }

            /// Sets the scan line at which the tearing effect output pulses (`STESL`, 44h).
            /// `0` is equivalent to `TEON` in mode 1.
            pub $($async)? fn set_tear_scanline(&mut self, line: u16) -> Result<(), Error<IFACE::Error>> {
                self.write_command(Command::STESL.code(), &line.to_be_bytes()) $($await)*
            }

            /// Enters (`IDMON`, 39h) or leaves (`IDMOFF`, 38h) Idle mode, where colors are
            /// reduced to the MSB of each channel.
            pub $($async)? fn set_idle(&mut self, idle: bool) -> Result<(), Error<IFACE::Error>> {
                let command = if idle {
                    Command::IDMON
                } else {
                    Command::IDMOFF
                };
                self.write_command(command.code(), &[]) $($await)*
            }

            /// Enters deep standby (`DSTBON`, 4Fh). Only a hardware [`reset`] leaves it.
            pub $($async)? fn enter_deep_standby(&mut self) -> Result<(), Error<IFACE::Error>> {
                self.write_register(DeepStandby::new().with_dstb(true)) $($await)*
            }

            /// Sets the frame memory window (`CASET`, `RASET`) in display coordinates.
            ///
            /// `x` and `width`, and `y` and `height`, must be even after adding the
            /// [`Config`] offsets.
            pub $($async)? fn set_window(
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
                self.write_command(Command::CASET.code(), &columns.to_params()) $($await)* ?;
                self.write_command(Command::RASET.code(), &rows.to_params()) $($await)*
            }

            /// Starts a memory write (`RAMWR`, 2Ch) at the window origin.
            pub $($async)? fn write_pixels(&mut self, data: &[u8]) -> Result<(), Error<IFACE::Error>> {
                Ok(self.iface.write_pixels(Command::RAMWR.code(), data) $($await)* ?)
            }

            /// Continues a memory write (`RAMWRC`, 3Ch) after the last written pixel.
            pub $($async)? fn write_pixels_continue(&mut self, data: &[u8]) -> Result<(), Error<IFACE::Error>> {
                Ok(self.iface.write_pixels(Command::RAMWRC.code(), data) $($await)* ?)
            }
        }
    };
}

pub(crate) use define_driver;
