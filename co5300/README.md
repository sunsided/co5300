# Chipone CO5300 AMOLED Display Controller Driver

[![Crates.io][crates-image]][crates-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
[![Safety Dance][safety-image]][safety-link]
![MSRV][msrv-image]
[![EUPL 1.2 licensed][license-eupl-image]][license-eupl-link]
[![Apache 2.0 licensed][license-apache-image]][license-apache-link]
[![MIT licensed][license-mit-image]][license-mit-link]

> A platform-agnostic, transport-agnostic driver for the Chipone CO5300 AMOLED display
> controller.

The driver speaks CO5300 commands through an `Interface` trait with two methods,
`write_command(cmd, params)` and `write_pixels(cmd, data)`, plus an async twin
(`AsyncInterface`, `async` feature). That fits:

- 4-wire SPI with a D/C pin: `SpiInterface`, on `embedded-hal` 1.0 `SpiDevice` and
  `OutputPin`;
- single-line SPI with the datasheet's `02h` instruction/address framing:
  `FramedSpiInterface`, no D/C pin needed;
- Quad-SPI: build an adapter on the `qspi` framing helper and your HAL's half-duplex
  transfer (no HAL dependency in this crate);
- MIPI-DSI through ESP-IDF's `esp_lcd_panel_io_tx_param`/`tx_color`, as on the Kode Dot
  (an example adapter is in the crate documentation).

```rust,ignore
use co5300::{Co5300, Config, SpiInterface};

let mut display = Co5300::new(SpiInterface::new(spi, dc), Config::new(410, 502));
co5300::reset(&mut rst, &mut delay)?;
display.init(&mut delay, &[])?; // panel-specific commands go in the slice

display.set_window(0, 0, 410, 2)?;
display.write_pixels(&rows)?;          // RAMWR
display.write_pixels_continue(&more)?; // RAMWRC
display.set_brightness(0x80)?;
```

With the `embedded-graphics` feature, the driver is a `DrawTarget<Color = Rgb565>` that
writes through the interface, a window and a contiguous fill at a time, with no frame
buffer in the host. The chip only accepts windows with an even start and length, so
unaligned areas are widened by one pixel with their edge pixels repeated.

## Implemented so far

- Hardware reset with datasheet timing, and initialization: panel commands, user command
  page, `SetSPIMode` for SPI transports, `SLPOUT`, `COLMOD`, brightness, `DISPON`
- Sleep in/out, display on/off, software reset, deep standby, idle mode
- Brightness, brightness control, HBM brightness and mode, ACL, sunlight enhancement
- Pixel format (RGB565, RGB666, RGB888), `MADCTL`, inversion
- Tearing effect on/off and scan line
- Memory window with alignment checks, memory write and continue
- Direct access to the command map via [`co5300-registers`]

Not implemented: reads (no interface needs them for drawing), partial mode, the
manufacturer command pages beyond passing them through `init`.

## Sources

Chip facts are from the [CO5300 datasheet][ds] V0.01 (24 July 2023); the register facts are
listed in [`co5300-registers`](https://crates.io/crates/co5300-registers). The driver adds:

| Fact | Source |
|------|--------|
| Interfaces: MIPI-DSI and Quad-SPI, selected by `IM[1:0]` | Datasheet 1; 4.4; 5.1 |
| Quad-SPI framing: `02h`, `{00h, CMD, 00h}`, parameters, all on `SIO0`; pixel instructions and line usage; RGB565 MSB first | Datasheet 5.2.1; 5.2.2 (figures on pages 21–24) |
| `D/CX`: low command, high parameter or display data; `DCX` setup/hold 4 ns | Datasheet 4.2; 6.4 |
| SDI sampled on the rising edge of SCL; write cycle ≥ 20 ns | Datasheet 5.2; 6.4 |
| `RESX` low ≥ 10 µs; wait 5 ms before commands and 120 ms before `SLPOUT` | Datasheet 6.6 |
| Power-on: initial code, then `SLPOUT`, then ≥ 60 ms to `DISPON` | Datasheet 5.6.2 (`T4`) |
| 5 ms after `SLPIN`/`SLPOUT`; 120 ms from `SLPOUT` to `SLPIN`; 83 ms from `SLPIN` to `RESX` low | Datasheet 7.5.11; 7.5.12; 5.6.3 (`T6`) |
| 5 ms after `SWRESET`, 120 ms before `SLPOUT` if reset in Sleep Out mode | Datasheet 7.5.2 |
| `RAMWR` starts at the window origin; `RAMWRC` continues | Datasheet 7.5.23; 7.5.33 |
| Window start and length even | Datasheet 7.5.21; 7.5.22 |
| Up to 480 source columns | Datasheet 1 |
| 4-wire SPI byte layout: one 8-bit command byte with D/C low, then parameters with D/C high | [`esp_lcd_co5300`][esp] v2.2.0, `CO5300_PANEL_IO_SPI_CONFIG` (`lcd_cmd_bits = 8`, `dc_gpio_num`) |
| SPI mode 0 works | `esp_lcd_co5300` v2.2.0, `spi_mode = 0` |
| SPI and Quad-SPI hosts set `C4h` = `80h` during initialization | `esp_lcd_co5300` v2.2.0, `vendor_specific_init_default`; [Zephyr][zephyr] `display_co5300.c`, `co5300_configure_panel` (SPI and MSPI buses only) |
| Plain SPI hosts use the `02h` single-line framing | Zephyr `chipone,co5300-spi.yaml` |
| `panel::ESPRESSIF_DEFAULT` command list | `esp_lcd_co5300` v2.2.0, `vendor_specific_init_default` |

No code was taken from `esp_lcd_co5300`, Zephyr or other CO5300 drivers; the facts above
were restated from them.

## Assumptions

- `CSX` may go high between a command byte and its parameters in 4-wire SPI mode.
- `init` holds the brightness from `Config` (default `FFh`), since the reset default `00h`
  shows nothing; it relies on `WRCTRLD`'s reset default (`BCTRL` set).
- `COLMOD` is written with the same format in `VIPF` and `IFPF`.
- `reset` holds `RESX` low for 10 ms, not the 10 µs minimum, so it also ends deep standby
  (more than 3 ms).

[ds]: https://files.waveshare.com/wiki/common/Co5300_Datasheet.pdf
[esp]: https://github.com/espressif/esp-iot-solution/tree/master/components/display/lcd/esp_lcd_co5300
[zephyr]: https://github.com/zephyrproject-rtos/zephyr/blob/main/drivers/display/display_co5300.c
[`co5300-registers`]: https://crates.io/crates/co5300-registers

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[crates-image]: https://img.shields.io/crates/v/co5300
[crates-link]: https://crates.io/crates/co5300
[docs-image]: https://docs.rs/co5300/badge.svg
[docs-link]: https://docs.rs/co5300/
[build-image]: https://github.com/sunsided/co5300/workflows/Rust/badge.svg
[build-link]: https://github.com/sunsided/co5300/actions
[safety-image]: https://img.shields.io/badge/unsafe-forbidden-success.svg
[safety-link]: https://github.com/rust-secure-code/safety-dance/
[msrv-image]: https://img.shields.io/badge/rustc-1.85+-blue.svg
[license-eupl-image]: https://img.shields.io/badge/license-EUPL_1.2-blue.svg
[license-eupl-link]: https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12
[license-apache-image]: https://img.shields.io/badge/license-Apache_2.0-blue.svg
[license-apache-link]: https://www.apache.org/licenses/LICENSE-2.0
[license-mit-image]: https://img.shields.io/badge/license-MIT-blue.svg
[license-mit-link]: https://opensource.org/licenses/MIT
