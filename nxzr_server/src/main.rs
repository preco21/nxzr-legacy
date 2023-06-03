use std::str::FromStr;

use clap::{builder::PossibleValue, Parser, Subcommand, ValueEnum};
use nxzr_device::{system, Address};
use server::{ReconnectType, ServerOpts};
use tokio::signal;

mod server;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run server daemon
    Run(RunOpts),
}

#[derive(Parser)]
struct RunOpts {
    #[arg(short, long)]
    reconnect: Option<String>,
}

impl RunOpts {
    pub async fn perform(self) -> anyhow::Result<()> {
        match self.reconnect {
            Some(addr_or_auto) => {
                if addr_or_auto == "auto" {
                    tracing::info!("running server with automatic reconnect mode.");
                    server::run(
                        ServerOpts {
                            reconnect: Some(ReconnectType::Auto),
                            ..Default::default()
                        },
                        signal::ctrl_c(),
                    )
                    .await?
                } else {
                    tracing::info!("running server with manual reconnect mode.");
                    server::run(
                        ServerOpts {
                            reconnect: Some(ReconnectType::Manual(Address::from_str(
                                &addr_or_auto,
                            )?)),
                            ..Default::default()
                        },
                        signal::ctrl_c(),
                    )
                    .await?
                }
            }
            None => {
                tracing::info!("running server with initial connection mode.");
                server::run(ServerOpts::default(), signal::ctrl_c()).await?
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a tracer.
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    // Check whether the program runs with elevated privileges.
    system::check_privileges().await?;
    // Run CLI.
    let args = Cli::parse();
    match args.command {
        Cmd::Run(r) => r.perform().await?,
        #[cfg(feature = "setup-support")]
        Cmd::Setup(r) => r.perform().await?,
    }
    Ok(())
}
