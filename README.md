# Chipone CO5300 AMOLED Display Controller

[![Build Status](https://github.com/sunsided/co5300/workflows/Rust/badge.svg)](https://github.com/sunsided/co5300/actions)
[![codecov](https://codecov.io/gh/sunsided/co5300/graph/badge.svg)](https://codecov.io/gh/sunsided/co5300)

> Platform-agnostic Rust support for the Chipone CO5300 AMOLED display controller, over
> SPI, Quad-SPI or MIPI-DSI.

> **Status:** tested against a command-level model of the chip (host tests and Wokwi), not
> yet verified on real hardware.

| Crate                                  | Description                                                          |
|----------------------------------------|----------------------------------------------------------------------|
| [`co5300-registers`](co5300-registers) | A typed map of the CO5300's commands and parameters.                 |
| [`co5300`](co5300)                     | A transport-agnostic driver (blocking and async), `embedded-graphics` target. |
| [`co5300-sim`](co5300-sim)             | A command-level model with a frame memory for tests and emulation.   |

Everything follows the Chipone CO5300 datasheet,
[V0.01](https://files.waveshare.com/wiki/common/Co5300_Datasheet.pdf) (24 July 2023), the
newest revision found. Facts the datasheet lacks, such as the 4-wire SPI byte layout and
`SetSPIMode` on SPI hosts, come from Espressif's Apache-2.0
[`esp_lcd_co5300`](https://github.com/espressif/esp-iot-solution/tree/master/components/display/lcd/esp_lcd_co5300)
component (v2.2.0) and Zephyr's Apache-2.0 CO5300 driver and bindings. No code was taken
from them or from other drivers. Each crate README has a Sources table.

## Interfaces

The datasheet documents MIPI-DSI and a Quad-SPI interface with Standard (one line), Dual and
Quad modes, including the instruction/address framing. For a 4-wire SPI mode with a D/C
pin it only names the `D/CX` pin and its timing; the byte layout comes from
`esp_lcd_co5300`.

The driver is written against an `Interface` trait (`write_command`, `write_pixels`), so the
transport lives outside it:

- `SpiInterface`: 4-wire SPI with a D/C pin on `embedded-hal`;
- `FramedSpiInterface`: single-line SPI with the `02h` framing;
- `co5300::qspi`: the Quad-SPI framing as plain data, for HAL-specific adapters;
- MIPI-DSI: an adapter on ESP-IDF's `esp_lcd_panel_io_tx_param`/`tx_color`, shown in the
  crate documentation, for boards such as the Kode Dot.

## Assumptions

Behavior the datasheet leaves open, and what these crates assume:

- In 4-wire SPI mode, `CSX` may go high between a command and its parameters.
- Pixel data is sent with `RAMWR` once and `RAMWRC` for every further chunk.
- Windows that break the even start/length rule are widened by the `embedded-graphics`
  target, repeating the edge pixels; the plain API rejects them.
- The panel-specific initialization (manufacturer pages, gamma, timing) is up to the user;
  `init` sends only chip-level commands.

The model's own assumptions (mirroring, window wrap-around, idle mode colors, reset pulse
length) are listed in the `co5300-sim` crate docs.

## Board: Kode Dot

The Kode Dot (ESP32-P4) has the CO5300 on MIPI-DSI; ESP-IDF keeps the DSI bus, DPI and the
frame buffer. The panel is 410 × 502 (`KODE_DOT` screen constant).

- The panel supply `VCI_EN` comes from TCA6408A pin P7: latch P7 high before making it an
  output, then hold 20 ms.
- Reset is shared with touch on GPIO7: low for 10 ms, then high, then wait 120 ms.
- The DSI PHY LDO runs at 2.5 V.

## Emulation

The model in `co5300-sim` is used both behind fake `embedded-hal` buses in host tests and
as a [Wokwi](https://wokwi.com) custom chip with a frame buffer (`sim/wokwi-chip`), driven
over 4-wire SPI since Wokwi has no Quad-SPI. The ESP32-C3 firmware in `examples/esp32c3`
draws a test pattern; its scenario checks the serial output and compares the chip's
display with a reference screenshot using `wokwi-cli`.

### Use the chip in your own Wokwi project

Every [GitHub Release](https://github.com/sunsided/co5300/releases) carries the chip as
`chip.zip`. Add a `chip-co5300` part and this dependency to your `diagram.json`, with the
latest release version written without the tag's `v` (`@v0.1.0` fails to load):

```json
"dependencies": { "chip-co5300": "github:sunsided/co5300@0.1.0" }
```

This works on [wokwi.com](https://wokwi.com), in the Wokwi VS Code extension and with
`wokwi-cli`; nothing needs to be built.

### Run the example locally

Needs [`wokwi-cli`](https://docs.wokwi.com/wokwi-ci/cli-installation) and a
`WOKWI_CLI_TOKEN` from the [Wokwi CI dashboard](https://wokwi.com/dashboard/ci):

```sh
task wokwi:test
```

[`sim/wokwi-chip`](sim/wokwi-chip/README.md) describes building the chip and its pins;
[`examples/esp32c3`](examples/esp32c3/README.md) the wiring and the scenario.

## Releasing

Both crates share one version. From `main`, with the changes listed under
`## [Unreleased]` in each crate's `CHANGELOG.md`:

```sh
cargo release <patch|minor|major>            # dry run
cargo release <patch|minor|major> --execute
```

This commits `Release X.Y.Z` and pushes the tag `vX.Y.Z`. The tag runs the release workflow,
which publishes `co5300-registers` and `co5300` to crates.io via
[trusted publishing](https://crates.io/docs/trusted-publishing) (no API token in the
repository; versions already on crates.io are skipped, and then no token is requested) and
then creates a GitHub Release named `X.Y.Z` with the changelog sections as notes.

Each GitHub Release carries the packaged `.crate` files and the Wokwi chip as `chip.zip`,
all with [build provenance attestations](https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations):

```sh
gh attestation verify chip.zip -R sunsided/co5300
```

## Code of Conduct

We abide by the [Contributor Covenant][cc] and ask that you do as well.

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[cc]: https://contributor-covenant.org
