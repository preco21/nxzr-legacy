use nxzr_device::system;
use server::ServerOpts;
use tokio::signal;
use tracing_subscriber::prelude::*;

mod controller;
mod server;
mod service;
// mod tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a tracer.
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::TRACE)
        .with(tracing_subscriber::fmt::Layer::default());
    tracing::subscriber::set_global_default(subscriber)?;

    // Run system checks.
    system::check_privileges().await?;
    system::prepare_device().await?;

    // Run main service.
    server::run(ServerOpts::default(), signal::ctrl_c()).await?;
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
