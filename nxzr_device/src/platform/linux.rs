use crate::shared::Address;

pub use crate::platform_impl::helper;
pub use crate::platform_impl::syscheck;

// Interop platform specific [crate::platform_impl::sock::Address] with [Address].
impl From<Address> for crate::platform_impl::sock::Address {
    fn from(addr: Address) -> Self {
        addr.0.into()
    }
}

impl From<crate::platform_impl::sock::Address> for Address {
    fn from(addr: crate::platform_impl::sock::Address) -> Self {
        addr.0.into()
    }
}

// Interop [bluer::Address] with [Address].
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
