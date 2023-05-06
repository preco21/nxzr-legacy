use nxzr_core::{
    controller::{
        protocol::{Protocol, ProtocolConfig, TransportRead},
        ControllerType,
    },
    protocol::{ProtocolControl, Transport as ProtocolTransport},
};
use nxzr_device::{
    device::{Device, DeviceConfig},
    helper,
    session::{SessionConfig, SessionListener},
    syscheck,
    transport::{Transport, TransportConfig},
};
use std::{error::Error, time::Duration};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    syscheck::check_system_requirements().await?;

    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    tracing_subscriber::fmt::init();

    // FIXME: accept device id here
    let mut device = Device::new(DeviceConfig::default())
        .await?
        .ensure_adapter_address_switch()
        .await?;

    // FIXME: implement reconnect, currently connecting from scratch only supported
    device.check_paired_devices(true).await?;

    let session = SessionListener::new(SessionConfig {
        address: Some(device.address().await?.into()),
        ..Default::default()
    })?;

    if let Err(err) = session.bind().await {
        tracing::warn!("{:?}", err);
        tracing::warn!("fallback: restarting the bluetooth session due to incompatibilities with the bluez `input` plugin, disable this plugin to avoid issues.");
        tracing::info!("restarting bluetooth service...");
        helper::restart_bluetooth_service()?;
        sleep(Duration::from_millis(1000)).await;
        // FIXME: accept device id here
        device = Device::new(DeviceConfig::default()).await?;
        session.bind().await?;
    };

    session.listen().await?;

    device.set_powered(true).await?;
    device.set_pairable(true).await?;

    // FIXME: make it customizable
    device
        .set_alias(ControllerType::ProController.name())
        .await?;

    tracing::info!("advertising the bluetooth SDP record...");

    // FIXME: allow ignoring errors
    device.register_sdp_record().await?;

    device.set_discoverable(true).await?;
    device.set_class().await?;

    tracing::info!("waiting for Switch to connect...");

    let paired_session = session.accept().await?;
    device.set_discoverable(false).await?;
    device.set_pairable(false).await?;

    let (transport, transport_handle) =
        Transport::register(paired_session, TransportConfig::default()).await?;
    // FIXME: allow customizing config
    let (protocol, protocol_handle) =
        ProtocolControl::connect(transport, ProtocolConfig::default())?;

    let event_rx = protocol.events().await?;

    // FIXME:
    tokio::spawn(async move {
        while let Some(evt) = event_rx.recv().await {
            tracing::info!("{:?}", evt);
        }
    });

    select! {
        _ = transport.closed() => break,
        _ = protocol.closed() => break,
        _ = shutdown_tx.closed() => break,
    }

    drop(protocol_handle);
    drop(transport_handle);

    Ok(())
}
