//! Generic BLE driver targeting mostly Bluetooth Advertisements. Implements the HCI layer.

// For development, allow dead_code
#![warn(clippy::pedantic)]
// Clippy complains about the mass enum matching functions
#![allow(clippy::too_many_lines)]
// #[must_use] doesn't need to be on absolutely everything even though it should.
#![allow(clippy::must_use_candidate)]
#![allow(
    clippy::missing_errors_doc,
    clippy::range_plus_one,
    clippy::type_complexity,
    clippy::doc_markdown
)]
#![deny(unconditional_recursion)]
#![allow(dead_code)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg_attr(not(feature = "std"), macro_use)]
extern crate alloc;
use alloc::boxed::Box;

pub(crate) use futures_util::stream::Stream;
/// Workaround for returning futures from async Traits.
pub type LocalBoxFuture<'a, T> = core::pin::Pin<Box<dyn core::future::Future<Output = T> + 'a>>;
/// Workaround for returning streams from async Traits.
pub type BoxStream<'a, T> = core::pin::Pin<Box<dyn Stream<Item = T> + 'a>>;
extern crate core;
pub mod bytes;
pub mod channel;
#[cfg(feature = "classic")]
pub mod classic;
pub mod error;
#[cfg(feature = "hci")]
pub mod hci;
pub mod le;
pub mod uri;
pub mod uuid;
#[cfg(feature = "winrt_drivers")]
pub mod windows;

use core::convert::{TryFrom, TryInto};

/// Byte Packing/Unpacking error. Usually used for packing/unpacking a struct/type into/from
/// a byte buffer.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum PackError {
    BadOpcode,
    BadLength { expected: usize, got: usize },
    BadBytes { index: Option<usize> },
    InvalidFields,
}
impl PackError {
    /// Ensure `buf.len() == expected`. Returns `Ok(())` if they are equal or
    /// `Err(HCIPackError::BadLength)` not equal.
    #[inline]
    pub fn expect_length(expected: usize, buf: &[u8]) -> Result<(), PackError> {
        if buf.len() == expected {
            Ok(())
        } else {
            Err(PackError::BadLength {
                expected,
                got: buf.len(),
            })
        }
    }
    /// Ensure `buf.len() >= expected`. Returns `Ok(())` if they are or
    /// `Err(HCIPackError::BadLength)` not.
    #[inline]
    pub fn atleast_length(expected: usize, buf: &[u8]) -> Result<(), PackError> {
        if buf.len() == expected {
            Ok(())
        } else {
            Err(PackError::BadLength {
                expected,
                got: buf.len(),
            })
        }
    }
    /// Returns `PackError::BadBytes { index: Some(index) }`.
    #[inline]
    pub fn bad_index(index: usize) -> PackError {
        PackError::BadBytes { index: Some(index) }
    }
}
impl crate::error::Error for PackError {}

/// Basic `ConversionError` for when primitives can't be converted to/from bytes because of invalid
/// states. Most modules use their own errors for when there is more information to report.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ConversionError(pub ());
/// Received Signal Strength Indicator (RSSI). Units: `dBm`. Range -127 dBm to +20 dBm. Defaults to
/// 0 dBm.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Default)]
pub struct RSSI(i8);
impl RSSI {
    pub const MIN_RSSI_I8: i8 = -127;
    pub const MAX_RSSI_I8: i8 = 20;
    pub const MAX_RSSI: RSSI = RSSI(Self::MAX_RSSI_I8);
    pub const MIN_RSSI: RSSI = RSSI(Self::MIN_RSSI_I8);
    /// Creates a new RSSI from `dbm`.
    /// # Panics
    /// Panics if `dbm < MIN_RSSI || dbm > MAX_RSSI`.
    pub fn new(dbm: i8) -> RSSI {
        assert!(
            dbm >= Self::MIN_RSSI_I8 && dbm <= Self::MAX_RSSI_I8,
            "invalid rssi '{}'",
            dbm
        );
        RSSI(dbm)
    }
    pub const UNSUPPORTED_RSSI: i8 = 127;
    pub fn maybe_rssi(val: i8) -> Result<Option<RSSI>, ConversionError> {
        match val {
            -127..=20 => Ok(Some(RSSI(val))),
            127 => Ok(None),
            _ => Err(ConversionError(())),
        }
    }
}
impl From<RSSI> for i8 {
    fn from(rssi: RSSI) -> Self {
        rssi.0
    }
}

impl From<RSSI> for u8 {
    fn from(rssi: RSSI) -> Self {
        rssi.0 as u8
    }
}
impl TryFrom<i8> for RSSI {
    type Error = ConversionError;

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        if value > Self::MAX_RSSI_I8 || value < Self::MIN_RSSI_I8 {
            Err(ConversionError(()))
        } else {
            Ok(RSSI(value))
        }
    }
}
impl TryFrom<u8> for RSSI {
    type Error = ConversionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        (value as i8).try_into()
    }
}
/// Stores milli-dBm.
/// So -100 dBm is = `RSSI(-100_000)`
/// 0 dBm = `RSSI(0)`
/// 10.05 dBm = `RSSI(10_050)`
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MilliDBM(pub i32);
impl MilliDBM {
    pub fn new(milli_dbm: i32) -> MilliDBM {
        MilliDBM(milli_dbm)
    }
}
/// Bluetooth address length (6 bytes)
pub const BT_ADDRESS_LEN: usize = 6;

/// Bluetooth Address. 6 bytes long.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct BTAddress(pub [u8; BT_ADDRESS_LEN]);
impl BTAddress {
    pub const LEN: usize = BT_ADDRESS_LEN;
    pub const ZEROED: BTAddress = BTAddress([0_u8; 6]);
    /// Creates a new 'BTAddress' from a byte slice.
    /// # Panics
    /// Panics if `bytes.len() != BT_ADDRESS_LEN` (6 bytes).
    pub fn new(bytes: &[u8]) -> BTAddress {
        assert_eq!(bytes.len(), BT_ADDRESS_LEN, "address wrong length");
        BTAddress(bytes.try_into().expect("length checked by assert_eq above"))
    }
    /// Uses the 6 lower bytes of `u` in Little Endian
    pub fn from_u64(u: u64) -> Self {
        let bytes = u.to_le_bytes();
        BTAddress([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]])
    }
    pub fn to_u64(self) -> u64 {
        u64::from_le_bytes([
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], 0, 0,
        ])
    }
    pub fn unpack_from(bytes: &[u8]) -> Result<Self, PackError> {
        PackError::expect_length(BT_ADDRESS_LEN, bytes)?;
        Ok(Self::new(bytes))
    }
    pub fn pack_into(self, bytes: &mut [u8]) -> Result<(), PackError> {
        PackError::expect_length(BT_ADDRESS_LEN, bytes)?;
        bytes.copy_from_slice(&self.0[..]);
        Ok(())
    }
    pub fn address_type(self) -> AddressType {
        let address_type_bits = (self.0[BT_ADDRESS_LEN - 1] & 0xC0) >> 6;
        match address_type_bits {
            0b00 => AddressType::NonResolvablePrivate,
            0b01 => AddressType::ResolvablePrivateAddress,
            0b11 => AddressType::StaticDevice,
            // Because of the mask above, _ should only match 0b10 (RFU).
            _ => AddressType::RFU,
        }
    }
    /// Returns `hash` (24-bit) and `prand` (24-bit) of the resolvable private address.
    /// `prand` includes the address type bits.
    pub fn private_address_parts(self) -> Option<(u32, u32)> {
        match self.address_type() {
            AddressType::ResolvablePrivateAddress => Some((
                u32::from_le_bytes([self.0[0], self.0[1], self.0[2], 0]),
                u32::from_le_bytes([self.0[3], self.0[4], self.0[5], 0]),
            )),
            _ => None,
        }
    }
}
impl core::fmt::Display for BTAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}
impl core::str::FromStr for BTAddress {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut out = [0u8; 6];

        let mut nth = 0;
        // Split between ':' and '-'
        // ex: `10-AB-23...` or `10:AB:23...`
        for byte in s.split(|c| c == ':' || c == '-') {
            if nth == 6 {
                return Err(ConversionError(()));
            }

            out[nth] = u8::from_str_radix(byte, 16).map_err(|_| ConversionError(()))?;

            nth += 1;
        }

        if nth != 6 {
            return Err(ConversionError(()));
        }

        Ok(BTAddress(out))
    }
}
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Debug, Hash)]
pub enum AddressType {
    NonResolvablePrivate = 0b00,
    ResolvablePrivateAddress = 0b01,
    RFU = 0b10,
    StaticDevice = 0b11,
}
/// 16-bit Bluetooth Company Identifier. Companies are assigned unique Company Identifiers to
/// Bluetooth SIG members requesting them. [See here for more](https://www.bluetooth.com/specifications/assigned-numbers/company-identifiers/)
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
#[cfg_attr(feature = "serde-1", derive(serde::Serialize, serde::Deserialize))]
pub struct CompanyID(pub u16);
impl CompanyID {
    /// Return the length in bytes of `CompanyID` (2-bytes, 16-bits)
    pub const fn byte_len() -> usize {
        2
    }
}
impl crate::bytes::ToFromBytesEndian for CompanyID {
    type AsBytesType = [u8; 2];

    #[must_use]
    fn to_bytes_le(&self) -> Self::AsBytesType {
        (self.0).to_bytes_le()
    }

    #[must_use]
    fn to_bytes_be(&self) -> Self::AsBytesType {
        (self.0).to_bytes_be()
    }

    #[must_use]
    fn from_bytes_le(bytes: &[u8]) -> Option<Self> {
        Some(CompanyID(u16::from_bytes_le(bytes)?))
    }

    #[must_use]
    fn from_bytes_be(bytes: &[u8]) -> Option<Self> {
        Some(CompanyID(u16::from_bytes_be(bytes)?))
    }
}
