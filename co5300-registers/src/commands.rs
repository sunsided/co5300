//! Command codes, the column/row address range and the Quad-SPI instructions.

/// Declares the command enum with its documentation, code and `from_code` lookup.
macro_rules! commands {
    ($($(#[$meta:meta])* $name:ident = $code:literal,)+) => {
        /// The user command set (`CMD1`), datasheet section 7.4 "Command List".
        ///
        /// Commands without a typed parameter in this crate take raw parameter bytes as
        /// described in datasheet section 7.5.
        #[allow(non_camel_case_types)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        #[repr(u8)]
        pub enum Command {
            $($(#[$meta])* $name = $code,)+
        }

        impl Command {
            /// Converts a command code into a command, if this crate lists it.
            pub const fn from_code(code: u8) -> Option<Self> {
                match code {
                    $($code => Some(Self::$name),)+
                    _ => None,
                }
            }
        }
    };
}

commands! {
    /// `NOP` (00h): no operation.
    NOP = 0x00,
    /// `SWRESET` (01h): software reset. Wait 5 ms before the next command; in Sleep Out mode,
    /// wait 120 ms before `SLPOUT`.
    SWRESET = 0x01,
    /// `RDDID` (04h): read display ID, three bytes `ID1`, `ID2`, `ID3`.
    RDDID = 0x04,
    /// `RDNUMED` (05h): read the number of errors on DSI.
    RDNUMED = 0x05,
    /// `RDDPM` (0Ah): read display power mode, see
    /// [`DisplayPowerMode`](crate::DisplayPowerMode).
    RDDPM = 0x0A,
    /// `RDDMADCTR` (0Bh): read display MADCTL, see [`Madctl`](crate::Madctl).
    RDDMADCTR = 0x0B,
    /// `RDDCOLMOD` (0Ch): read display pixel format, see [`Colmod`](crate::Colmod).
    RDDCOLMOD = 0x0C,
    /// `RDDIM` (0Dh): read display image mode.
    RDDIM = 0x0D,
    /// `RDDSM` (0Eh): read display signal mode.
    RDDSM = 0x0E,
    /// `RDDSDR` (0Fh): read display self-diagnostic result.
    RDDSDR = 0x0F,
    /// `SLPIN` (10h): enter Sleep In mode. Wait 5 ms before the next command.
    SLPIN = 0x10,
    /// `SLPOUT` (11h): leave Sleep In mode. Wait 5 ms before the next command.
    SLPOUT = 0x11,
    /// `PTLON` (12h): Partial Display mode on.
    PTLON = 0x12,
    /// `NORON` (13h): Normal Display mode on.
    NORON = 0x13,
    /// `INVOFF` (20h): display inversion off.
    INVOFF = 0x20,
    /// `INVON` (21h): display inversion on.
    INVON = 0x21,
    /// `ALLPOFF` (22h): all pixels off (black), frame memory unchanged.
    ALLPOFF = 0x22,
    /// `ALLPON` (23h): all pixels on (white), frame memory unchanged.
    ALLPON = 0x23,
    /// `DISPOFF` (28h): display off; frame memory output disabled, blank page inserted.
    DISPOFF = 0x28,
    /// `DISPON` (29h): display on.
    DISPON = 0x29,
    /// `CASET` (2Ah): column address range, see [`AddressRange`].
    CASET = 0x2A,
    /// `RASET` (2Bh): row address range, see [`AddressRange`].
    RASET = 0x2B,
    /// `RAMWR` (2Ch): memory write, starting at the window origin.
    RAMWR = 0x2C,
    /// `RAMRD` (2Eh): memory read, starting at the window origin.
    RAMRD = 0x2E,
    /// `PTLAR` (30h): partial area rows, four bytes like [`AddressRange`].
    PTLAR = 0x30,
    /// `PTLAR_H` (31h): partial area columns, four bytes like [`AddressRange`].
    PTLAR_H = 0x31,
    /// `TEOFF` (34h): tearing effect output off.
    TEOFF = 0x34,
    /// `TEON` (35h): tearing effect output on, see
    /// [`TearingEffectOn`](crate::TearingEffectOn).
    TEON = 0x35,
    /// `MADCTL` (36h): memory data access control, see [`Madctl`](crate::Madctl).
    MADCTL = 0x36,
    /// `IDMOFF` (38h): Idle mode off.
    IDMOFF = 0x38,
    /// `IDMON` (39h): Idle mode on; colors reduced to the MSB of R, G and B.
    IDMON = 0x39,
    /// `COLMOD` (3Ah): interface pixel format, see [`Colmod`](crate::Colmod).
    COLMOD = 0x3A,
    /// `RAMWRC` (3Ch): memory write, continuing after the last written pixel.
    RAMWRC = 0x3C,
    /// `RAMRDC` (3Eh): memory read, continuing after the last read pixel.
    RAMRDC = 0x3E,
    /// `STESL` (44h): tearing effect scan line `N[15:0]`, two bytes, MSB first.
    STESL = 0x44,
    /// `GSL` (45h): get scan line, two bytes.
    GSL = 0x45,
    /// `DSTBON` (4Fh): deep standby, see [`DeepStandby`](crate::DeepStandby).
    DSTBON = 0x4F,
    /// `WRDISBV` (51h): write display brightness, see [`Brightness`](crate::Brightness).
    WRDISBV = 0x51,
    /// `RDDISBV` (52h): read display brightness.
    RDDISBV = 0x52,
    /// `WRCTRLD` (53h): write CTRL display, see [`CtrlDisplay`](crate::CtrlDisplay).
    WRCTRLD = 0x53,
    /// `RDCTRLD` (54h): read CTRL display.
    RDCTRLD = 0x54,
    /// `WRACL` (55h): write ACL control, see [`AclControl`](crate::AclControl).
    WRACL = 0x55,
    /// `RDACL` (56h): read ACL control.
    RDACL = 0x56,
    /// `WRIMGEHCCTR` (58h): sunlight readable enhancement, see
    /// [`ColorEnhance`](crate::ColorEnhance).
    WRIMGEHCCTR = 0x58,
    /// `RDIMGEHCCTR` (59h): read color enhancement control.
    RDIMGEHCCTR = 0x59,
    /// `WRHBMDISBV` (63h): write HBM display brightness, see
    /// [`HbmBrightness`](crate::HbmBrightness).
    WRHBMDISBV = 0x63,
    /// `RDHBMDISBV` (65h): read HBM display brightness.
    RDHBMDISBV = 0x65,
    /// `HBMCTL` (66h): HBM mode enable, see [`HbmControl`](crate::HbmControl).
    HBMCTL = 0x66,
    /// `COLOPT` (80h): interface pixel format option.
    COLOPT = 0x80,
    /// `RDDDBS` (A1h): read DDB start.
    RDDDBS = 0xA1,
    /// `RDDDBC` (A8h): read DDB continuous.
    RDDDBC = 0xA8,
    /// `RDFCS` (AAh): read first checksum.
    RDFCS = 0xAA,
    /// `RDCCS` (AFh): read continuous checksum.
    RDCCS = 0xAF,
    /// `SetDISPMode` (C2h): display mode (RAM path and timing mode).
    SETDISPMODE = 0xC2,
    /// `SetSPIMode` (C4h): SPI mode, see [`SpiMode`](crate::SpiMode).
    SETSPIMODE = 0xC4,
    /// `RDID1` (DAh): read ID1.
    RDID1 = 0xDA,
    /// `RDID2` (DBh): read ID2.
    RDID2 = 0xDB,
    /// `RDID3` (DCh): read ID3.
    RDID3 = 0xDC,
    /// `CMD Page Switch` (FEh): select the command page, see
    /// [`CommandPage`](crate::CommandPage).
    PAGESEL = 0xFE,
    /// `CMD Page Status` (FFh): read the current command page.
    PAGESTATUS = 0xFF,
}

impl Command {
    /// Returns the command code.
    pub const fn code(&self) -> u8 {
        *self as u8
    }
}

impl From<Command> for u8 {
    fn from(value: Command) -> Self {
        value.code()
    }
}

/// The highest column or row address `CASET` and `RASET` accept (10 bits).
pub const MAX_ADDRESS: u16 = 0x3FF;

/// An inclusive column or row range, the parameters of `CASET` (2Ah) and `RASET` (2Bh).
///
/// Datasheet sections 7.5.21 and 7.5.22: the start must not exceed the end, and both the
/// start and the length (`end - start + 1`) must be divisible by 2. The defaults after reset
/// are `0..=0x1C5`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AddressRange {
    /// The first column or row (`SC`/`SP`).
    pub start: u16,
    /// The last column or row, inclusive (`EC`/`EP`).
    pub end: u16,
}

impl AddressRange {
    /// The range after power-on or reset: `0..=0x1C5`.
    pub const DEFAULT: Self = Self::new(0, 0x1C5);

    /// Creates a range from `start` to `end`, inclusive.
    pub const fn new(start: u16, end: u16) -> Self {
        Self { start, end }
    }

    /// Whether the range satisfies the datasheet's restrictions: `start <= end <= 0x3FF`,
    /// `start` even and the length even.
    pub const fn is_valid(&self) -> bool {
        self.start <= self.end
            && self.end <= MAX_ADDRESS
            && self.start % 2 == 0
            && (self.end - self.start + 1) % 2 == 0
    }

    /// The number of columns or rows in the range.
    pub const fn len(&self) -> u16 {
        self.end.saturating_sub(self.start) + 1
    }

    /// Always `false`: a range holds at least one address.
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Encodes the range as the four parameter bytes `SC[9:8]`, `SC[7:0]`, `EC[9:8]`,
    /// `EC[7:0]`.
    pub const fn to_params(&self) -> [u8; 4] {
        let [sh, sl] = (self.start & MAX_ADDRESS).to_be_bytes();
        let [eh, el] = (self.end & MAX_ADDRESS).to_be_bytes();
        [sh, sl, eh, el]
    }

    /// Decodes the four parameter bytes; bits above bit 9 are ignored.
    pub const fn from_params(params: [u8; 4]) -> Self {
        Self {
            start: u16::from_be_bytes([params[0], params[1]]) & MAX_ADDRESS,
            end: u16::from_be_bytes([params[2], params[3]]) & MAX_ADDRESS,
        }
    }
}

impl Default for AddressRange {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Instruction codes of the Quad-SPI interface, datasheet section 5.2.1.
///
/// The instruction is the first byte of every transfer. In Standard SPI mode it is sent on
/// one line, followed by a 24-bit address `{00h, CMD[7:0], 00h}` and the parameter or pixel
/// bytes (section 5.2.2).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum QspiInstruction {
    /// Command or pixel write; address and data on one line.
    Write = 0x02,
    /// Command or pixel read; address on one line, data out on one line.
    Read = 0x03,
    /// Pixel write on four lines, address on one line (24 clocks).
    QuadPixelWrite = 0x32,
    /// Pixel write on four lines, address on four lines (6 clocks).
    QuadPixelWriteQuadAddress = 0x12,
    /// Pixel write on two lines, address on one line (24 clocks).
    DualPixelWrite = 0x72,
    /// Pixel write on two lines, address on two lines (12 clocks).
    DualPixelWriteDualAddress = 0x52,
    /// Enter Quad SPI (QPI) mode: later instructions use four lines.
    EnterQuad = 0x38,
    /// Enter Dual SPI mode: later instructions use two lines.
    EnterDual = 0x3B,
    /// Return from Dual or Quad SPI mode to Standard SPI. Sent as `FFh` on one line, it also
    /// resets the interface when host and chip disagree about the mode (section 5.2.4).
    ResetToSingle = 0xFF,
}

impl QspiInstruction {
    /// Returns the instruction code.
    pub const fn code(&self) -> u8 {
        *self as u8
    }

    /// Converts an instruction code into an instruction, if it names one.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0x02 => Some(Self::Write),
            0x03 => Some(Self::Read),
            0x32 => Some(Self::QuadPixelWrite),
            0x12 => Some(Self::QuadPixelWriteQuadAddress),
            0x72 => Some(Self::DualPixelWrite),
            0x52 => Some(Self::DualPixelWriteDualAddress),
            0x38 => Some(Self::EnterQuad),
            0x3B => Some(Self::EnterDual),
            0xFF => Some(Self::ResetToSingle),
            _ => None,
        }
    }
}

/// The 24-bit Quad-SPI address that carries a command code: `{00h, CMD[7:0], 00h}`
/// (datasheet section 5.2.2).
pub const fn qspi_address(command: u8) -> u32 {
    (command as u32) << 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_round_trip() {
        for code in 0..=u8::MAX {
            if let Some(cmd) = Command::from_code(code) {
                assert_eq!(cmd.code(), code);
            }
        }
        assert_eq!(Command::from_code(0x2C), Some(Command::RAMWR));
        assert_eq!(Command::from_code(0x01), Some(Command::SWRESET));
        assert_eq!(Command::from_code(0x02), None);
    }

    #[test]
    fn address_range_params() {
        let range = AddressRange::new(6, 0x1DD);
        assert_eq!(range.to_params(), [0x00, 0x06, 0x01, 0xDD]);
        assert_eq!(AddressRange::from_params(range.to_params()), range);
        assert_eq!(AddressRange::DEFAULT.to_params(), [0x00, 0x00, 0x01, 0xC5]);
    }

    #[test]
    fn address_range_validity() {
        assert!(AddressRange::new(0, 1).is_valid());
        assert!(AddressRange::new(2, 409).is_valid());
        assert!(!AddressRange::new(1, 2).is_valid(), "odd start");
        assert!(!AddressRange::new(0, 2).is_valid(), "odd length");
        assert!(!AddressRange::new(4, 1).is_valid(), "start after end");
        assert!(
            !AddressRange::new(0, 0x400 - 1 + 2).is_valid(),
            "beyond 10 bits"
        );
        assert_eq!(AddressRange::new(2, 409).len(), 408);
    }

    #[test]
    fn qspi_codes() {
        assert_eq!(qspi_address(0x2C), 0x00_2C_00);
        assert_eq!(qspi_address(0x3C), 0x00_3C_00);
        for code in 0..=u8::MAX {
            if let Some(i) = QspiInstruction::from_code(code) {
                assert_eq!(i.code(), code);
            }
        }
    }
}
