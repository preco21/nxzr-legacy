use macaddr::MacAddr6;
use std::{
    fmt::{self, Display, Formatter},
    ops::{Deref, DerefMut},
    str::FromStr,
};

/// Bluetooth address.
///
/// The serialized representation is a string in colon-hexadecimal notation.
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Address(pub [u8; 6]);

impl Address {
    /// Creates a new Bluetooth address with the specified value.
    pub const fn new(addr: [u8; 6]) -> Self {
        Self(addr)
    }

    /// Any Bluetooth address.
    ///
    /// Corresponds to `00:00:00:00:00:00`.
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

/// Interop [bluer::Address] with [crate::Address].
impl From<bluer::Address> for Address {
    fn from(addr: bluer::Address) -> Self {
        addr.0.into()
    }
}

impl From<Address> for bluer::Address {
    fn from(addr: Address) -> Self {
        addr.0.into()
    }
}

impl From<bluer::Address> for nxzr_shared::Address {
    fn from(addr: bluer::Address) -> Self {
        addr.0.into()
    }
}

/// Interop [nxzr_core::Address] with [crate::Address].
impl From<nxzr_core::Address> for Address {
    fn from(addr: nxzr_core::Address) -> Self {
        addr.0.into()
    }
}

impl From<Address> for nxzr_core::Address {
    fn from(addr: Address) -> Self {
        addr.0.into()
    }
}

/// Invalid Bluetooth address error.
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
