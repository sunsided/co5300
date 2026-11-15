# CO5300 Wokwi custom chip

Wraps the [`co5300-sim`](../../co5300-sim) command-level model into a
[Wokwi custom chip](https://docs.wokwi.com/chips-api/getting-started) with a frame buffer,
so it can stand in for a CO5300 AMOLED panel in Wokwi projects.

```sh
./build.sh   # produces co5300.chip.wasm next to co5300.chip.json
```

Reference it from a project's `wokwi.toml`:

```toml
[[chip]]
name = 'co5300'
binary = '../../sim/wokwi-chip/co5300.chip.wasm'
```

and add a `chip-co5300` part to `diagram.json`.

Projects outside this repository (on wokwi.com, in VS Code or with `wokwi-cli`) can use
the `chip.zip` attached to each GitHub Release instead of building the chip. Add a
`chip-co5300` part and a dependency to `diagram.json`; no `[[chip]]` entry is needed:

```json
"dependencies": { "chip-co5300": "github:sunsided/co5300@0.1.0" }
```

The version is given without the `v` prefix of the release tag; `@v0.1.0` fails to load.

The display is 410 × 502 pixels, the Kode Dot's panel. Wokwi has no Quad-SPI, so the chip
listens on plain SPI, mode 0.

| Pin    | Function                                                          |
|--------|-------------------------------------------------------------------|
| `CSX`  | Chip select, active low (pulled up)                               |
| `SCL`  | SPI clock                                                         |
| `SDI`  | SPI data in                                                       |
| `DCX`  | Data/command select: low command, high parameters or pixels       |
| `RESX` | Reset, active low (pulled up)                                     |
| `VDDI`, `GND` | Power; not modelled                                        |

| Attribute | Values | Function |
|-----------|--------|----------|
| `framed`  | `0` (default), `1` | `1` decodes the single-line `02h` framing (datasheet section 5.2.2) and ignores `DCX` |

Differences from the real part:

- No Quad-SPI, no MIPI-DSI, no reads (`SDO` is not connected).
- The display shows the frame memory at full intensity; brightness, ACL, HBM and
  sunlight enhancement are stored but do not change the picture.
- `DCX` is sampled when `CSX` goes low, so it must not change during a transfer. The
  `co5300` driver only changes it while `CSX` is high.
- Commands take effect immediately; the waits the datasheet asks for are not enforced, only
  tracked by the model (see `co5300-sim`).
