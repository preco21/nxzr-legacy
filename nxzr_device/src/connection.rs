use crate::{
    device::{self, DeviceHandle},
    session, system, Address,
};
use nxzr_core::controller::ControllerType;
use strum::Display;
use thiserror::Error;
use tokio::time;

#[derive(Error, Debug)]
pub enum DeviceConnectionError {
    #[error("failed to resolve paired switches automatically")]
    FailedToResolvePairedSwitches,
    #[error(transparent)]
    DeviceError(#[from] device::DeviceError),
    #[error(transparent)]
    SessionError(#[from] session::SessionError),
    #[error(transparent)]
    SysCheckError(#[from] system::SysCheckError),
    #[error(transparent)]
    SystemCommandError(#[from] system::SystemCommandError),
}

#[tracing::instrument(target = "device_main")]
pub async fn establish_initial_connection(
    dev_id: Option<String>,
    controller_type: ControllerType,
) -> Result<(session::PairedSession, Address, DeviceHandle), DeviceConnectionError> {
    let (mut device, mut handle) = device::Device::create(device::DeviceConfig {
        dev_id: dev_id.clone(),
    })
    .await?;
    device.check_paired_switches(true).await?;

    let address = device.address().await?;
    let session = session::SessionListener::new(session::SessionConfig {
        address: Some(address),
        ..Default::default()
    })?;

    // When session is failed to bind, resort to the fallback and recreate `device`.
    if let Err(err) = session.bind().await {
        tracing::warn!("{:?}", err);
        tracing::warn!("fallback: restarting Bluetooth session due to incompatibilities with the bluez `input` plugin, disable this plugin to avoid issues.");
        tracing::info!("restarting Bluetooth service...");
        system::restart_bluetooth_service().await?;
        time::sleep(time::Duration::from_millis(1000)).await;
        (device, handle) = device::Device::create(device::DeviceConfig { dev_id }).await?;
        // If it failed again, just bail out.
        session.bind().await?;
    };

    // Start listening on the session and prepare for connection.
    session.listen().await?;
    device.set_pairable(true).await?;

    tracing::info!("setting device alias to {}", controller_type.name());
    device.set_alias(controller_type.name()).await?;

    tracing::info!("advertising Bluetooth SDP record...");
    let record_handle = device.register_sdp_record().await?;
    device.set_discoverable(true).await?;

    tracing::info!("setting device class...");
    device.ensure_device_class().await?;

    tracing::info!("waiting for Switch to connect...");
    // Trying to accept incoming connection.
    let paired_session = session.accept().await?;
    device.set_discoverable(false).await?;
    device.set_pairable(false).await?;

    // Drop SDP-record advertisement.
    drop(record_handle);
    Ok((paired_session, address, handle))
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ReconnectType {
    Auto,
    Manual(Address),
}

#[tracing::instrument(target = "device_main")]
pub async fn establish_reconnect_connection(
    dev_id: Option<String>,
    controller_type: ControllerType,
    reconnect: ReconnectType,
) -> Result<(session::PairedSession, Address, DeviceHandle), DeviceConnectionError> {
    let (device, handle) = device::Device::create(device::DeviceConfig {
        dev_id: dev_id.clone(),
    })
    .await?;
    let target_addr: Address = match reconnect {
        ReconnectType::Auto => {
            let paired_switches = device.paired_switches().await?;
            if paired_switches.is_empty() {
                return Err(DeviceConnectionError::FailedToResolvePairedSwitches);
            }
            if paired_switches.len() > 1 {
                tracing::warn!(
                    "found the multiple paired switches, using the first one as a default."
                );
            }
            paired_switches[0].address().into()
        }
        ReconnectType::Manual(addr) => addr,
    };
    let paired_session = session::PairedSession::connect(session::PairedSessionConfig {
        reconnect_address: target_addr,
        ..Default::default()
    })
    .await?;
    Ok((paired_session, target_addr, handle))
}
