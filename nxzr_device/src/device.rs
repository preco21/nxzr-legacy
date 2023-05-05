use crate::helper::{self, HelperError};
use std::{collections::HashSet, str::FromStr};
use thiserror::Error;

// Gamepad/Joystick device class
const DEVICE_CLASS: u32 = 0x002508;

const SWITCH_DEVICE_NAME: &str = "Nintendo Switch";
const SWITCH_MAC_PREFIX: &[u8] = &[0x94, 0x59, 0xCB];
const SWITCH_SDP_RECORD_STRING: &'static str = include_str!("sdp/switch-controller.xml");
const SWITCH_HID_UUID: &str = "00001124-0000-1000-8000-00805f9b34fb";

#[derive(Clone, Error, Debug)]
pub enum DeviceError {
    #[error("failed to change MAC address")]
    MacAddrChangeFailed,
    #[error("failed to set device class")]
    DeviceClassSettingFailed,
    #[error("no such adapter name: {0}")]
    NoSuchAdapterExists(String),
    #[error("helper error: {0}")]
    Helper(HelperError),
    #[error("internal error: {0}")]
    Internal(DeviceInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum DeviceInternalError {
    #[error("io: {0}")]
    Io(std::io::ErrorKind),
    #[error("bluer: {0}")]
    Bluer(bluer::ErrorKind),
    #[error("uuid: {0}")]
    Uuid(uuid::Error),
}

impl From<HelperError> for DeviceError {
    fn from(err: HelperError) -> Self {
        Self::Helper(err)
    }
}

impl From<std::io::Error> for DeviceError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(DeviceInternalError::Io(err.kind()))
    }
}

impl From<bluer::Error> for DeviceError {
    fn from(err: bluer::Error) -> Self {
        Self::Internal(DeviceInternalError::Bluer(err.kind))
    }
}

impl From<uuid::Error> for DeviceError {
    fn from(err: uuid::Error) -> Self {
        Self::Internal(DeviceInternalError::Uuid(err))
    }
}

#[derive(Debug, Default)]
pub struct DeviceConfig {
    // Name of the bluetooth adapter to use.
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
    adapter: bluer::Adapter,
    session: bluer::Session,
}

impl Device {
    #[tracing::instrument(target = "device")]
    pub async fn new(config: DeviceConfig) -> Result<Self, DeviceError> {
        let session = bluer::Session::new().await?;
        let adapter = match config.id {
            Some(adapter_name) => {
                let mut found_adapter = None;
                for name in session.adapter_names().await? {
                    if name == adapter_name {
                        found_adapter = Some(session.adapter(&adapter_name)?);
                        break;
                    }
                }
                match found_adapter {
                    Some(adapter) => adapter,
                    None => return Err(DeviceError::NoSuchAdapterExists(adapter_name)),
                }
            }
            None => session.default_adapter().await?,
        };
        Ok(Self { adapter, session })
    }

    #[tracing::instrument(target = "device")]
    pub async fn ensure_adapter_address_switch(self) -> Result<Self, DeviceError> {
        tracing::info!(
            "attempting to change MAC address of the bluetooth adapter to Switch compatible one"
        );
        let addr = self.address().await?;
        if &addr.as_ref()[..3] != SWITCH_MAC_PREFIX {
            let adapter_name = self.adapter_name().to_owned();
            let mut addr_bytes: [u8; 6] = [0x00; 6];
            addr_bytes[..3].copy_from_slice(SWITCH_MAC_PREFIX);
            addr_bytes[3..].copy_from_slice(&addr.as_ref()[3..]);
            let new_addr = bluer::Address::new(addr_bytes);
            helper::set_adapter_address(adapter_name.as_str(), new_addr).await?;
            // We need to re-instantiate device.
            drop(self);
            let new_self = Self::new(DeviceConfig {
                id: Some(adapter_name.to_owned()),
            })
            .await?;
            if new_self.address().await? != new_addr {
                tracing::error!("failed to change MAC address of the bluetooth adapter");
                return Err(DeviceError::MacAddrChangeFailed);
            }
            tracing::info!(
                "successfully changed MAC address of the bluetooth adapter to {}",
                new_addr
            );
            return Ok(new_self);
        }
        Ok(self)
    }

    #[tracing::instrument(target = "device")]
    pub async fn check_paired_devices(&self, disconnect: bool) -> Result<(), DeviceError> {
        let Some(uuids) = self.uuids().await? else {
            return Ok(())
        };
        if uuids.len() > 3 {
            tracing::warn!("there's too many SDP-records active, Switch might refuse connection.");
            tracing::warn!(
                "try to disable `input` plugin by modifying /lib/systemd/system/bluetooth.service"
            );
            if disconnect {
                for dev in self.paired_devices().await? {
                    tracing::info!("unpairing device of address: {}", dev.address());
                    self.unpair_device(dev.address()).await?;
                }
            } else {
                let paired_addresses: Vec<String> = self
                    .paired_devices()
                    .await?
                    .iter()
                    .map(|d| d.address().to_string())
                    .collect();
                tracing::warn!(
                    "attempting initial pairing, but there are already paired devices: {}",
                    paired_addresses.join(", ")
                );
            }
        }
        Ok(())
    }

    pub fn adapter_name(&self) -> &str {
        self.adapter.name()
    }

    pub async fn address(&self) -> Result<bluer::Address, DeviceError> {
        let addr = self.adapter.address().await?;
        Ok(addr)
    }

    pub async fn paired_devices(&self) -> Result<Vec<bluer::Device>, DeviceError> {
        let mut devices = vec![];
        for addr in self.adapter.device_addresses().await? {
            let dev = self.adapter.device(addr)?;
            if let Some(name) = dev.name().await? {
                if name == SWITCH_DEVICE_NAME {
                    devices.push(dev);
                }
            }
        }
        Ok(devices)
    }

    pub async fn register_sdp_record(&self) -> Result<(), DeviceError> {
        self.session
            .register_profile(bluer::rfcomm::Profile {
                uuid: uuid::Uuid::new_v4(),
                service: Some(uuid::Uuid::from_str(SWITCH_HID_UUID)?),
                role: Some(bluer::rfcomm::Role::Server),
                require_authentication: Some(false),
                require_authorization: Some(false),
                service_record: Some(SWITCH_SDP_RECORD_STRING.to_owned()),
                ..Default::default()
            })
            .await?;
        Ok(())
    }

    pub async fn set_class(&self) -> Result<(), DeviceError> {
        // If current adapter's device class is same as expected, do nothing.
        if self.adapter.class().await? == DEVICE_CLASS {
            return Ok(());
        }
        let class = helper::set_device_class(self.adapter_name(), DEVICE_CLASS).await?;
        if self.adapter.class().await? != class {
            return Err(DeviceError::DeviceClassSettingFailed);
        }
        Ok(())
    }

    pub async fn unpair_device(&self, address: bluer::Address) -> Result<(), DeviceError> {
        self.adapter.remove_device(address).await?;
        Ok(())
    }

    pub async fn set_powered(&self, flag: bool) -> Result<(), DeviceError> {
        self.adapter.set_powered(flag).await?;
        Ok(())
    }

    pub async fn set_pairable(&self, flag: bool) -> Result<(), DeviceError> {
        self.adapter.set_pairable(flag).await?;
        Ok(())
    }

    pub async fn set_discoverable(&self, flag: bool) -> Result<(), DeviceError> {
        self.adapter.set_discoverable(flag).await?;
        Ok(())
    }

    pub async fn set_alias(&self, name: String) -> Result<(), DeviceError> {
        self.adapter.set_alias(name).await?;
        Ok(())
    }

    pub async fn uuids(&self) -> Result<Option<HashSet<bluer::Uuid>>, DeviceError> {
        let uuids = self.adapter.uuids().await?;
        Ok(uuids)
    }
}
