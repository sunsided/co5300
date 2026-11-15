//! # Chipone CO5300 AMOLED Display Controller Commands
//!
//! This crate provides a typed map of the CO5300's command set, following the CO5300
//! datasheet V0.01 (Chipone, 24 July 2023).
//!
//! The CO5300 is controlled with 8-bit command codes ("Display Command Set", DCS, plus a few
//! manufacturer commands), each followed by zero or more parameter bytes. The same command
//! codes are used on every host interface (MIPI-DSI, Quad-SPI, SPI); only the framing differs.
//!
//! - [`Command`] lists the user command set (`CMD1`, datasheet section 7.4).
//! - Commands with one parameter byte are modelled as registers ([`Madctl`], [`Colmod`],
//!   [`Brightness`], ...), built on [`hardware-registers`](hardware_registers)' SPI register
//!   traits with the command code as register address and the parameter byte as value.
//! - [`AddressRange`] encodes the four parameter bytes of `CASET` (2Ah) and `RASET` (2Bh).
//! - [`QspiInstruction`] lists the instruction codes of the Quad-SPI framing (section 5.2).

#![deny(missing_docs)]
#![no_std]
#![forbid(unsafe_code)]
// bitfield-struct generates `with_*_checked` setters that return `Result<_, ()>`.
#![allow(clippy::result_unit_err)]
// Enables the `doc_cfg` feature when the `docsrs` configuration attribute is defined.
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Exports commonly used traits.
pub mod prelude {
    pub use crate::{ReadableRegister, Register, WritableRegister};
    pub use hardware_registers::register_address::RegisterAddress8;
    pub use hardware_registers::sizes::R1;
    pub use hardware_registers::spi::*;
    pub use hardware_registers::{FromBits, HardwareRegister, ToBits, WritableHardwareRegister};
}

/// Implements the command code and size information shared by all registers.
macro_rules! register {
    ($type:ident, $cmd:expr) => {
        impl $crate::Register for $type {}
        impl $crate::prelude::HardwareRegister<$crate::prelude::R1> for $type {}

        impl $crate::prelude::SPIRegister<$crate::prelude::RegisterAddress8, $crate::prelude::R1>
            for $type
        {
            type Backing = u8;

            const REGISTER_ADDRESS: $crate::prelude::RegisterAddress8 =
                $crate::prelude::RegisterAddress8::new(($cmd).code());
        }
    };
}

/// Implements decoding, which makes a register readable.
macro_rules! read_access {
    ($type:ident) => {
        impl $crate::prelude::FromBits<u8> for $type {
            #[inline]
            fn from_bits(value: u8) -> Self {
                value.into()
            }

            #[inline]
            fn from_bits_ref(value: &u8) -> Self {
                (*value).into()
            }
        }

        impl $crate::ReadableRegister for $type {}
    };
}

/// Implements encoding, which makes a register writable.
macro_rules! write_access {
    ($type:ident) => {
        impl $crate::prelude::ToBits for $type {
            type Target = u8;

            #[inline]
            fn to_bits(&self) -> Self::Target {
                (*self).into()
            }
        }

        impl $crate::prelude::WritableHardwareRegister<$crate::prelude::R1> for $type {}
        impl $crate::WritableRegister for $type {}
    };
}

macro_rules! readable_register {
    ($type:ident, $cmd:expr) => {
        register!($type, $cmd);
        read_access!($type);
    };
}

macro_rules! writable_register {
    ($type:ident, $cmd:expr) => {
        register!($type, $cmd);
        read_access!($type);
        write_access!($type);
    };
}

mod commands;
mod registers;
mod types;

pub use commands::*;
pub use registers::*;
pub use types::*;

/// A command whose single parameter byte is modelled as a register.
///
/// The register address is the command code; the value is the parameter byte.
pub trait Register: prelude::SPIRegister8 {
    /// The command code that carries this register.
    const COMMAND: u8 =
        <Self as prelude::SPIRegister<prelude::RegisterAddress8, prelude::R1>>::REGISTER_ADDRESS
            .into_inner();
}

/// A register that can be decoded from its parameter byte.
pub trait ReadableRegister: Register + prelude::FromBits<u8> {}

/// A register that can be written with its command.
pub trait WritableRegister: Register + prelude::WritableSPIRegister8 {}
