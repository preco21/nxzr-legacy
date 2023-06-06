use std::{future::Future, net::ToSocketAddrs};

use crate::server::Server;
use nxzr_device::system;
use nxzr_proto::tracing_server;
use server::ServerOpts;
use tokio::{
    signal,
    sync::{broadcast, mpsc},
};
use tracing_subscriber::prelude::*;

mod common;
mod controller;
mod server;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a tracer.
    let (log_tx, mut log_rx) = mpsc::unbounded_channel();
    let writer_channel = common::WriterChannel::new(log_tx);
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::TRACE)
        .with(tracing_subscriber::fmt::Layer::default().with_writer(writer_channel))
        .with(tracing_subscriber::fmt::Layer::default());
    tracing::subscriber::set_global_default(subscriber)?;

    // Tonic service requires cloneable channel so that normal mpsc channels cannot be used.
    let (tracing_tx, _tracing_rx) = broadcast::channel(1024);
    tokio::spawn({
        let tracing_tx = tracing_tx.clone();
        async move {
            while let Some(log) = log_rx.recv().await {
                let _ = tracing_tx.send(log);
            }
        }
    });

    // Run system checks.
    system::check_privileges().await?;
    system::prepare_device().await?;

    tokio::spawn(async move {
        // Run the main service.
        run(ServerOpts::default(), signal::ctrl_c()).await.unwrap();
    });

    // Setup the tracing service.
    tonic::transport::Server::builder()
        .add_service(tracing_server::TracingServer::new(
            service::TracingService::new(tracing_tx),
        ))
        .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();

    // Run cleanup.
    system::cleanup_device().await;
    Ok(())
}

// server::run(
//     ServerOpts {
//         reconnect: Some(ReconnectType::Manual(Address::from_str(
//             &addr_or_auto,
//         )?)),
//         ..Default::default()
//     },
//     signal::ctrl_c(),
// )

pub async fn run(opts: ServerOpts, shutdown: impl Future) -> anyhow::Result<()> {
    let (server, server_handle) = Server::run(opts).await?;
    tokio::select! {
        _ = server.will_close() => {},
        _ = shutdown => {},
    }
    drop(server_handle);
    server.closed().await;
    Ok(())
}

// // FIXME: use rwlock to store Server?
// // FIXME: or just spawn a manager thread?
// // FIXME: shutdown handling must be at top-level. as mini-redis, and it must be coordinated by broadcast channel...
// async fn run(opts: ServerOpts, shutdown: impl Future) -> anyhow::Result<()> {
//     // Use tokio::spawn and channels to notify close.
//     let (server, server_handle) = tokio::select! {
//         res = Server::run(opts) => res?,
//         _ = shutdown => return Ok(()),
//     };
//     server.will_close().await;
//     // FIXME: spawn server or change to
//     // tokio::select! {
//     //     _ = server.will_close() => {},
//     //     _ = shutdown => {},
//     // }
//     drop(server_handle);
//     server.closed().await;

//     Ok(())
// }
