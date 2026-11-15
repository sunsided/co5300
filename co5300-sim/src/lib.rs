//! # Chipone CO5300 command-level model
//!
//! A model of the CO5300 that executes commands and keeps a frame memory, used in two
//! places:
//!
//! - behind fake `embedded-hal` buses ([`FakeSpi`], [`FakeDc`], [`FakeReset`],
//!   [`FakeDelay`]) in host tests of the `co5300` driver, and
//! - wrapped into a [Wokwi](https://wokwi.com) custom chip with a frame buffer
//!   (`sim/wokwi-chip`).
//!
//! The model implements, following the datasheet V0.01:
//!
//! - both SPI framings: 4-wire SPI with the `D/CX` pin (command byte low, data high;
//!   section 4.2) and single-line SPI with the `02h` instruction and the
//!   `{00h, CMD, 00h}` address (section 5.2.2), selected with [`Protocol`];
//! - the frame memory: `CASET`/`RASET` windows with their alignment rule (sections 7.5.21,
//!   7.5.22), `RAMWR` from the window origin and `RAMWRC` continuing (7.5.23, 7.5.33), and
//!   the RGB565, RGB666 and RGB888 pixel formats of `COLMOD` (7.5.32, 5.2.2);
//! - `MADCTL` mirroring and BGR order (7.5.29);
//! - the display state: sleep, display on/off, all pixels on/off, inversion, idle mode,
//!   deep standby, and the brightness, ACL, HBM, SRE and tearing effect settings;
//! - reset by the `RESX` pin (at least 10 µs low) or `SWRESET`, restoring the defaults;
//! - the datasheet's waits, reported as [`Violation`]s: 5 ms after reset, `SWRESET`,
//!   `SLPIN` and `SLPOUT`; 120 ms from reset to `SLPOUT`; 60 ms from `SLPOUT` to `DISPON`;
//!   120 ms from `SLPOUT` to `SLPIN`.
//!
//! ## Not modelled
//!
//! Reads, the manufacturer command pages and their passwords, partial mode, the tearing
//! effect signal itself, the packed pixel formats (RGB111, RGB332, gray), the Dual and Quad
//! SPI modes, and MIPI-DSI.
//!
//! ## Assumptions
//!
//! The datasheet leaves these points open; the model behaves as follows:
//!
//! - The panel shows frame memory column `c` and row `r` at `(c, r)`; with `MX` or `MY` set
//!   it shows them at `width - 1 - c` or `height - 1 - r`. Writes outside the panel are
//!   dropped.
//! - A memory write that runs past the end of the window wraps to the window origin.
//! - Brightness does not change the [`visible_pixel`](Co5300Model::visible_pixel) colors.
//! - Idle mode shows each channel's MSB as full or no intensity.
//! - In 4-wire mode, `CSX` may go high between a command and its parameters.
//! - A `RESX` pulse shorter than 10 µs does not reset the chip (the datasheet rejects
//!   pulses below 5 µs and calls 5–10 µs "reset start").
//! - Unknown commands (including the manufacturer pages) are accepted and ignored.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

extern crate alloc;

mod bus;

pub use bus::{FakeDc, FakeDelay, FakeReset, FakeSpi};

use alloc::vec;
use alloc::vec::Vec;
use co5300_registers::{
    AclControl, AddressRange, Brightness, Colmod, ColorEnhance, Command, CommandPage, CtrlDisplay,
    DeepStandby, HbmBrightness, HbmControl, Madctl, PixelFormat, QspiInstruction, SpiMode,
    TearingEffectOn,
};

const MS: u64 = 1_000_000;

/// The SPI framing the model decodes.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Protocol {
    /// 4-wire SPI: the `D/CX` pin separates the command byte (low) from data (high).
    DcPin,
    /// Single-line SPI: each transfer is `02h`, `{00h, CMD, 00h}`, then data.
    Framed,
}

/// A rule of the datasheet the host broke.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A command arrived before the wait after a reset, `SWRESET`, `SLPIN` or `SLPOUT`.
    TooSoon {
        /// The command code.
        command: u8,
    },
    /// `SLPOUT` arrived within 120 ms of a reset.
    SleepOutAfterReset,
    /// `DISPON` arrived within 60 ms of `SLPOUT`.
    DisplayOnAfterSleepOut,
    /// `SLPIN` arrived within 120 ms of `SLPOUT`.
    SleepInAfterSleepOut,
    /// A `CASET` or `RASET` range that breaks the alignment rule.
    InvalidWindow {
        /// The command code.
        command: u8,
        /// The range.
        range: AddressRange,
    },
    /// A command with fewer parameter bytes than it needs.
    MissingParameters {
        /// The command code.
        command: u8,
    },
    /// A framed transfer with an unknown instruction or a malformed address.
    InvalidFrame,
    /// Pixel data in a format the model does not decode.
    UnsupportedFormat,
}

/// A rectangle of panel pixels, inclusive.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Region {
    /// The first column.
    pub x0: u16,
    /// The first row.
    pub y0: u16,
    /// The last column.
    pub x1: u16,
    /// The last row.
    pub y1: u16,
}

impl Region {
    fn include(&mut self, x: u16, y: u16) {
        self.x0 = self.x0.min(x);
        self.y0 = self.y0.min(y);
        self.x1 = self.x1.max(x);
        self.y1 = self.y1.max(y);
    }
}

/// The command state and settings of the chip; everything a reset restores.
#[derive(Debug, Clone)]
struct State {
    sleeping: bool,
    display_on: bool,
    idle: bool,
    inverted: bool,
    all_pixels: Option<bool>,
    deep_standby: bool,
    colmod: Colmod,
    madctl: Madctl,
    brightness: Brightness,
    ctrl_display: CtrlDisplay,
    acl: AclControl,
    enhance: ColorEnhance,
    hbm_brightness: HbmBrightness,
    hbm: HbmControl,
    tearing_effect: Option<TearingEffectOn>,
    tear_scanline: u16,
    page: CommandPage,
    spi_mode: SpiMode,
    columns: AddressRange,
    rows: AddressRange,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sleeping: true,
            display_on: false,
            idle: false,
            inverted: false,
            all_pixels: None,
            deep_standby: false,
            colmod: Colmod::new(),
            madctl: Madctl::new(),
            brightness: Brightness::new(),
            ctrl_display: CtrlDisplay::new(),
            acl: AclControl::new(),
            enhance: ColorEnhance::new(),
            hbm_brightness: HbmBrightness::new(),
            hbm: HbmControl::new(),
            tearing_effect: None,
            tear_scanline: 0,
            page: CommandPage::new(),
            spi_mode: SpiMode::new(),
            columns: AddressRange::DEFAULT,
            rows: AddressRange::DEFAULT,
        }
    }
}

/// A command-level model of the CO5300 with its frame memory.
#[derive(Debug, Clone)]
pub struct Co5300Model {
    width: u16,
    height: u16,
    /// The frame memory as `0x00RRGGBB`, one entry per panel pixel.
    memory: Vec<u32>,
    state: State,

    protocol: Protocol,
    dc: bool,
    /// The current command and its parameter bytes so far.
    command: Option<u8>,
    params: Vec<u8>,
    /// The bytes of a framed transfer's header seen so far.
    header: Vec<u8>,
    /// The write position, in frame memory columns and rows.
    cursor: (u16, u16),
    /// The bytes of an incomplete pixel.
    pending: Vec<u8>,

    now_ns: u64,
    reset_low_since: Option<u64>,
    /// No command before this time.
    busy_until: u64,
    /// No `SLPOUT` before this time.
    sleep_out_from: u64,
    /// No `DISPON` before this time.
    display_on_from: u64,
    /// No `SLPIN` before this time.
    sleep_in_from: u64,

    log: Vec<u8>,
    violations: Vec<Violation>,
    dirty: Option<Region>,
}

impl Co5300Model {
    /// Creates a model of a `width` × `height` panel, freshly powered on and reset, with
    /// the 4-wire SPI protocol.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            memory: vec![0; usize::from(width) * usize::from(height)],
            state: State::default(),
            protocol: Protocol::DcPin,
            dc: true,
            command: None,
            params: Vec::new(),
            header: Vec::new(),
            cursor: (0, 0),
            pending: Vec::new(),
            now_ns: 0,
            reset_low_since: None,
            // Created as if `RESX` was just released (section 6.6, note 5).
            busy_until: 5 * MS,
            sleep_out_from: 120 * MS,
            display_on_from: 0,
            sleep_in_from: 0,
            log: Vec::new(),
            violations: Vec::new(),
            dirty: None,
        }
    }

    /// Selects the SPI framing.
    pub fn with_protocol(mut self, protocol: Protocol) -> Self {
        self.protocol = protocol;
        self
    }

    /// The panel width in pixels.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// The panel height in pixels.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Restores the defaults, as `SWRESET` or a `RESX` pulse does. The frame memory keeps
    /// its contents.
    pub fn reset(&mut self) {
        self.state = State::default();
        self.command = None;
        self.params.clear();
        self.pending.clear();
        self.busy_until = self.now_ns + 5 * MS;
        self.sleep_out_from = self.now_ns + 120 * MS;
        self.mark_all_dirty();
    }

    /// Sets the level of the active-low `RESX` pin.
    pub fn set_reset_pin(&mut self, high: bool) {
        match (high, self.reset_low_since) {
            (false, None) => self.reset_low_since = Some(self.now_ns),
            (true, Some(since)) => {
                self.reset_low_since = None;
                let low_ns = self.now_ns - since;
                // Deep standby needs a pulse of more than 3 ms (section 7.5.37).
                let needed = if self.state.deep_standby {
                    3 * MS
                } else {
                    10_000
                };

                if low_ns >= needed {
                    self.reset();
                }
            }
            _ => {}
        }
    }

    /// Sets the level of the `D/CX` pin: low selects a command, high data.
    pub fn set_dc_pin(&mut self, high: bool) {
        self.dc = high;
    }

    /// Advances the model's time.
    pub fn advance_ns(&mut self, ns: u64) {
        self.now_ns += ns;
    }

    /// The command codes received so far, in order.
    pub fn commands(&self) -> &[u8] {
        &self.log
    }

    /// The datasheet rules broken so far.
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// Clears the [`commands`](Self::commands) and [`violations`](Self::violations) logs,
    /// so a long-running host that never reads them keeps a flat memory footprint.
    pub fn clear_log(&mut self) {
        self.log.clear();
        self.violations.clear();
    }

    /// The pixel in frame memory at panel position `(x, y)`, as `0x00RRGGBB`.
    pub fn pixel(&self, x: u16, y: u16) -> u32 {
        self.memory[usize::from(y) * usize::from(self.width) + usize::from(x)]
    }

    /// The color the panel shows at `(x, y)`, as `0x00RRGGBB`: black while asleep, off or
    /// in deep standby, then all-pixels mode, inversion and idle mode applied.
    pub fn visible_pixel(&self, x: u16, y: u16) -> u32 {
        let s = &self.state;

        if s.sleeping || !s.display_on || s.deep_standby {
            return 0;
        }

        match s.all_pixels {
            Some(true) => return 0xFF_FF_FF,
            Some(false) => return 0,
            None => {}
        }

        let mut color = self.pixel(x, y);

        if s.inverted {
            color = !color & 0xFF_FF_FF;
        }

        if s.idle {
            color = (0..3).fold(0, |acc, channel| {
                let shift = channel * 8;
                let on = (color >> shift) & 0x80 != 0;
                acc | (u32::from(on) * 0xFF) << shift
            });
        }

        color
    }

    /// Returns and clears the panel area whose visible pixels changed.
    pub fn take_dirty(&mut self) -> Option<Region> {
        self.dirty.take()
    }

    /// Whether the chip is in Sleep In mode.
    pub fn is_sleeping(&self) -> bool {
        self.state.sleeping
    }

    /// Whether the display is on (`DISPON`).
    pub fn is_display_on(&self) -> bool {
        self.state.display_on
    }

    /// Whether Idle mode is on.
    pub fn is_idle(&self) -> bool {
        self.state.idle
    }

    /// Whether the chip is in deep standby.
    pub fn is_deep_standby(&self) -> bool {
        self.state.deep_standby
    }

    /// The `COLMOD` setting.
    pub fn colmod(&self) -> Colmod {
        self.state.colmod
    }

    /// The `MADCTL` setting.
    pub fn madctl(&self) -> Madctl {
        self.state.madctl
    }

    /// The display brightness (`WRDISBV`).
    pub fn brightness(&self) -> u8 {
        self.state.brightness.dbv()
    }

    /// The brightness control (`WRCTRLD`).
    pub fn ctrl_display(&self) -> CtrlDisplay {
        self.state.ctrl_display
    }

    /// The ACL setting.
    pub fn acl(&self) -> AclControl {
        self.state.acl
    }

    /// The sunlight readable enhancement setting.
    pub fn color_enhance(&self) -> ColorEnhance {
        self.state.enhance
    }

    /// The HBM brightness and whether HBM mode is on.
    pub fn hbm(&self) -> (u8, bool) {
        (self.state.hbm_brightness.dbv(), self.state.hbm.hbm_en())
    }

    /// The tearing effect output setting, `None` when off, and the scan line.
    pub fn tearing_effect(&self) -> (Option<TearingEffectOn>, u16) {
        (self.state.tearing_effect, self.state.tear_scanline)
    }

    /// Whether SPI writes to the frame memory are enabled (`SetSPIMode`, C4h).
    pub fn spi_ram_write(&self) -> bool {
        self.state.spi_mode.spi_wram()
    }

    /// The memory window (`CASET`, `RASET`).
    pub fn window(&self) -> (AddressRange, AddressRange) {
        (self.state.columns, self.state.rows)
    }

    /// Starts an SPI transaction (`CSX` low).
    pub fn spi_select(&mut self) {
        self.header.clear();
    }

    /// Receives one SPI byte.
    pub fn spi_byte(&mut self, byte: u8) {
        if self.reset_low_since.is_some() || self.state.deep_standby {
            return;
        }

        match self.protocol {
            Protocol::DcPin if !self.dc => self.begin(byte),
            Protocol::DcPin => self.data(byte),
            Protocol::Framed if self.header.len() < 4 => {
                self.header.push(byte);

                if self.header.len() == 4 {
                    let valid = self.header[0] == QspiInstruction::Write.code()
                        && self.header[1] == 0
                        && self.header[3] == 0;

                    if valid {
                        self.begin(self.header[2]);
                    } else {
                        self.violations.push(Violation::InvalidFrame);
                    }
                }
            }
            Protocol::Framed => {
                if self.command.is_some() {
                    self.data(byte);
                }
            }
        }
    }

    /// Ends an SPI transaction (`CSX` high).
    pub fn spi_deselect(&mut self) {
        // A framed transfer carries exactly one command.
        if self.protocol == Protocol::Framed {
            if (1..4).contains(&self.header.len()) {
                self.violations.push(Violation::InvalidFrame);
            }

            self.finish();
        }
    }

    /// Executes a command with all its parameter bytes, as a whole transfer would.
    pub fn execute(&mut self, command: u8, params: &[u8]) {
        self.begin(command);
        for &byte in params {
            self.data(byte);
        }
        self.finish();
    }

    /// Starts a new command; finishes the previous one.
    fn begin(&mut self, command: u8) {
        self.finish();
        self.log.push(command);
        self.check_timing(command);
        self.command = Some(command);
        self.params.clear();
        self.pending.clear();

        if command == Command::RAMWR.code() {
            self.cursor = (self.state.columns.start, self.state.rows.start);
        }

        if expected_params(command) == Some(0) {
            self.apply();
        }
    }

    /// Receives a parameter or pixel byte of the current command.
    fn data(&mut self, byte: u8) {
        let Some(command) = self.command else {
            return;
        };

        if command == Command::RAMWR.code() || command == Command::RAMWRC.code() {
            self.memory_byte(byte);
            return;
        }

        self.params.push(byte);

        if expected_params(command) == Some(self.params.len()) {
            self.apply();
        }
    }

    /// Ends the current command, applying it if it still waits for parameters.
    fn finish(&mut self) {
        if self.command.is_some() {
            self.apply();
        }
    }

    fn check_timing(&mut self, command: u8) {
        let now = self.now_ns;

        if now < self.busy_until {
            self.violations.push(Violation::TooSoon { command });
        }

        if command == Command::SLPOUT.code() && now < self.sleep_out_from {
            self.violations.push(Violation::SleepOutAfterReset);
        }

        if command == Command::DISPON.code() && now < self.display_on_from {
            self.violations.push(Violation::DisplayOnAfterSleepOut);
        }

        if command == Command::SLPIN.code() && now < self.sleep_in_from {
            self.violations.push(Violation::SleepInAfterSleepOut);
        }
    }

    /// Executes the current command with its parameters and clears it.
    fn apply(&mut self) {
        let Some(code) = self.command.take() else {
            return;
        };

        let params = core::mem::take(&mut self.params);

        if let Some(expected) = expected_params(code) {
            if params.len() < expected {
                self.violations
                    .push(Violation::MissingParameters { command: code });
                return;
            }
        }

        let first = params.first().copied().unwrap_or_default();
        let now = self.now_ns;
        let s = &mut self.state;
        let mut visible = true;

        match Command::from_code(code) {
            Some(Command::SWRESET) => {
                self.reset();
                return;
            }
            Some(Command::SLPIN) => {
                s.sleeping = true;
                self.busy_until = now + 5 * MS;
            }
            Some(Command::SLPOUT) => {
                s.sleeping = false;
                self.busy_until = now + 5 * MS;
                self.display_on_from = now + 60 * MS;
                self.sleep_in_from = now + 120 * MS;
            }
            Some(Command::NORON) => s.all_pixels = None,
            Some(Command::INVOFF) => s.inverted = false,
            Some(Command::INVON) => s.inverted = true,
            Some(Command::ALLPOFF) => s.all_pixels = Some(false),
            Some(Command::ALLPON) => s.all_pixels = Some(true),
            Some(Command::DISPOFF) => s.display_on = false,
            Some(Command::DISPON) => s.display_on = true,
            Some(Command::CASET) | Some(Command::RASET) => {
                let range = AddressRange::from_params([params[0], params[1], params[2], params[3]]);

                if !range.is_valid() {
                    self.violations.push(Violation::InvalidWindow {
                        command: code,
                        range,
                    });
                }

                if code == Command::CASET.code() {
                    s.columns = range;
                } else {
                    s.rows = range;
                }
                visible = false;
            }
            Some(Command::TEOFF) => s.tearing_effect = None,
            Some(Command::TEON) => s.tearing_effect = Some(TearingEffectOn::from(first)),
            Some(Command::MADCTL) => s.madctl = Madctl::from(first),
            Some(Command::IDMOFF) => s.idle = false,
            Some(Command::IDMON) => s.idle = true,
            Some(Command::COLMOD) => s.colmod = Colmod::from(first),
            Some(Command::STESL) => {
                s.tear_scanline = u16::from_be_bytes([params[0], params[1]]);
                visible = false;
            }
            Some(Command::DSTBON) => s.deep_standby = DeepStandby::from(first).dstb(),
            Some(Command::WRDISBV) => s.brightness = Brightness::from(first),
            Some(Command::WRCTRLD) => s.ctrl_display = CtrlDisplay::from(first),
            Some(Command::WRACL) => s.acl = AclControl::from(first),
            Some(Command::WRIMGEHCCTR) => s.enhance = ColorEnhance::from(first),
            Some(Command::WRHBMDISBV) => s.hbm_brightness = HbmBrightness::from(first),
            Some(Command::HBMCTL) => s.hbm = HbmControl::from(first),
            Some(Command::SETSPIMODE) => s.spi_mode = SpiMode::from(first),
            Some(Command::PAGESEL) => s.page = CommandPage::from(first),
            _ => visible = false,
        }

        if visible {
            self.mark_all_dirty();
        }
    }

    /// Receives a byte of pixel data.
    fn memory_byte(&mut self, byte: u8) {
        let format = self.state.colmod.spi_format();
        let Some(size) = format.bytes_per_pixel() else {
            if self.pending.is_empty() {
                self.violations.push(Violation::UnsupportedFormat);
                self.pending.push(byte);
            }
            return;
        };

        self.pending.push(byte);

        if self.pending.len() < size {
            return;
        }

        let p = core::mem::take(&mut self.pending);
        let (mut r, g, mut b) = match format {
            PixelFormat::Rgb565 => {
                let v = u16::from_be_bytes([p[0], p[1]]);
                let (r, g, b) = ((v >> 11) as u8, (v >> 5) as u8 & 0x3F, v as u8 & 0x1F);
                (r << 3 | r >> 2, g << 2 | g >> 4, b << 3 | b >> 2)
            }
            // Six bits in the upper bits of each byte (section 5.2.2, "RGB666").
            PixelFormat::Rgb666 => (p[0] | p[0] >> 6, p[1] | p[1] >> 6, p[2] | p[2] >> 6),
            _ => (p[0], p[1], p[2]),
        };

        if self.state.madctl.bgr() {
            core::mem::swap(&mut r, &mut b);
        }

        self.store(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b));
    }

    /// Stores a pixel at the cursor and advances it through the window.
    fn store(&mut self, color: u32) {
        let (column, row) = self.cursor;
        let (columns, rows) = (self.state.columns, self.state.rows);

        let x = if self.state.madctl.mx() {
            self.width.checked_sub(1 + column)
        } else {
            Some(column)
        };
        let y = if self.state.madctl.my() {
            self.height.checked_sub(1 + row)
        } else {
            Some(row)
        };

        if let (Some(x), Some(y)) = (x, y) {
            if x < self.width && y < self.height {
                let i = usize::from(y) * usize::from(self.width) + usize::from(x);
                if self.memory[i] != color {
                    self.memory[i] = color;
                    match &mut self.dirty {
                        Some(region) => region.include(x, y),
                        None => {
                            self.dirty = Some(Region {
                                x0: x,
                                y0: y,
                                x1: x,
                                y1: y,
                            })
                        }
                    }
                }
            }
        }

        self.cursor = if column >= columns.end {
            let next = if row >= rows.end { rows.start } else { row + 1 };
            (columns.start, next)
        } else {
            (column + 1, row)
        };
    }

    fn mark_all_dirty(&mut self) {
        if self.width > 0 && self.height > 0 {
            self.dirty = Some(Region {
                x0: 0,
                y0: 0,
                x1: self.width - 1,
                y1: self.height - 1,
            });
        }
    }
}

/// The number of parameter bytes a command takes, if the model knows the command.
fn expected_params(command: u8) -> Option<usize> {
    use Command::*;
    Some(match Command::from_code(command)? {
        NOP | SWRESET | SLPIN | SLPOUT | PTLON | NORON | INVOFF | INVON | ALLPOFF | ALLPON
        | DISPOFF | DISPON | TEOFF | IDMOFF | IDMON => 0,
        CASET | RASET | PTLAR | PTLAR_H => 4,
        STESL => 2,
        TEON | MADCTL | COLMOD | DSTBON | WRDISBV | WRCTRLD | WRACL | WRIMGEHCCTR | WRHBMDISBV
        | HBMCTL | SETSPIMODE | PAGESEL | SETDISPMODE | COLOPT => 1,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(model: &mut Co5300Model) {
        model.advance_ns(120 * MS);
        model.execute(0x11, &[]);
        model.advance_ns(60 * MS);
        model.execute(0x3A, &[0x55]);
        model.execute(0x29, &[]);
    }

    #[test]
    fn clear_log_empties_commands_and_violations() {
        let mut model = Co5300Model::new(4, 4);
        // SLPOUT right after reset breaks the 120 ms rule.
        model.execute(0x11, &[]);
        assert!(!model.commands().is_empty());
        assert!(!model.violations().is_empty());

        model.clear_log();
        assert!(model.commands().is_empty());
        assert!(model.violations().is_empty());
    }

    #[test]
    fn defaults() {
        let model = Co5300Model::new(8, 8);
        assert!(model.is_sleeping());
        assert!(!model.is_display_on());
        assert_eq!(u8::from(model.colmod()), 0x77);
        assert_eq!(
            model.window(),
            (AddressRange::DEFAULT, AddressRange::DEFAULT)
        );
        assert_eq!(model.visible_pixel(0, 0), 0);
    }

    #[test]
    fn rgb565_window_write_and_continue() {
        let mut model = Co5300Model::new(8, 8);
        ready(&mut model);
        model.execute(0x2A, &[0, 2, 0, 3]);
        model.execute(0x2B, &[0, 4, 0, 5]);
        model.execute(0x2C, &[0xF8, 0x00, 0x07, 0xE0]);
        model.execute(0x3C, &[0x00, 0x1F, 0xFF, 0xFF]);
        assert_eq!(model.pixel(2, 4), 0xFF0000);
        assert_eq!(model.pixel(3, 4), 0x00FF00);
        assert_eq!(model.pixel(2, 5), 0x0000FF);
        assert_eq!(model.pixel(3, 5), 0xFFFFFF);
        assert_eq!(model.visible_pixel(2, 4), 0xFF0000);
        assert!(model.violations().is_empty(), "{:?}", model.violations());
    }

    #[test]
    fn rgb888_and_666() {
        let mut model = Co5300Model::new(4, 4);
        ready(&mut model);
        model.execute(0x3A, &[0x77]);
        model.execute(0x2A, &[0, 0, 0, 1]);
        model.execute(0x2B, &[0, 0, 0, 1]);
        model.execute(0x2C, &[0x12, 0x34, 0x56]);
        assert_eq!(model.pixel(0, 0), 0x123456);
        model.execute(0x3A, &[0x66]);
        model.execute(0x2C, &[0xFC, 0x00, 0x80]);
        assert_eq!(model.pixel(0, 0), 0xFF0082);
    }

    #[test]
    fn unaligned_window_is_reported() {
        let mut model = Co5300Model::new(8, 8);
        ready(&mut model);
        model.execute(0x2A, &[0, 1, 0, 2]);
        assert_eq!(
            model.violations(),
            [Violation::InvalidWindow {
                command: 0x2A,
                range: AddressRange::new(1, 2)
            }]
        );
    }

    #[test]
    fn timing_rules() {
        let mut model = Co5300Model::new(4, 4);
        model.execute(0x11, &[]);
        model.execute(0x29, &[]);
        assert_eq!(
            model.violations(),
            [
                Violation::TooSoon { command: 0x11 },
                Violation::SleepOutAfterReset,
                Violation::TooSoon { command: 0x29 },
                Violation::DisplayOnAfterSleepOut,
            ]
        );
    }

    #[test]
    fn display_modes() {
        let mut model = Co5300Model::new(2, 2);
        ready(&mut model);
        model.execute(0x2C, &[0x80, 0x00]);
        let color = model.pixel(0, 0);
        assert_eq!(model.visible_pixel(0, 0), color);
        model.execute(0x21, &[]);
        assert_eq!(model.visible_pixel(0, 0), !color & 0xFFFFFF);
        model.execute(0x20, &[]);
        model.execute(0x39, &[]);
        assert_eq!(model.visible_pixel(0, 0), 0xFF0000);
        model.execute(0x23, &[]);
        assert_eq!(model.visible_pixel(0, 0), 0xFFFFFF);
        model.execute(0x13, &[]);
        model.execute(0x28, &[]);
        assert_eq!(model.visible_pixel(0, 0), 0);
    }

    #[test]
    fn madctl_mirrors_and_swaps() {
        let mut model = Co5300Model::new(4, 4);
        ready(&mut model);
        model.execute(0x36, &[0xC8]);
        model.execute(0x2A, &[0, 0, 0, 1]);
        model.execute(0x2B, &[0, 0, 0, 1]);
        model.execute(0x2C, &[0xF8, 0x00]);
        assert_eq!(model.pixel(3, 3), 0x0000FF);
    }

    #[test]
    fn reset_pin_needs_10_us() {
        let mut model = Co5300Model::new(4, 4);
        ready(&mut model);
        model.set_reset_pin(false);
        model.advance_ns(5_000);
        model.set_reset_pin(true);
        assert!(model.is_display_on());
        model.set_reset_pin(false);
        model.advance_ns(10_000);
        model.set_reset_pin(true);
        assert!(!model.is_display_on());
        assert!(model.is_sleeping());
    }

    #[test]
    fn deep_standby_ignores_bus_until_long_reset() {
        let mut model = Co5300Model::new(4, 4);
        ready(&mut model);
        model.execute(0x4F, &[0x01]);
        assert!(model.is_deep_standby());
        model.spi_select();
        model.set_dc_pin(false);
        model.spi_byte(0x28);
        model.spi_deselect();
        assert!(model.is_display_on(), "commands are ignored");
        model.set_reset_pin(false);
        model.advance_ns(MS);
        model.set_reset_pin(true);
        assert!(model.is_deep_standby());
        model.set_reset_pin(false);
        model.advance_ns(4 * MS);
        model.set_reset_pin(true);
        assert!(!model.is_deep_standby());
    }

    #[test]
    fn framed_protocol() {
        let mut model = Co5300Model::new(4, 4).with_protocol(Protocol::Framed);
        model.advance_ns(120 * MS);
        for transfer in [
            &[0x02, 0x00, 0x11, 0x00][..],
            &[0x02, 0x00, 0x3A, 0x00, 0x55],
            &[0x02, 0x00, 0x2A, 0x00, 0, 0, 0, 1],
            &[0x02, 0x00, 0x2B, 0x00, 0, 0, 0, 1],
            &[0x02, 0x00, 0x2C, 0x00, 0x07, 0xE0],
        ] {
            model.spi_select();
            transfer.iter().for_each(|&b| model.spi_byte(b));
            model.spi_deselect();
            model.advance_ns(10 * MS);
        }
        assert_eq!(model.pixel(0, 0), 0x00FF00);
        assert_eq!(model.commands(), [0x11, 0x3A, 0x2A, 0x2B, 0x2C]);

        model.spi_select();
        [0x32, 0x00, 0x2C, 0x00]
            .iter()
            .for_each(|&b| model.spi_byte(b));
        model.spi_deselect();
        assert_eq!(model.violations(), [Violation::InvalidFrame]);
    }
}
