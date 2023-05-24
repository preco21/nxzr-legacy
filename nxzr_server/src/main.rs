use clap::{builder::PossibleValue, Parser, Subcommand, ValueEnum};
use nxzr_device::system;

mod external_scripts;

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
    /// Run setup
    Setup(SetupOpts),
}

#[derive(Parser)]
struct RunOpts {}

impl RunOpts {
    pub async fn perform(self) -> anyhow::Result<()> {
        //
        Ok(())
    }
}

#[derive(Parser)]
struct SetupOpts {
    #[arg(short, long, value_enum)]
    mode: SetupMode,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SetupMode {
    InstallServer,
    SetupConfig,
}

impl ValueEnum for SetupMode {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::InstallServer, Self::SetupConfig]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
        Some(match self {
            Self::InstallServer => {
                PossibleValue::new("install").help("Install system requirements for the daemon")
            }
            Self::SetupConfig => PossibleValue::new("setup").help("Setup config for the daemon"),
        })
    }
}

impl SetupOpts {
    pub async fn perform(self) -> anyhow::Result<()> {
        match self.mode {
            SetupMode::InstallServer => {
                println!("Running server install...");
                external_scripts::run_server_install()?;
                println!("Successfully installed required components.");
            }
            SetupMode::SetupConfig => {
                println!("Running config setup...");
                external_scripts::run_setup_config()?;
                println!("Successfully made changes for system config.");
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
        Cmd::Setup(r) => r.perform().await?,
    }
    Ok(())
}
