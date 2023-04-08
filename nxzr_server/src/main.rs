#[macro_use]
extern crate log;

use nxzr_core::controller::protocol::ControllerProtocol;
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::sleep,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let session = Session::new();
    let (transport, transport_handle) = Transport::register(session); // accepted 이후에 생성 필요
                                                                      // incoming messages are automatically read and write with protocol + transport.
    let (control, protocol_handle, close_rx) = ProtocolControl::new(transport).await?;
    // ㄴ run loop polling read, accept commands, fire writes to transport.
    // ㄴ detect connection lost,

    tokio::spawn(async move {
        loop {
            // shutdown signal ->
            // close_rx ->
            control.process_cmd(cmd);
            // when shutdown signal accepted
            // break;
        }
        drop(protocol_handle); // protocol control loop를 terminate 하는 역할, 내부에서 transport thread pause 함께?
        drop(transport_handle); // transport 내 이벤트 루프를 terminate 하는 역할
    });

    control.closed().await;
    transport.closed().await;

    Ok(())
}
