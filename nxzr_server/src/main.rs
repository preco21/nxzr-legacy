use anyhow::Ok;
use nxzr_device::{
    device::{self, DeviceConfig},
    system,
};
use service::NxzrService;
use std::{future::Future, net::ToSocketAddrs, sync::Arc};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::prelude::*;

mod controller;
mod server;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let module_filter = tracing_subscriber::filter::Targets::new()
        .with_target("nxzr_core", tracing::Level::TRACE)
        .with_target("nxzr_device", tracing::Level::TRACE)
        .with_target("nxzr_server", tracing::Level::TRACE);
    // Conditionally sets event format between debug/release mode.
    #[cfg(debug_assertions)]
    let event_format = tracing_subscriber::fmt::format();
    #[cfg(not(debug_assertions))]
    let event_format = tracing_subscriber::fmt::format().json();
    let subscriber = tracing_subscriber::registry()
        .with(module_filter)
        .with(tracing_subscriber::fmt::Layer::default().event_format(event_format));
    tracing::subscriber::set_global_default(subscriber)?;

    // Check for system requirements.
    system::check_privileges().await?;
    system::check_system_requirements().await?;

    Ok(())
}

pub async fn run(shutdown: impl Future) -> anyhow::Result<()> {
    let shutdown_token = CancellationToken::new();
    let (will_close_tx, mut will_close_rx) = mpsc::channel::<()>(1);
    let (notify_shutdown_tx, notify_shutdown_rx) = mpsc::channel::<()>(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel::<()>(1);

    // Setup a device.
    //
    // Note that the device will only rely on the first adapter (e.g. hci0), and
    // will never restart, for example, due to incompatibilities with the bluez
    // `input` plugin.
    //
    // This is guaranteed to not happen because we will only serve the daemon
    // managed in a container of the WSL.
    tracing::info!("setting up device...");
    let (device, device_handle) = device::Device::create(DeviceConfig::default()).await?;
    let device = Arc::new(device);

    tracing::info!("setting up NXZR services...");
    let addr = "[::1]:50051"
        .to_socket_addrs()?
        .next()
        .ok_or(anyhow::anyhow!("failed to select an address to bind"))?;
    let nxzr_task_handle = tokio::spawn(async move {
        let nxzr_service = NxzrService::new(device.clone(), notify_shutdown_tx.closed(), shutdown_complete_tx.).await?;
        let svc = nxzr_proto::nxzr_server::NxzrServer::new(nxzr_service);
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve_with_shutdown(addr, notify_shutdown_tx.closed())
            .await?;
        let _ = will_close_tx.send(()).await;
        Ok(())
    });

    tokio::select! {
        _ = shutdown => {},
        // When spawned tasks which have `will_close_tx` use it in case a
        // shutdown was issued from inside of that task, this will also be
        // considered as a shutdown signal.
        _ = will_close_rx.recv() => {},
    }
    tracing::info!("shutdown signal received, terminating...");

    // When `notify_shutdown_rx` is dropped, all tasks which have called
    // `notify_shutdown_tx.closed()` will receive the shutdown signal and can
    // exit.
    drop(notify_shutdown_rx);
    // Drop final `Sender` of `shutdown_complete_tx` so the `Receiver` below can complete.
    drop(shutdown_complete_tx);

    // Wait for the service to close.
    tracing::info!("waiting for services to close...");
    if let Err(err) = nxzr_task_handle.await {
        tracing::error!("error while terminating NXZR task handle: {}", err);
    };

    // Wait for all active background tasks to finish processing. As the
    // `Sender` handle of `shutdown_complete_tx` held by this scope has been
    // dropped above, the only remaining `Sender` instances are held by
    // background tasks. When those drop, the `mpsc` channel will close and
    // `recv()` will return `None`.
    tracing::info!("waiting for the background tasks to finish processing...");
    let _ = shutdown_complete_rx.recv().await;

    // Finally, wait for the device instance to close.
    tracing::info!("terminating device...");
    drop(device_handle);
    device.closed().await;

    tracing::info!("successfully shutdown the service gracefully.");
    Ok(())
}
