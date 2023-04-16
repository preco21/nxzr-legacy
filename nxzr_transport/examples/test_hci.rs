use std::time::Duration;

use nxzr_transport::sock::hci::{Datagram, Filter, SocketAddr};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::sleep,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let local_sa = SocketAddr::new(0);
    let dg = Datagram::bind(local_sa).await?;
    dg.as_ref().set_filter(Filter {
        type_mask: 1 << 0x04,
        event_mask: [1 << 0x13, 0],
        opcode: 0,
    })?;

    println!("Listening on local... Press enter to quit.");
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        println!("\nReading for hci socket...");

        let buf_size = 10;
        let mut buf = vec![0; buf_size as _];
        tokio::select! {
            n = dg.recv(&mut buf) => {
                match n {
                    Ok(0) => {
                        println!("Stream ended");
                        break;
                    },
                    Ok(n) => {
                        let buf = &buf[..n];
                        println!("Received {} bytes", buf.len());
                    },
                    Err(err) => {
                        println!("Read failed: {}", &err);
                        continue;
                    },
                }
            },
            _ = lines.next_line() => break,
        };
    }

    println!("Exiting...");
    sleep(Duration::from_secs(1)).await;

    Ok(())
}
