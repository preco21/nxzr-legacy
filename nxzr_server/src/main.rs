use anyhow::Ok;
use clap::{Parser, Subcommand};
use nxzr_device::{
    device::{self, DeviceConfig},
    system,
};
use nxzr_shared::shutdown::Shutdown;
use service::NxzrService;
use std::{future::Future, net::ToSocketAddrs, sync::Arc};
use tokio::{signal, sync::mpsc};
use tracing_subscriber::prelude::*;

mod controller;
mod service;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run server daemon
    Run,
    /// Run system integrity check
    Check,
}

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

    // Run CLI.
    let args = Cli::parse();
    match args.command {
        Cmd::Run => {
            tracing::info!("running daemon...");
            // Checks for system requirements.
            system::check_privileges().await?;
            system::check_system_requirements().await?;
            // Then, runs the actual service.
            run(signal::ctrl_c()).await?
        }
        Cmd::Check => {
            tracing::info!("running system check...");
            // Checks for system requirements only, then exits.
            system::check_privileges().await?;
            system::check_system_requirements().await?;
        }
    }

    Ok(())
}

pub async fn run(shutdown: impl Future) -> anyhow::Result<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);
    let shutdown_token = Shutdown::new(shutdown_tx, shutdown_complete_tx.clone());

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

    let addr = "[::1]:50052"
        .to_socket_addrs()?
        .next()
        .ok_or(anyhow::anyhow!("failed to select an address to bind"))?;
    tracing::info!("service listening on: {}", addr.to_string());
    let service_task_handle = tokio::spawn({
        let device = device.clone();
        let shutdown_token = shutdown_token.clone();
        async move {
            let _shutdown_guard = shutdown_token.drop_guard();
            let nxzr_service = NxzrService::new(device, shutdown_token.clone()).await?;
            let svc = nxzr_proto::nxzr_server::NxzrServer::new(nxzr_service);
            tonic::transport::Server::builder()
                .add_service(svc)
                .serve_with_shutdown(addr, shutdown_token.recv_shutdown())
                .await?;
            Ok(())
        }
    });

    tokio::select! {
        _ = shutdown => {
            tracing::info!("kill signal received, closing...");
        },
        // A cloned `shutdown_token` is passed to each task so that the task can
        // issue a shutdown internally, this will be considered as a normal
        // shutdown signal too.
        _ = shutdown_token.recv_shutdown() => {
            tracing::info!("internal shutdown request received, closing...");
        },
    }
    tracing::info!("initiating process shutdown...");

    // When `shutdown_rx` is dropped, all tasks which have called
    // `shutdown_tx.closed()` will receive the shutdown signal and can exit.
    drop(shutdown_rx);

    // Wait for the service to close.
    tracing::info!("waiting for services to close...");
    if let Err(err) = service_task_handle.await {
        tracing::error!("error while terminating NXZR task handle: {}", err);
    };

    // Wait for all active background tasks to finish processing. As the
    // `Sender` handle of `shutdown_complete_tx` held by this scope has been
    // dropped above, the only remaining `Sender` instances are held by
    // background tasks. When those drop, the `mpsc` channel will close and
    // `recv()` will return `None`.
    tracing::info!("waiting for background tasks to finish cleanup...");
    // Drop final `Sender` of `shutdown_complete_tx` so the `Receiver` half can complete.
    drop(shutdown_complete_tx);
    let _ = shutdown_complete_rx.recv().await;

    // Finally, wait for the device instance to close.
    tracing::info!("terminating device...");
    drop(device_handle);
    device.closed().await;

    tracing::info!("daemon successfully terminated.");
    Ok(())
}
