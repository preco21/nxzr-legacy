use macaddr::MacAddr6;
use std::{
    fmt::{self, Debug, Display, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Address(pub [u8; 6]);

impl Address {
    pub const fn new(addr: [u8; 6]) -> Self {
        Self(addr)
    }

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

impl Debug for Address {
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
