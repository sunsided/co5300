//! Quad-SPI framing, independent of any HAL.
//!
//! `embedded-hal` has no multi-line SPI trait, so this module only computes what goes on the
//! wire; feed the result to your HAL's half-duplex transfer. Datasheet section 5.2: every
//! transfer starts with an 8-bit instruction, then a 24-bit address that carries the
//! command code as `{00h, CMD[7:0], 00h}`, then the data. In Standard SPI mode (the mode
//! after reset) the instruction is sent on one line; the pixel write instructions choose
//! how many lines carry the address and the data:
//!
//! | Instruction | Address | Data | Use |
//! |-------------|---------|------|-----|
//! | `02h` | 1 line, 24 clocks | 1 line | commands and parameters; pixels |
//! | `32h` | 1 line, 24 clocks | 4 lines | pixels |
//! | `12h` | 4 lines, 6 clocks | 4 lines | pixels |
//! | `72h` | 1 line, 24 clocks | 2 lines | pixels |
//! | `52h` | 2 lines, 12 clocks | 2 lines | pixels |
//!
//! Pixel bytes are sent MSB first; on four lines, `SIO3` carries bit 7 and `SIO0` bit 4 of
//! the first clock, then bits 3 and 0 on the second. RGB565 is two bytes per pixel, red in
//! the upper bits of the first byte (section 5.2.2, "RGB565").
//!
//! The Dual and Quad SPI modes entered with `3Bh` and `38h`, where the instruction itself
//! uses two or four lines, are not modelled here.
//!
//! ## Example: esp-hal
//!
//! For reference only; this crate does not depend on esp-hal. With esp-hal 1.x, a quad
//! pixel write maps onto `Spi::half_duplex_write`:
//!
//! ```rust,ignore
//! use co5300::qspi::{Frame, PixelMode};
//! use esp_hal::spi::master::{Address, Command, DataMode};
//!
//! let frame = Frame::pixels(0x2C, PixelMode::Quad);
//! spi.half_duplex_write(
//!     DataMode::Quad,
//!     Command::_8Bit(frame.instruction.into(), DataMode::Single),
//!     Address::_24Bit(frame.address, DataMode::Single),
//!     0,
//!     pixels,
//! )?;
//! ```

use co5300_registers::{QspiInstruction, qspi_address};

/// The number of data lines used in a phase of a transfer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Lines {
    /// One line, `SIO0`.
    Single = 1,
    /// Two lines, `SIO0` and `SIO1`.
    Dual = 2,
    /// Four lines, `SIO0` to `SIO3`.
    Quad = 4,
}

/// How a pixel write uses the data lines, selecting its instruction code.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PixelMode {
    /// `02h`: address and data on one line.
    Single,
    /// `72h`: address on one line, data on two.
    Dual,
    /// `52h`: address and data on two lines.
    DualAddress,
    /// `32h`: address on one line, data on four.
    Quad,
    /// `12h`: address and data on four lines.
    QuadAddress,
}

/// The instruction, address and line usage of one transfer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Frame {
    /// The instruction code, always sent on one line in Standard SPI mode.
    pub instruction: u8,
    /// The 24-bit address, `{00h, CMD[7:0], 00h}`.
    pub address: u32,
    /// The lines carrying the address.
    pub address_lines: Lines,
    /// The lines carrying the data.
    pub data_lines: Lines,
}

impl Frame {
    /// A command write (`02h`): instruction, address and parameters on one line.
    pub const fn command(command: u8) -> Self {
        Self {
            instruction: QspiInstruction::Write.code(),
            address: qspi_address(command),
            address_lines: Lines::Single,
            data_lines: Lines::Single,
        }
    }

    /// A command read (`03h`): instruction and address on one line; the chip answers on
    /// `SIO0` (section 5.2.3).
    pub const fn read(command: u8) -> Self {
        Self {
            instruction: QspiInstruction::Read.code(),
            ..Self::command(command)
        }
    }

    /// A pixel write with a memory write command, `RAMWR` (2Ch) or `RAMWRC` (3Ch).
    pub const fn pixels(command: u8, mode: PixelMode) -> Self {
        let (instruction, address_lines, data_lines) = match mode {
            PixelMode::Single => (QspiInstruction::Write, Lines::Single, Lines::Single),
            PixelMode::Dual => (QspiInstruction::DualPixelWrite, Lines::Single, Lines::Dual),
            PixelMode::DualAddress => (
                QspiInstruction::DualPixelWriteDualAddress,
                Lines::Dual,
                Lines::Dual,
            ),
            PixelMode::Quad => (QspiInstruction::QuadPixelWrite, Lines::Single, Lines::Quad),
            PixelMode::QuadAddress => (
                QspiInstruction::QuadPixelWriteQuadAddress,
                Lines::Quad,
                Lines::Quad,
            ),
        };

        Self {
            instruction: instruction.code(),
            address: qspi_address(command),
            address_lines,
            data_lines,
        }
    }

    /// The instruction and address as four bytes, MSB first. For a [`Lines::Single`]
    /// address, this is exactly what goes on `SIO0` before the data.
    pub const fn header(&self) -> [u8; 4] {
        let [_, a2, a1, a0] = self.address.to_be_bytes();
        [self.instruction, a2, a1, a0]
    }

    /// The command code carried in the address.
    pub const fn command_code(&self) -> u8 {
        (self.address >> 8) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_frame() {
        let frame = Frame::command(0x51);
        assert_eq!(frame.header(), [0x02, 0x00, 0x51, 0x00]);
        assert_eq!(frame.command_code(), 0x51);
        assert_eq!(frame.data_lines, Lines::Single);
        assert_eq!(Frame::read(0xDA).header(), [0x03, 0x00, 0xDA, 0x00]);
    }

    #[test]
    fn pixel_frames() {
        assert_eq!(
            Frame::pixels(0x2C, PixelMode::Single).header(),
            [0x02, 0x00, 0x2C, 0x00]
        );
        let quad = Frame::pixels(0x3C, PixelMode::Quad);
        assert_eq!(quad.header(), [0x32, 0x00, 0x3C, 0x00]);
        assert_eq!(
            (quad.address_lines, quad.data_lines),
            (Lines::Single, Lines::Quad)
        );
        let quad_addr = Frame::pixels(0x2C, PixelMode::QuadAddress);
        assert_eq!(quad_addr.instruction, 0x12);
        assert_eq!(quad_addr.address_lines, Lines::Quad);
        assert_eq!(Frame::pixels(0x2C, PixelMode::Dual).instruction, 0x72);
        let dual_addr = Frame::pixels(0x2C, PixelMode::DualAddress);
        assert_eq!(
            (dual_addr.instruction, dual_addr.address_lines),
            (0x52, Lines::Dual)
        );
    }
}
