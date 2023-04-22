use std::time::Duration;

use nxzr_transport::sock::hci::{Datagram, Filter, SocketAddr};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::sleep,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// NOTE: To test HCI sockets properly, you must run this example as "root".
// You can test the example with following commands:
// sudo hcitool -i hci0 cmd 0x04 0x0008 04 13 05 01 01 00 01 00
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let local_sa = SocketAddr::new(0);
    let dg = Datagram::bind(local_sa).await?;
    // dg.as_ref().set_filter(Filter {
    //     type_mask: 0xffffffff,
    //     event_mask: [0xffffffff, 0xffffffff],
    //     opcode: 0,
    // })?;
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
                        println!("{buf:?}");
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
