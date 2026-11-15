# Changelog

All notable changes to this project will be documented in this file.
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-15

[0.1.0]: https://github.com/sunsided/co5300/releases/tag/v0.1.0

### Added

- Transport-agnostic blocking driver on an `Interface` trait (`write_command`,
  `write_pixels`), and an async driver on `AsyncInterface` (`async` feature): hardware
  reset, initialization with a panel-specific command list, sleep, display on/off,
  brightness, HBM, ACL, sunlight enhancement, pixel format, `MADCTL`, inversion, tearing
  effect, idle mode, deep standby, memory window and memory writes.
- Interface adapters for 4-wire SPI with a D/C pin (`SpiInterface`) and single-line SPI
  with the datasheet's instruction/address framing (`FramedSpiInterface`).
- A HAL-independent Quad-SPI framing helper (`qspi`).
- An `embedded-graphics` `DrawTarget` that writes to the chip's frame memory without a
  host frame buffer (`embedded-graphics` feature).
- Espressif's default panel command list (`panel::ESPRESSIF_DEFAULT`).
