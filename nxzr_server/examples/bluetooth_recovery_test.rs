use nxzr_device::{device, system};
use tokio::time::{self, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a tracer.
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    system::check_system_requirements().await?;

    let (device, _handle) = device::Device::create(device::DeviceConfig::default()).await?;

    loop {
        let fut = async {
            let paired_switches = device.paired_switches().await?;
            let adapter_addr = device.address().await?;

            tracing::info!("paired dev: {:?}", adapter_addr);
            tracing::info!("paired switches: {:?}", paired_switches);

            anyhow::Result::<()>::Ok(())
        };
        let res = fut.await;
        tracing::trace!("{:?}", res);

        time::sleep(Duration::from_millis(1000)).await;
    }
}
