use nxzr_device::{device, system};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a tracer.
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    system::prepare_device().await?;

    let device = device::Device::new(device::DeviceConfig::default()).await?;
    let paired_switches = device.paired_switches().await?;
    let adapter_addr = device.address().await?;

    tracing::info!("paired dev: {:?}", adapter_addr);
    tracing::info!("paired switches: {:?}", paired_switches);

    Ok(())
}
