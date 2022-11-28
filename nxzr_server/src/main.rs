#[macro_use]
extern crate log;

use bluer::adv::Advertisement;
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::sleep,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    env_logger::init();

    info!("Hello, world!");

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    println!(
        "Advertising on Bluetooth adapter {} with address {}",
        adapter.name(),
        adapter.address().await?
    );
    let le_advertisement = Advertisement {
        advertisement_type: bluer::adv::Type::Peripheral,
        service_uuids: vec!["123e4567-e89b-12d3-a456-426614174000".parse().unwrap()]
            .into_iter()
            .collect(),
        discoverable: Some(true),
        local_name: Some("le_advertise".to_string()),
        ..Default::default()
    };
    println!("{:?}", &le_advertisement);
    let handle = adapter.advertise(le_advertisement).await?;

    println!("Press enter to quit");
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let _ = lines.next_line().await;

    println!("Removing advertisement");
    drop(handle);
    sleep(Duration::from_secs(1)).await;

    Ok(())
}
