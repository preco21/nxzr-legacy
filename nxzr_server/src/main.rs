use nxzr_device::system;
use server::ServerOpts;
use tokio::{signal, sync::mpsc};
use tracing_subscriber::prelude::*;

mod common;
mod controller;
mod server;
// mod service;
// mod tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (log_tx, mut log_rx) = mpsc::unbounded_channel();
    let writer_channel = common::WriterChannel::new(log_tx);

    tokio::spawn(async move {
        while let Some(t) = log_rx.recv().await {
            println!("relay channel: {}", t);
        }
        println!("stream ended");
    });

    // Setup a tracer.
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::TRACE)
        .with(tracing_subscriber::fmt::Layer::default().with_writer(writer_channel))
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
