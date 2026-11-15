//! Panel-specific command lists for [`Co5300::init`](crate::Co5300::init).
//!
//! These are not chip facts: they depend on the panel module. Check your panel vendor's
//! initialization code before relying on one.

use crate::PanelCommand;

/// The default vendor sequence of Espressif's Apache-2.0 `esp_lcd_co5300` component
/// (v2.2.0, `esp_lcd_co5300_spi.c`, `vendor_specific_init_default`), minus the parts `init`
/// already sends or that depend on the panel geometry.
///
/// | Command | Value | Meaning (datasheet) |
/// |---------|-------|---------------------|
/// | `FEh` | `00h` | user command set |
/// | `C4h` | `80h` | SPI write to SRAM enabled |
/// | `35h` | `00h` | tearing effect on, V-blanking |
/// | `53h` | `20h` | brightness control on, dimming off |
/// | `51h` | `FFh` | brightness 100 % |
/// | `63h` | `FFh` | HBM brightness 100 % |
///
/// Differences from the Espressif list: its window commands (`2Ah`, `2Bh`, for a panel with
/// a 6-column offset), `SLPOUT` and `DISPON` are left out; Espressif sends `35h` without a
/// parameter, while the datasheet gives it one, sent here as `00h`; Espressif waits 10 ms
/// after `35h`, `53h`, `51h` and `63h`, the datasheet asks for no wait.
pub const ESPRESSIF_DEFAULT: &[PanelCommand<'static>] = &[
    (0xFE, &[0x00]),
    (0xC4, &[0x80]),
    (0x35, &[0x00]),
    (0x53, &[0x20]),
    (0x51, &[0xFF]),
    (0x63, &[0xFF]),
];
