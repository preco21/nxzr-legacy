use crate::platform_impl::device;
use crate::shared::{Address, Uuid};
use std::collections::HashSet;

pub type DeviceError = device::DeviceError;

#[derive(Debug, Default)]
pub struct DeviceConfig {
    // Name of Bluetooth adapter to use.
    //
    // In form of a raw string that matches the adapter name, which is
    // generally represented in the hci* notation. (e.g. hci0, hci1, ...)
    //
    // If None, it will automatically choose the first one by sorting adapter
    // names in lexicographic order.
    pub id: Option<String>,
}

#[derive(Debug)]
pub struct Device {
    inner: device::Device,
}

impl Device {
    #[tracing::instrument(target = "device")]
    pub async fn new(config: DeviceConfig) -> Result<Self, DeviceError> {
        let inner = device::Device::new(config).await?;
        Ok(Self { inner })
    }

    #[tracing::instrument(target = "device")]
    pub async fn ensure_adapter_address_switch(self) -> Result<Self, DeviceError> {
        let new_dev = self.inner.ensure_adapter_address_switch().await?;
        Ok(Self { inner: new_dev })
    }

    #[tracing::instrument(target = "device")]
    pub async fn check_paired_devices(&self, disconnect: bool) -> Result<(), DeviceError> {
        self.inner.check_paired_devices(disconnect).await
    }

    pub fn adapter_name(&self) -> &str {
        self.inner.adapter_name()
    }

    pub async fn address(&self) -> Result<Address, DeviceError> {
        self.inner.address().await.map(|addr| addr.into())
    }

    // FIXME: currently not in use, may be used in future, find out to employ this one
    // pub async fn paired_devices(&self) -> Result<Vec<bluer::Device>, DeviceError> {
    //     self.inner.paired_devices().await
    // }

    pub async fn register_sdp_record(&self) -> Result<ProfileHandle, DeviceError> {
        let inner_handle = self.inner.register_sdp_record().await?;
        Ok(ProfileHandle {
            _inner: inner_handle,
        })
    }

    pub async fn ensure_device_class(&self) -> Result<(), DeviceError> {
        self.inner.ensure_device_class().await
    }

    pub async fn unpair_device(&self, address: Address) -> Result<(), DeviceError> {
        self.inner.unpair_device(address.into()).await
    }

    pub async fn set_powered(&self, flag: bool) -> Result<(), DeviceError> {
        self.inner.set_powered(flag).await
    }

    pub async fn set_pairable(&self, flag: bool) -> Result<(), DeviceError> {
        self.inner.set_pairable(flag).await
    }

    pub async fn set_discoverable(&self, flag: bool) -> Result<(), DeviceError> {
        self.inner.set_discoverable(flag).await
    }

    pub async fn set_alias(&self, name: String) -> Result<(), DeviceError> {
        self.inner.set_alias(name).await
    }

    pub async fn uuids(&self) -> Result<Option<HashSet<Uuid>>, DeviceError> {
        self.inner.uuids().await
    }
}

#[derive(Debug)]
pub struct ProfileHandle {
    _inner: device::ProfileHandle,
}

impl Drop for ProfileHandle {
    fn drop(&mut self) {
        // Required for drop order
    }
}
