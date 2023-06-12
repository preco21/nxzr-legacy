use crate::{device, session, system, transport, Address};
use nxzr_core::{controller::ControllerType, protocol};
use strum::Display;
use thiserror::Error;
use tokio::{sync::mpsc, time};

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("failed to resolve paired switches automatically")]
    FailedToResolvePairedSwitches,
    #[error(transparent)]
    DeviceError(#[from] device::DeviceError),
    #[error(transparent)]
    TransportError(#[from] transport::TransportError),
    #[error(transparent)]
    SessionError(#[from] session::SessionError),
    #[error(transparent)]
    SysCheckError(#[from] system::SysCheckError),
    #[error(transparent)]
    SystemCommandError(#[from] system::SystemCommandError),
    #[error(transparent)]
    ProtocolError(#[from] protocol::ProtocolError),
}

#[tracing::instrument(target = "connection")]
pub async fn create_session_listener(
    device: &device::Device,
) -> Result<session::SessionListener, ConnectionError> {
    // Check if there's existing Switches and disconnect it.
    device.check_paired_switches(true).await?;

    // Create a session based on the device address.
    let dev_address = device.address().await?;
    let session_listener = session::SessionListener::new(session::SessionConfig {
        dev_address: Some(dev_address),
        ..Default::default()
    })?;

    // Bind to the session.
    session_listener.bind().await?;

    Ok(session_listener)
}

#[tracing::instrument(target = "connection")]
pub async fn create_session_listener_with_fallback(
    device_pair: (device::Device, device::DeviceHandle),
) -> Result<
    (
        session::SessionListener,
        device::Device,
        device::DeviceHandle,
    ),
    ConnectionError,
> {
    let (device, device_handle) = device_pair;

    // Check if there's existing Switches and disconnect it.
    device.check_paired_switches(true).await?;

    // Create a session based on the device address.
    let dev_address = device.address().await?;
    let session_listener = session::SessionListener::new(session::SessionConfig {
        dev_address: Some(dev_address),
        ..Default::default()
    })?;

    // When session is failed to bind, resort to the fallback and recreate `device`.
    if let Err(err) = session_listener.bind().await {
        tracing::warn!("{:?}", err);
        tracing::warn!("fallback: restarting Bluetooth session due to incompatibilities with the bluez `input` plugin, disable this plugin to avoid issues.");
        tracing::info!("restarting Bluetooth service...");
        let dev_id = device.adapter_name().to_string();
        // Destroy the existing device in order to replace it with new one.
        drop(device);
        drop(device_handle);
        system::restart_bluetooth_service().await?;
        time::sleep(time::Duration::from_millis(1000)).await;
        let (device, device_handle) = device::Device::create(device::DeviceConfig {
            dev_id: Some(dev_id),
        })
        .await?;
        // If it failed again, just bail out.
        session_listener.bind().await?;
        return Ok((session_listener, device, device_handle));
    }

    Ok((session_listener, device, device_handle))
}

#[tracing::instrument(target = "connection")]
pub async fn establish_initial_connection(
    device: &device::Device,
    session_listener: &session::SessionListener,
    controller_type: ControllerType,
) -> Result<session::PairedSession, ConnectionError> {
    session_listener.listen().await?;
    device.set_pairable(true).await?;

    tracing::info!("setting device alias to \"{}\"", controller_type.name());
    device.set_alias(controller_type.name()).await?;

    tracing::info!("advertising Bluetooth SDP record...");
    let record_handle = device.register_sdp_record().await?;
    device.set_discoverable(true).await?;

    tracing::info!("setting device class...");
    device.ensure_device_class().await?;

    tracing::info!("waiting for Switch to connect...");
    let paired_session = session_listener.accept().await?;
    device.set_discoverable(false).await?;
    device.set_pairable(false).await?;

    // Drop the SDP-record advertisement.
    drop(record_handle);

    Ok(paired_session)
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ReconnectType {
    Auto,
    Manual(Address),
}

#[tracing::instrument(target = "connection")]
pub async fn establish_reconnect_connection(
    device: &device::Device,
    reconnect_type: ReconnectType,
) -> Result<(session::PairedSession, Address), ConnectionError> {
    let target_addr: Address = match reconnect_type {
        ReconnectType::Auto => {
            let paired_switches = device.paired_switches().await?;
            if paired_switches.is_empty() {
                return Err(ConnectionError::FailedToResolvePairedSwitches);
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
    Ok((paired_session, target_addr))
}

#[derive(Debug)]
pub struct ConnectionConfig {
    pub paired_session: session::PairedSession,
    pub controller_type: ControllerType,
}

#[derive(Debug)]
pub struct Connection {
    protocol: protocol::Protocol,
    transport: transport::Transport,
    will_close_tx: mpsc::Sender<()>,
}

impl Connection {
    #[tracing::instrument(target = "connection")]
    pub async fn run(
        config: ConnectionConfig,
    ) -> Result<(Self, ConnectionHandle), ConnectionError> {
        let ConnectionConfig {
            paired_session,
            controller_type,
        } = config;
        let dev_address = paired_session.dev_address;
        let reconnect = paired_session.is_reconnect;

        // Use that paired session for the further processing.
        let (transport, transport_handle) =
            transport::Transport::register(paired_session, transport::TransportConfig::default())
                .await?;
        let (protocol, protocol_handle) = protocol::Protocol::connect(
            transport.clone(),
            protocol::ProtocolConfig {
                dev_address: dev_address.into(),
                controller_type,
                reconnect,
                ..Default::default()
            },
        )
        .await?;

        let (close_tx, close_rx) = mpsc::channel(1);
        let (will_close_tx, will_close_rx) = mpsc::channel(1);
        tokio::spawn({
            let protocol = protocol.clone();
            let transport = transport.clone();
            async move {
                tokio::select! {
                    _ = protocol.closed() => {},
                    _ = transport.closed() => {},
                    _ = close_tx.closed() => {},
                }
                drop(will_close_rx);
                drop(protocol_handle);
                drop(transport_handle);
            }
        });

        Ok((
            Self {
                protocol,
                transport,
                will_close_tx,
            },
            ConnectionHandle {
                _close_rx: close_rx,
            },
        ))
    }

    pub fn protocol(&self) -> protocol::Protocol {
        self.protocol.clone()
    }

    pub async fn will_close(&self) {
        self.will_close_tx.closed().await;
    }

    pub async fn closed(&self) {
        self.protocol.closed().await;
        self.transport.closed().await;
    }
}

pub struct ConnectionHandle {
    _close_rx: mpsc::Receiver<()>,
}
