use nxzr_core::controller::protocol::Protocol;
use nxzr_device::{
    device::{Device, DeviceConfig},
    session::{SessionConfig, SessionListener},
};
use std::{error::Error, time::Duration};

mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let device = Device::new(DeviceConfig {
        // FIXME: accept device id here
        ..Default::default()
    })
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
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let (cmd_tx, cmd_rx) = mpsc::channel(1);
    let session = Session::new();
    let ctrl = ProtocolControl::new(session, shutdown_tx); // transport 생성, protocol 루프 돌리기, accepted 이후 작동
                                                           // ㄴ 1. poll read/write
                                                           // ㄴ 2. wait for first connection

    tokio::spawn(async move {
        ctrl.connected().await;
        loop {
            select! {
                cmd = cmd_rx.recv() => ctrl.process_cmd(cmd),
                _ = shutdown_tx.closed() => break,
            }
        }
    });

    // run and waits for the connection to lost
    ctrl.run().await;

    Ok(())
}
