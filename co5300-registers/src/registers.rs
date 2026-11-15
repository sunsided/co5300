//! Commands with a single parameter byte, modelled as registers.
//!
//! Each register's address is its write command; its value is the parameter byte. Bit names
//! and defaults follow the datasheet's command descriptions (section 7.5) and command list
//! (section 7.4). Bits the datasheet marks as "don't care" or `0` are padding and stay `0`.

use crate::Command;
use crate::types::{AclMode, EnhancementLevel, Page, PixelFormat};
use bitfield_struct::bitfield;

/// [`TEON`](Command::TEON) (35h), datasheet section 7.5.28.
///
/// Turns on the tearing effect output on the `TE` pin. Default `0x00`.
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TearingEffectOn {
    #[bits(7)]
    __: u8,

    /// `M`: `false` outputs V-blanking only (mode 1); `true` outputs V- and H-blanking
    /// (mode 2).
    #[bits(1, access = RW)]
    pub m: bool,
}

writable_register!(TearingEffectOn, Command::TEON);

/// [`MADCTL`](Command::MADCTL) (36h), datasheet section 7.5.29.
///
/// The direction in which the frame memory is written and the color order. Default `0x00`.
/// Read back with [`RDDMADCTR`](Command::RDDMADCTR) (0Bh).
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Madctl {
    /// `MY`: rows are written in decreasing order.
    #[bits(1, access = RW)]
    pub my: bool,

    /// `MX`: columns are written in decreasing order.
    #[bits(1, access = RW)]
    pub mx: bool,

    #[bits(2)]
    __: u8,

    /// `RGB`: `false` RGB, `true` BGR order.
    #[bits(1, access = RW)]
    pub bgr: bool,

    #[bits(3)]
    __: u8,
}

writable_register!(Madctl, Command::MADCTL);

/// [`COLMOD`](Command::COLMOD) (3Ah), datasheet section 7.5.32.
///
/// The pixel format of the image data. Default `0x77` (24 bits per pixel).
/// Read back with [`RDDCOLMOD`](Command::RDDCOLMOD) (0Ch).
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Colmod {
    /// `SPI_IFPF_SEL`: `false` the SPI interface uses [`ifpf`](Self::ifpf), `true` it uses
    /// [`vipf`](Self::vipf).
    #[bits(1, access = RW)]
    pub spi_ifpf_sel: bool,

    /// `VIPF[2:0]`: the alternative pixel format, selected by
    /// [`spi_ifpf_sel`](Self::spi_ifpf_sel).
    #[bits(3, access = RW, default = PixelFormat::Rgb888)]
    pub vipf: PixelFormat,

    #[bits(1)]
    __: u8,

    /// `IFPF[2:0]`: the control interface pixel format.
    #[bits(3, access = RW, default = PixelFormat::Rgb888)]
    pub ifpf: PixelFormat,
}

writable_register!(Colmod, Command::COLMOD);

impl Colmod {
    /// Selects the same format in both `VIPF` and `IFPF`, as `0x55` for RGB565, so the format
    /// applies whichever field the interface uses.
    pub const fn uniform(format: PixelFormat) -> Self {
        Self::new().with_vipf(format).with_ifpf(format)
    }

    /// The format the SPI interface uses, as selected by `SPI_IFPF_SEL`.
    pub const fn spi_format(&self) -> PixelFormat {
        if self.spi_ifpf_sel() {
            self.vipf()
        } else {
            self.ifpf()
        }
    }
}

/// [`DSTBON`](Command::DSTBON) (4Fh), datasheet section 7.5.37.
///
/// Leaving deep standby needs a low pulse of more than 3 ms on `RESX`.
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DeepStandby {
    #[bits(7)]
    __: u8,

    /// `DSTB`: enter deep standby.
    #[bits(1, access = RW)]
    pub dstb: bool,
}

writable_register!(DeepStandby, Command::DSTBON);

/// [`WRDISBV`](Command::WRDISBV) (51h), datasheet section 7.5.38.
///
/// Display brightness: `0x00` is 0 %, `0xFF` is 100 %. Default `0x00`.
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Brightness {
    /// `DBV[7:0]`.
    #[bits(8, access = RW)]
    pub dbv: u8,
}

writable_register!(Brightness, Command::WRDISBV);

/// [`WRCTRLD`](Command::WRCTRLD) (53h), datasheet section 7.5.40.
///
/// Brightness control. Default `0x28` (`BCTRL` and `DD` set).
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CtrlDisplay {
    #[bits(2)]
    __: u8,

    /// `BCTRL` (`BC_EN`): the brightness value `DBV` is used.
    #[bits(1, access = RW, default = true)]
    pub bctrl: bool,

    #[bits(1)]
    __: u8,

    /// `DD` (`DIM_EN`): dimming is applied when `BCTRL` changes.
    #[bits(1, access = RW, default = true)]
    pub dd: bool,

    #[bits(3)]
    __: u8,
}

writable_register!(CtrlDisplay, Command::WRCTRLD);

/// [`WRACL`](Command::WRACL) (55h), datasheet section 7.5.42.
///
/// Auto current limit. Default `0x00` (off).
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AclControl {
    #[bits(6)]
    __: u8,

    /// `ACL[1:0]`.
    #[bits(2, access = RW, default = AclMode::Off)]
    pub acl: AclMode,
}

writable_register!(AclControl, Command::WRACL);

/// [`WRIMGEHCCTR`](Command::WRIMGEHCCTR) (58h), datasheet section 7.5.44.
///
/// Sunlight readable enhancement (SRE). Default `0x00` (off).
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ColorEnhance {
    #[bits(5)]
    __: u8,

    /// `SLR_EN`: enables sunlight readable enhancement.
    #[bits(1, access = RW)]
    pub enable: bool,

    /// `SLR_LEVEL[1:0]`.
    #[bits(2, access = RW, default = EnhancementLevel::Low)]
    pub level: EnhancementLevel,
}

writable_register!(ColorEnhance, Command::WRIMGEHCCTR);

/// [`WRHBMDISBV`](Command::WRHBMDISBV) (63h), datasheet section 7.5.46.
///
/// Brightness in HBM (high brightness) mode: `0x00` is 0 %, `0xFF` is 100 %. Default `0x00`.
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct HbmBrightness {
    /// `DBV_HBM[7:0]`.
    #[bits(8, access = RW)]
    pub dbv: u8,
}

writable_register!(HbmBrightness, Command::WRHBMDISBV);

/// [`HBMCTL`](Command::HBMCTL) (66h), datasheet section 7.5.49.
///
/// Enters or leaves HBM mode. Default `0x00`.
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct HbmControl {
    #[bits(6)]
    __: u8,

    /// `HBM_EN`: HBM mode enabled.
    #[bits(1, access = RW)]
    pub hbm_en: bool,

    #[bits(1)]
    __: u8,
}

writable_register!(HbmControl, Command::HBMCTL);

/// [`SETSPIMODE`](Command::SETSPIMODE) (C4h), datasheet section 7.5.57.
///
/// Default `0x00`.
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SpiMode {
    /// `SPI_WRAM`: SPI write to SRAM enabled.
    #[bits(1, access = RW)]
    pub spi_wram: bool,

    #[bits(7)]
    __: u8,
}

writable_register!(SpiMode, Command::SETSPIMODE);

/// [`PAGESEL`](Command::PAGESEL) (FEh), datasheet section 7.5.61.
///
/// Selects the command page. Default `0x00` (`CMD1`). Read back with
/// [`PAGESTATUS`](Command::PAGESTATUS) (FFh).
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CommandPage {
    /// `CMD_Page_Selection[3:0]`.
    #[bits(4, access = RW, default = Page::User)]
    pub page: Page,

    #[bits(4)]
    __: u8,
}

writable_register!(CommandPage, Command::PAGESEL);

/// [`RDDPM`](Command::RDDPM) (0Ah), datasheet sections 7.4 and 7.5.5. Read-only.
///
/// The display power mode. Default `0x08` (`NORON`).
#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DisplayPowerMode {
    /// `BSTON`: booster on.
    #[bits(1, access = RO)]
    pub bston: bool,

    /// `IDMON`: Idle mode on.
    #[bits(1, access = RO)]
    pub idmon: bool,

    /// `PTLON`: Partial mode on.
    #[bits(1, access = RO)]
    pub ptlon: bool,

    /// `SLPOUT`: Sleep Out mode.
    #[bits(1, access = RO)]
    pub slpout: bool,

    /// `NORON`: Normal Display mode on.
    #[bits(1, access = RO, default = true)]
    pub noron: bool,

    /// `DISPON`: display on.
    #[bits(1, access = RO)]
    pub dispon: bool,

    #[bits(2)]
    __: u8,
}

readable_register!(DisplayPowerMode, Command::RDDPM);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn defaults_match_datasheet() {
        assert_eq!(u8::from(TearingEffectOn::new()), 0x00);
        assert_eq!(u8::from(Madctl::new()), 0x00);
        assert_eq!(u8::from(Colmod::new()), 0x77);
        assert_eq!(u8::from(DeepStandby::new()), 0x00);
        assert_eq!(u8::from(Brightness::new()), 0x00);
        assert_eq!(u8::from(CtrlDisplay::new()), 0x28);
        assert_eq!(u8::from(AclControl::new()), 0x00);
        assert_eq!(u8::from(ColorEnhance::new()), 0x00);
        assert_eq!(u8::from(HbmBrightness::new()), 0x00);
        assert_eq!(u8::from(HbmControl::new()), 0x00);
        assert_eq!(u8::from(SpiMode::new()), 0x00);
        assert_eq!(u8::from(CommandPage::new()), 0x00);
        assert_eq!(u8::from(DisplayPowerMode::new()), 0x08);
    }

    #[test]
    fn bit_positions() {
        assert_eq!(u8::from(TearingEffectOn::new().with_m(true)), 0x01);
        assert_eq!(u8::from(Madctl::new().with_my(true)), 0x80);
        assert_eq!(u8::from(Madctl::new().with_mx(true)), 0x40);
        assert_eq!(u8::from(Madctl::new().with_bgr(true)), 0x08);
        assert_eq!(u8::from(Colmod::uniform(PixelFormat::Rgb565)), 0x55);
        assert_eq!(u8::from(Colmod::uniform(PixelFormat::Rgb666)), 0x66);
        assert_eq!(
            u8::from(Colmod::new().with_spi_ifpf_sel(true)),
            0xF7,
            "SPI_IFPF_SEL is bit 7"
        );
        assert_eq!(u8::from(DeepStandby::new().with_dstb(true)), 0x01);
        assert_eq!(u8::from(CtrlDisplay::new().with_dd(false)), 0x20);
        assert_eq!(u8::from(AclControl::new().with_acl(AclMode::On)), 0x03);
        assert_eq!(
            u8::from(
                ColorEnhance::new()
                    .with_enable(true)
                    .with_level(EnhancementLevel::High)
            ),
            0x06
        );
        assert_eq!(u8::from(HbmControl::new().with_hbm_en(true)), 0x02);
        assert_eq!(u8::from(SpiMode::new().with_spi_wram(true)), 0x80);
        assert_eq!(
            u8::from(CommandPage::new().with_page(Page::Extension)),
            0x20
        );
        let dpm = DisplayPowerMode::from(0b1001_1100);
        assert!(dpm.bston() && dpm.slpout() && dpm.noron() && dpm.dispon());
        assert!(!dpm.idmon() && !dpm.ptlon());
    }

    #[test]
    fn spi_format_follows_select_bit() {
        let colmod = Colmod::new()
            .with_vipf(PixelFormat::Rgb565)
            .with_ifpf(PixelFormat::Rgb888);
        assert_eq!(colmod.spi_format(), PixelFormat::Rgb888);
        assert_eq!(
            colmod.with_spi_ifpf_sel(true).spi_format(),
            PixelFormat::Rgb565
        );
    }

    #[test]
    fn command_codes() {
        assert_eq!(Madctl::COMMAND, 0x36);
        assert_eq!(Colmod::COMMAND, 0x3A);
        assert_eq!(Brightness::COMMAND, 0x51);
        assert_eq!(SpiMode::COMMAND, 0xC4);
        assert_eq!(
            <Brightness as SPIRegister<RegisterAddress8, R1>>::REGISTER_ADDRESS.into_inner(),
            0x51
        );
    }
}
