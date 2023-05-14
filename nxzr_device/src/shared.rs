use macaddr::MacAddr6;
use std::{
    fmt::{self, Display, Formatter},
    ops::{Deref, DerefMut},
    str::FromStr,
};
use thiserror::Error;
use tokio::process::Command;

pub use uuid::Uuid;

// Bluetooth address.
//
// The serialized representation is a string in colon-hexadecimal notation.
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Address(pub [u8; 6]);

impl Address {
    // Creates a new Bluetooth address with the specified value.
    pub const fn new(addr: [u8; 6]) -> Self {
        Self(addr)
    }

    // Any Bluetooth address.
    //
    // Corresponds to `00:00:00:00:00:00`.
    pub const fn any() -> Self {
        Self([0; 6])
    }
}

impl Deref for Address {
    type Target = [u8; 6];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Address {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl From<MacAddr6> for Address {
    fn from(addr: MacAddr6) -> Self {
        Self(addr.into_array())
    }
}

impl From<Address> for MacAddr6 {
    fn from(addr: Address) -> Self {
        addr.0.into()
    }
}

// Invalid Bluetooth address error.
#[derive(Debug, Clone)]
pub struct InvalidAddressError(pub String);

impl fmt::Display for InvalidAddressError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "invalid Bluetooth address: {}", &self.0)
    }
}

impl std::error::Error for InvalidAddressError {}

impl FromStr for Address {
    type Err = InvalidAddressError;
    fn from_str(s: &str) -> std::result::Result<Self, InvalidAddressError> {
        let fields = s
            .split(':')
            .map(|s| u8::from_str_radix(s, 16).map_err(|_| InvalidAddressError(s.to_string())))
            .collect::<std::result::Result<Vec<_>, InvalidAddressError>>()?;
        Ok(Self(
            fields
                .try_into()
                .map_err(|_| InvalidAddressError(s.to_string()))?,
        ))
    }
}

impl From<[u8; 6]> for Address {
    fn from(addr: [u8; 6]) -> Self {
        Self(addr)
    }
}

impl From<Address> for [u8; 6] {
    fn from(addr: Address) -> Self {
        addr.0
    }
}

/// UUID extension trait to convert to and from Bluetooth short UUIDs.
pub trait UuidExt {
    /// 32-bit short form of Bluetooth UUID.
    fn as_u32(&self) -> Option<u32>;
    /// 16-bit short form of Bluetooth UUID.
    fn as_u16(&self) -> Option<u16>;
    /// Long form of 32-bit short form Bluetooth UUID.
    fn from_u32(v: u32) -> Uuid;
    /// Long form of 16-bit short form Bluetooth UUID.
    fn from_u16(v: u16) -> Uuid;
}

const BASE_UUID: u128 = 0x00000000_0000_1000_8000_00805f9b34fb;
const BASE_MASK_32: u128 = 0x00000000_ffff_ffff_ffff_ffffffffffff;
const BASE_MASK_16: u128 = 0xffff0000_ffff_ffff_ffff_ffffffffffff;

impl UuidExt for Uuid {
    fn as_u32(&self) -> Option<u32> {
        let value = self.as_u128();
        if value & BASE_MASK_32 == BASE_UUID {
            Some((value >> 96) as u32)
        } else {
            None
        }
    }

    fn as_u16(&self) -> Option<u16> {
        let value = self.as_u128();
        if value & BASE_MASK_16 == BASE_UUID {
            Some((value >> 96) as u16)
        } else {
            None
        }
    }

    fn from_u32(v: u32) -> Uuid {
        Uuid::from_u128(BASE_UUID | ((v as u128) << 96))
    }

    fn from_u16(v: u16) -> Uuid {
        Uuid::from_u128(BASE_UUID | ((v as u128) << 96))
    }
}

#[derive(Clone, Error, Debug)]
pub enum SystemCommandError {
    #[error("failed to execute a command: {0}")]
    CommandFailed(String),
    #[error("internal error: {0}")]
    Internal(SystemCommandInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum SystemCommandInternalError {
    #[error("utf8: {0}")]
    Utf8Error(std::str::Utf8Error),
    #[error("io: {kind}; {message}")]
    Io {
        kind: std::io::ErrorKind,
        message: String,
    },
}

impl From<std::str::Utf8Error> for SystemCommandError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Internal(SystemCommandInternalError::Utf8Error(err))
    }
}

impl From<std::io::Error> for SystemCommandError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(SystemCommandInternalError::Io {
            kind: err.kind(),
            message: err.to_string(),
        })
    }
}

pub(crate) async fn run_system_command(mut command: Command) -> Result<(), SystemCommandError> {
    let output = command.output().await?;
    if !output.status.success() {
        return Err(SystemCommandError::CommandFailed(
            std::str::from_utf8(&output.stderr)?.to_owned(),
        ));
    }
    Ok(())
}
