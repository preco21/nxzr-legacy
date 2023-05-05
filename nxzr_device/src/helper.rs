use crate::{
    device::{Device, DeviceConfig, DeviceError},
    sock::Address,
};
use bytes::Buf;
use thiserror::Error;
use tokio::{io::BufReader, process::Command};

const SWITCH_MAC_PREFIX: &[u8] = &[0x94, 0x59, 0xCB];

#[derive(Clone, Error, Debug)]
pub enum HelperError {
    #[error("failed to execute a command: {0}")]
    CommandFailed(String),
    #[error("device error: {0}")]
    Device(DeviceError),
    #[error("internal error: {0}")]
    Internal(HelperInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum HelperInternalError {
    #[error("utf8: {0}")]
    Utf8Error(std::str::Utf8Error),
    #[error("io: {0}")]
    Io(std::io::ErrorKind),
    #[error("bluer: {0}")]
    Bluer(bluer::ErrorKind),
}

impl From<DeviceError> for HelperError {
    fn from(err: DeviceError) -> Self {
        Self::Device(err)
    }
}

impl From<std::str::Utf8Error> for HelperError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Internal(HelperInternalError::Utf8Error(err))
    }
}

impl From<std::io::Error> for HelperError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(HelperInternalError::Io(err.kind()))
    }
}

impl From<bluer::Error> for HelperError {
    fn from(err: bluer::Error) -> Self {
        Self::Internal(HelperInternalError::Bluer(err.kind))
    }
}

#[tracing::instrument(target = "helper")]
pub async fn set_adapter_address(adapter_name: &str, address: Address) -> Result<(), HelperError> {
    tracing::info!(
        "resetting the bluetooth adapter ({:?}) with address `{:?}`",
        adapter_name,
        address
    );
    // Set the bluetooth adapter address by adapter name.
    //
    // The user will need to install `apt-get install bluez-utils`.
    run_command({
        let mut cmd = Command::new("bdaddr");
        cmd.args(&["-i", adapter_name, address.to_string().as_ref()]);
        cmd
    })
    .await?;
    // Reset the bluetooth adapter by running `hciconfig`.
    run_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&[adapter_name, "reset"]);
        cmd
    })
    .await?;
    // Restart bluetooth service.
    systemctl::restart("bluetooth.service")?;
    Ok(())
}

#[tracing::instrument(target = "helper")]
pub async fn ensure_adapter_address_switch(device: Device) -> Result<Device, HelperError> {
    let addr = device.address().await?;
    if &addr.as_ref()[..3] != SWITCH_MAC_PREFIX {
        let adapter_name = device.adapter_name().to_owned();
        let mut addr_bytes: [u8; 6] = [0x00; 6];
        addr_bytes[..3].copy_from_slice(SWITCH_MAC_PREFIX);
        addr_bytes[3..].copy_from_slice(&addr.as_ref()[3..]);
        set_adapter_address(adapter_name.as_str(), Address::new(addr_bytes)).await?;
        // We need to re-instantiate device.
        drop(device);
        return Ok(Device::new(DeviceConfig {
            id: Some(adapter_name.to_owned()),
        })
        .await?);
    }
    Ok(device)
}

#[tracing::instrument(target = "helper")]
pub async fn set_device_class(adapter_name: String) -> Result<(), HelperError> {
    Ok(())
}

pub async fn run_command(mut command: Command) -> Result<(), HelperError> {
    let output = command.output().await?;
    if !output.status.success() {
        return Err(HelperError::CommandFailed(
            std::str::from_utf8(output.stderr.as_ref())?.to_owned(),
        ));
    }
    Ok(())
}
