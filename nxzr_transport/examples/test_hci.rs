use std::time::Duration;

use nxzr_transport::sock::{
    hci::{HciSocket, Socket, SocketAddr, StreamListener},
    sys::hci_filter,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    time::sleep,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let local_sa = SocketAddr::new(0);
    let listener = StreamListener::bind(local_sa).await?;
    listener.as_ref().set_filter(hci_filter {
        type_mask: 1 << 0x04,
        event_mask: [1 << 0x13, 0],
        opcode: 0,
    })?;

    println!("Listening on local... Press enter to quit.");
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        println!("\nWaiting for connection...");

        let (mut stream, sa) = tokio::select! {
            l = listener.accept() => {
                match l {
                    Ok(v) => v,
                    Err(err) => {
                        println!("Accepting connection failed: {}", &err);
                        continue;
                    }
                }
            },
            _ = lines.next_line() => break,
        };

        loop {
            let buf_size = 10;
            let mut buf = vec![0; buf_size as _];
            let n = match stream.read(&mut buf).await {
                Ok(0) => {
                    println!("Stream ended");
                    break;
                }
                Ok(n) => n,
                Err(err) => {
                    println!("Read failed: {}", &err);
                    continue;
                }
            };
            let buf = &buf[..n];
            println!("Received {} bytes", buf.len());
        }
    }

    println!("Exiting...");
    sleep(Duration::from_secs(1)).await;

    Ok(())
}
