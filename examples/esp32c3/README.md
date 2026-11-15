# CO5300 on ESP32-C3

Initializes a CO5300 over 4-wire SPI with the blocking `co5300` driver and draws a test
pattern through its `embedded-graphics` target: four color bars and a gradient, written
straight into the chip's frame memory. It prints `CO5300 init ok` and `CO5300 frame drawn`
on the serial console.

| ESP32-C3 | CO5300 |
|----------|--------|
| GPIO4    | SCL    |
| GPIO5    | SDI    |
| GPIO6    | CSX    |
| GPIO7    | DCX    |
| GPIO10   | RESX   |

The panel is 410 × 502, as on the Kode Dot. There the CO5300 is on MIPI-DSI instead, see
the `co5300` crate documentation.

## Run in Wokwi

Requires `rustup target add riscv32imc-unknown-none-elf wasm32-unknown-unknown`,
[`wokwi-cli`](https://docs.wokwi.com/wokwi-ci/cli-installation) and `WOKWI_CLI_TOKEN`.

```sh
../../sim/wokwi-chip/build.sh
cargo build --release
wokwi-cli . --scenario scenarios/draw.yaml --timeout 30000
```

The scenario waits for both serial lines, then compares the chip's display with
`scenarios/screenshots/test-pattern.png` pixel by pixel. To update the reference after
changing the pattern, replace `compare-with` with `save-to` once.

## Quad-SPI

`embedded-hal` has no Quad-SPI trait and this example uses plain SPI. On real hardware with
esp-hal, a Quad-SPI adapter can build its transfers with `co5300::qspi::Frame` and
`Spi::half_duplex_write`; the `co5300::qspi` module documentation shows the mapping. It is
a reference only: the `co5300` crates do not depend on esp-hal.

## Run on hardware

```sh
cargo run --release   # uses espflash
```
