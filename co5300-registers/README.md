# Chipone CO5300 Display Controller Commands

[![Crates.io][crates-image]][crates-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
[![Safety Dance][safety-image]][safety-link]
![MSRV][msrv-image]
[![EUPL 1.2 licensed][license-eupl-image]][license-eupl-link]
[![Apache 2.0 licensed][license-apache-image]][license-apache-link]
[![MIT licensed][license-mit-image]][license-mit-link]

> A typed map of the Chipone CO5300 AMOLED display controller's command set.

The CO5300 is controlled with 8-bit command codes followed by parameter bytes, on every
host interface. This crate lists the codes (`Command`), models the single-parameter
commands as registers with named fields, built on
[`hardware-registers`](https://crates.io/crates/hardware-registers)' SPI register traits
(command code as register address, parameter byte as value), and encodes the `CASET`/`RASET`
address range and the Quad-SPI instruction codes.

The map follows the [CO5300 datasheet][ds] V0.01 (Chipone, 24 July 2023), the newest
revision found.

## Sources

All facts are from datasheet V0.01.

| Fact | Source |
|------|--------|
| Command codes and parameter counts of the user command set (`CMD1`) | 7.4 Command List |
| `TEON` (35h) `M` in bit 0 | 7.4; 7.5.28 |
| `MADCTL` (36h): `MY` bit 7, `MX` bit 6, `RGB` bit 3, default `00h` | 7.4; 7.5.29 |
| `COLMOD` (3Ah): `SPI_IFPF_SEL` bit 7, `VIPF[2:0]` bits 6–4, `IFPF[2:0]` bits 2–0, default `77h`; format codes | 7.4; 7.5.32 |
| `DSTBON` (4Fh) `DSTB` bit 0; exit by a `RESX` low pulse of more than 3 ms | 7.5.37 |
| `WRDISBV` (51h), `WRHBMDISBV` (63h): `00h` 0 %, `FFh` 100 %, default `00h` | 7.5.38; 7.5.46 |
| `WRCTRLD` (53h): `BCTRL` bit 5, `DD` bit 3, default `28h` | 7.4; 7.5.40 |
| `WRACL` (55h): `ACL[1:0]`, `00` off, `11` on | 7.5.42 |
| 58h: `SLR_EN` bit 2, `SLR_LEVEL[1:0]` low/medium/high | 7.4; 7.5.44 |
| 66h: `HBM_EN` bit 1 | 7.4; 7.5.49 |
| `SetSPIMode` (C4h): `SPI_WRAM` bit 7, SPI write to SRAM enable | 7.4; 7.5.57 |
| `CMD Page Switch` (FEh): page in bits 7–4 (`00h`, `10h` … `80h`) | 7.5.61 |
| `RDDPM` (0Ah): `BSTON` … `DISPON` in bits 7–2, default `08h` | 7.4 |
| `CASET`/`RASET`: 10-bit start and end, defaults `0`–`1C5h`; start ≤ end, start and length even | 7.4; 7.5.21; 7.5.22 |
| Quad-SPI instructions `02h`, `03h`, `32h`, `12h`, `72h`, `52h`, `38h`, `3Bh`, `FFh`; address `{00h, CMD, 00h}` | 5.2.1; 5.2.2; 5.2.3 |

Datasheet inconsistencies: section 7.5.59 gives the default of `RDID2` (DBh) as `11h`, the
command list `10h`; the command list's `COLMOD` table lists `001` twice. Section 7.5.9
names 0Eh `RDDIM`; this crate calls it `RDDSM` (read display signal mode).

See also the [`co5300`](https://crates.io/crates/co5300) driver.

[ds]: https://files.waveshare.com/wiki/common/Co5300_Datasheet.pdf

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[crates-image]: https://img.shields.io/crates/v/co5300-registers
[crates-link]: https://crates.io/crates/co5300-registers
[docs-image]: https://docs.rs/co5300-registers/badge.svg
[docs-link]: https://docs.rs/co5300-registers/
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
