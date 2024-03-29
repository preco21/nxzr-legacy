use crate::{system, Address, Uuid};
use std::{collections::HashSet, future::Future, str::FromStr};
use tokio::sync::mpsc;

// Gamepad/Joystick device class
const DEVICE_CLASS: u32 = 0x002508;

const SWITCH_DEVICE_NAME: &str = "Nintendo Switch";
const SWITCH_MAC_PREFIX: &[u8] = &[0x94, 0x59, 0xCB];
const SWITCH_SDP_RECORD_STRING: &str = include_str!("sdp/switch-controller.xml");
const SWITCH_HID_UUID: &str = "00001124-0000-1000-8000-00805f9b34fb";

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("failed to create a session, maybe Bluetooth is disabled?; bluer: {0}")]
    SessionCreationFailed(bluer::Error),
    #[error("failed to change MAC address")]
    MacAddrChangeFailed,
    #[error("failed to set device class")]
    DeviceClassSettingFailed,
    #[error("no such adapter name: {0}")]
    NoSuchAdapterExists(String),
    #[error("internal error: {0}")]
    Internal(DeviceInternalError),
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceInternalError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("bluer: {} {}", .0.kind, .0.message)]
    Bluer(bluer::Error),
    #[error("uuid: {0}")]
    Uuid(uuid::Error),
    #[error("system command: {0}")]
    SystemCommand(system::SystemCommandError),
}

impl From<std::io::Error> for DeviceError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.into())
    }
}

impl From<bluer::Error> for DeviceError {
    fn from(err: bluer::Error) -> Self {
        Self::Internal(DeviceInternalError::Bluer(err))
    }
}

impl From<uuid::Error> for DeviceError {
    fn from(err: uuid::Error) -> Self {
        Self::Internal(DeviceInternalError::Uuid(err))
    }
}

impl From<system::SystemCommandError> for DeviceError {
    fn from(err: system::SystemCommandError) -> Self {
        Self::Internal(DeviceInternalError::SystemCommand(err))
    }
}

#[derive(Debug, Default)]
pub struct DeviceConfig {
    /// Name of Bluetooth adapter to use.
    ///
    /// In form of a raw string that matches the adapter name, which is
    /// generally represented in the hci* notation. (e.g. hci0, hci1, ...)
    ///
    /// If None, it will automatically choose the first one by sorting adapter
    /// names in lexicographic order.
    pub dev_id: Option<String>,
}

#[derive(Debug)]
pub struct Device {
    adapter: bluer::Adapter,
    session: bluer::Session,
    closed_tx: mpsc::Sender<()>,
}

impl Device {
    #[tracing::instrument(target = "device")]
    pub async fn create(config: DeviceConfig) -> Result<(Self, DeviceHandle), DeviceError> {
        let (close_tx, close_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let session = bluer::Session::new()
            .await
            .map_err(|err| DeviceError::SessionCreationFailed(err))?;
        let adapter = match config.dev_id {
            Some(adapter_name) => {
                let mut found_adapter = None;
                for name in session
                    .adapter_names()
                    .await
                    .map_err(|err| DeviceError::SessionCreationFailed(err))?
                {
                    if name == adapter_name {
                        found_adapter = Some(
                            session
                                .adapter(&adapter_name)
                                .map_err(|err| DeviceError::SessionCreationFailed(err))?,
                        );
                        break;
                    }
                }
                match found_adapter {
                    Some(adapter) => adapter,
                    None => return Err(DeviceError::NoSuchAdapterExists(adapter_name)),
                }
            }
            None => session
                .default_adapter()
                .await
                .map_err(|err| DeviceError::SessionCreationFailed(err))?,
        };
        // Set the adapter to be powered for use.
        adapter.set_powered(true).await?;
        // Wait for the handle to drop, then run cleanups.
        tokio::spawn({
            let adapter = adapter.clone();
            async move {
                close_tx.closed().await;
                // Turn off the device as a cleanup after use, silently fails if it's not successful.
                if let Err(err) = adapter.set_powered(false).await {
                    tracing::error!("failed to power off the device: {}", err.to_string());
                };
                tracing::info!("device terminated.");
                drop(closed_rx)
            }
        });
        Ok((
            Self {
                adapter,
                session,
                closed_tx,
            },
            DeviceHandle {
                _close_rx: close_rx,
            },
        ))
    }

    #[tracing::instrument(target = "device")]
    pub async fn check_paired_switches(&self, disconnect: bool) -> Result<(), DeviceError> {
        let Some(uuids) = self.uuids().await? else {
            return Ok(())
        };
        if uuids.len() > 3 {
            // https://btprodspecificationrefs.blob.core.windows.net/assigned-numbers/Assigned%20Number%20Types/Assigned_Numbers.pdf
            tracing::warn!("there's too many SDP-records active, Switch might refuse connection.");
            tracing::trace!("UUIDs: {:?}", &uuids);
            if disconnect {
                for target in self.paired_switches().await? {
                    tracing::info!("unpairing device of address: {}", target.address());
                    self.unpair_device(target.address().into()).await?;
                }
            } else {
                let paired_addresses: Vec<String> = self
                    .paired_switches()
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

    pub async fn address(&self) -> Result<Address, DeviceError> {
        let addr = self.adapter.address().await?;
        Ok(addr.into())
    }

    pub async fn paired_switches(&self) -> Result<Vec<bluer::Device>, DeviceError> {
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

    pub async fn register_sdp_record(&self) -> Result<bluer::rfcomm::ProfileHandle, DeviceError> {
        let handle = self
            .session
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
        Ok(handle)
    }

    pub async fn ensure_device_class(&self) -> Result<(), DeviceError> {
        // If current adapter's device class is same as expected, do nothing.
        if self.adapter.class().await? == DEVICE_CLASS {
            return Ok(());
        }
        let class = system::set_device_class(self.adapter_name(), DEVICE_CLASS).await?;
        if self.adapter.class().await? != class {
            return Err(DeviceError::DeviceClassSettingFailed);
        }
        Ok(())
    }

    pub async fn unpair_device(&self, address: Address) -> Result<(), DeviceError> {
        self.adapter.remove_device(address.into()).await?;
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

    pub async fn uuids(&self) -> Result<Option<HashSet<Uuid>>, DeviceError> {
        let uuids = self.adapter.uuids().await?;
        Ok(uuids)
    }

    pub fn closed(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
    }
}

#[derive(Debug)]
pub struct DeviceHandle {
    _close_rx: mpsc::Receiver<()>,
}

#[tracing::instrument(target = "device_helper")]
pub async fn ensure_adapter_compatible_address(
    dev_pair: (Device, DeviceHandle),
) -> Result<(Device, DeviceHandle), DeviceError> {
    let (dev, handle) = dev_pair;
    tracing::info!(
        "attempting to change MAC address of Bluetooth adapter to target compatible one."
    );
    let addr = dev.address().await?;
    if &addr[..3] != SWITCH_MAC_PREFIX {
        let adapter_name = dev.adapter_name().to_owned();
        let mut addr_bytes: [u8; 6] = [0; 6];
        addr_bytes[..3].copy_from_slice(SWITCH_MAC_PREFIX);
        addr_bytes[3..].copy_from_slice(&addr[3..]);
        let new_addr = Address::new(addr_bytes);
        system::set_adapter_address(adapter_name.as_str(), new_addr).await?;
        // We need to re-instantiate device.
        drop(dev);
        drop(handle);
        let (new_dev, new_handle) = Device::create(DeviceConfig {
            dev_id: Some(adapter_name.to_owned()),
        })
        .await?;
        let cur_addr: Address = new_dev.address().await?.into();
        if cur_addr != new_addr {
            tracing::error!(
                "failed to change MAC address of Bluetooth adapter: current={:?} desired={:?}",
                cur_addr,
                new_addr
            );
            return Err(DeviceError::MacAddrChangeFailed);
        }
        tracing::info!(
            "successfully changed MAC address of Bluetooth adapter to {}.",
            new_addr
        );
        return Ok((new_dev, new_handle));
    }
    Ok((dev, handle))
}
