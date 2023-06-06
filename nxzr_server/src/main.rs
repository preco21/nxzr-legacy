use std::{future::Future, net::ToSocketAddrs};

use crate::server::Server;
use nxzr_device::system;
use server::ServerOpts;
use tokio::signal;
use tracing_subscriber::prelude::*;

mod controller;
mod server;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracer()?;

    system::check_privileges().await?;
    system::prepare_device().await?;

    // tokio::spawn(async move {
    //     // Run the main service.
    // });
    run(ServerOpts::default(), signal::ctrl_c()).await.unwrap();

    // Setup the tracing service.
    // tonic::transport::Server::builder()
    //     .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
    //     .await
    //     .unwrap();

    system::cleanup_device().await;
    Ok(())
}

fn setup_tracer() -> anyhow::Result<()> {
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
