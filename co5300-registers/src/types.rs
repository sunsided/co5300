//! Value types used in register fields.

/// Declares a field enum with `into_bits`/`from_bits` for bitfield-struct.
macro_rules! field_enum {
    (
        $(#[$meta:meta])* $name:ident, $mask:literal,
        $($(#[$vmeta:meta])* $variant:ident = $value:literal,)+
    ) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        #[repr(u8)]
        pub enum $name {
            $($(#[$vmeta])* $variant = $value,)+
        }

        impl $name {
            /// Converts the value into its field representation.
            pub const fn into_bits(self) -> u8 {
                self as u8
            }

            /// Converts the low bits of a field into a value.
            pub const fn from_bits(value: u8) -> Self {
                match value & $mask {
                    $($value => Self::$variant,)+
                    // All bit patterns are listed above.
                    _ => unreachable!(),
                }
            }
        }
    };
}

field_enum!(
    /// A pixel format in [`Colmod`](crate::Colmod), datasheet section 7.5.32.
    PixelFormat, 0b111,
    /// Not documented.
    Reserved0 = 0b000,
    /// 8 bits per pixel, 256 gray levels (SPI only).
    Gray256 = 0b001,
    /// 8 bits per pixel, RGB 3-3-2 (SPI only).
    Rgb332 = 0b010,
    /// 3 bits per pixel, RGB 1-1-1 (SPI only).
    Rgb111 = 0b011,
    /// Not documented.
    Reserved4 = 0b100,
    /// 16 bits per pixel, RGB 5-6-5 (65,536 colors).
    Rgb565 = 0b101,
    /// 18 bits per pixel, RGB 6-6-6 (262,144 colors), sent as three bytes.
    Rgb666 = 0b110,
    /// 24 bits per pixel, RGB 8-8-8 (16.7M colors). The reset default.
    Rgb888 = 0b111,
);

impl PixelFormat {
    /// The number of bytes per pixel on the interface for the 16, 18 and 24-bit formats
    /// (datasheet section 5.2.2), or `None` for the packed formats.
    pub const fn bytes_per_pixel(self) -> Option<usize> {
        match self {
            Self::Rgb565 => Some(2),
            Self::Rgb666 | Self::Rgb888 => Some(3),
            _ => None,
        }
    }
}

field_enum!(
    /// The ACL (auto current limit) setting in [`AclControl`](crate::AclControl), datasheet
    /// section 7.5.42.
    AclMode, 0b11,
    /// ACL disabled. The reset default.
    Off = 0b00,
    /// Not documented.
    Reserved1 = 0b01,
    /// Not documented.
    Reserved2 = 0b10,
    /// ACL enabled.
    On = 0b11,
);

field_enum!(
    /// The sunlight readable enhancement level in [`ColorEnhance`](crate::ColorEnhance),
    /// datasheet section 7.5.44.
    EnhancementLevel, 0b11,
    /// Low. The reset default.
    Low = 0,
    /// Medium.
    Medium = 1,
    /// High.
    High = 2,
    /// Not documented.
    Reserved3 = 3,
);

field_enum!(
    /// A command page in [`CommandPage`](crate::CommandPage), datasheet section 7.5.61.
    ///
    /// The manufacturer pages (`CMD2`) hold panel settings (power, gamma, GOA timing). They
    /// are locked after reset; see the datasheet's command table switch flow (section 7.1).
    Page, 0b1111,
    /// `CMD1`, the user command set. The default after power-on and reset.
    User = 0x0,
    /// `CMD2` page 0, panel ID.
    PanelId = 0x1,
    /// `CMD2` page 0, extension (display control).
    Extension = 0x2,
    /// `CMD2` gamma 3.
    Gamma3 = 0x3,
    /// `CMD2` page 0 (power control).
    Power = 0x4,
    /// `CMD2` gamma 1.
    Gamma1 = 0x5,
    /// `CMD2` gamma 2.
    Gamma2 = 0x6,
    /// `CMD2` GOA timing in normal mode.
    GoaTiming = 0x7,
    /// `CMD2` brightness control and ACL (OLED IP).
    BrightnessControl = 0x8,
    /// Not documented.
    Reserved9 = 0x9,
    /// Not documented.
    Reserved10 = 0xA,
    /// Not documented.
    Reserved11 = 0xB,
    /// Not documented.
    Reserved12 = 0xC,
    /// Not documented.
    Reserved13 = 0xD,
    /// Not documented.
    Reserved14 = 0xE,
    /// Not documented.
    Reserved15 = 0xF,
);
