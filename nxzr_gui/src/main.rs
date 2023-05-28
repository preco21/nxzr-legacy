mod bootstrap;
mod common;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a tracer.
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    bootstrap::bootstrap_program().await?;
    // Check whether the program runs with elevated privileges.
    // bootstrap::install_system_requirements().await?;
    Ok(())
}
