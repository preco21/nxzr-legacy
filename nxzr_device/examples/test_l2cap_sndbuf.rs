use nxzr_device::sock::l2cap::{SeqPacket, SeqPacketListener, SocketAddr};
use tokio::time::{self, Duration};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    println!("Testing reset_sndbuf");

    let addr = SocketAddr::new(
        nxzr_device::Address::any(),
        nxzr_device::sock::AddressType::BrEdr,
        0,
    );
    let dummy_sock = SeqPacketListener::bind(addr).await?;

    let handle = tokio::spawn(async move {
        let (s, _) = dummy_sock.accept().await.unwrap();
        println!("accept");
        s.reset_sndbuf().unwrap();
        println!("okay");
    });

    let _ = SeqPacket::connect(addr).await?;
    println!("connect successful");

    handle.await?;
    println!("No error, exiting...");
    time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
