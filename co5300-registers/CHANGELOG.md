# Changelog

All notable changes to this project will be documented in this file.
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Command map following the CO5300 datasheet V0.01: the user command set (`CMD1`) codes,
  the single-parameter commands as registers (`TEON`, `MADCTL`, `COLMOD`, `DSTBON`,
  `WRDISBV`, `WRCTRLD`, `WRACL`, sunlight enhancement, HBM brightness and enable,
  `SetSPIMode`, command page, `RDDPM`) with their defaults, the `CASET`/`RASET` address
  range with its alignment rule, and the Quad-SPI instruction codes.
