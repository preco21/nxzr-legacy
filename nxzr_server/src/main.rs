use std::str::FromStr;

use nxzr_device::{system, Address, ReconnectType};
use server::ServerOpts;
use tokio::signal;

mod controller;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a tracer.
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
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
