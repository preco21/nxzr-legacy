#[macro_use]
extern crate log;

use nxzr_core::controller::protocol::Protocol;
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    join, select,
    sync::mpsc,
    time::sleep,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let (cmd_tx, cmd_rx) = mpsc::channel(1);
    let session = Session::new();
    let ctrl = ProtocolControl::new(session, shutdown_tx); // transport 생성, protocol 루프 돌리기, accepted 이후 작동
                                                           // ㄴ 1. poll read/write
                                                           // ㄴ 2. wait for first connection

    tokio::spawn(async move {
        ctrl.connected().await;
        loop {
            select! {
                cmd = cmd_rx.recv() => ctrl.process_cmd(cmd),
                _ = shutdown_tx.closed() => break,
            }
        }
    });

    // run and waits for the connection to lost
    ctrl.run().await;

    Ok(())
}

struct ProtocolControl {}

impl ProtocolControl {
    async fn run(&self) {
        let (transport, transport_handle) = Transport::register();
        let protocol = Protocol::new();

        let read_loop_handle = tokio::spawn(async move {
            // maybe wait_for_response?
            protocol.connected().await;
            loop {
                select! {
                    res = protocol.process_read() => {},
                    _ = shutdown_tx.closed() => break,
                }
            }
        });
        let write_loop_handle = tokio::spawn(async move {
            protocol.connected().await;
            loop {
                select! {
                    // Determine write timing
                    _ = protocol.process_write() => {},
                    _ = shutdown_tx.closed() => break,
                }
            }
        });
        let cmd_loop_handle = tokio::spawn(async move {
            protocol.connected().await;
            loop {
                select! {
                    // 흠... cmd를 통째로 받기 보다는 여기서 cmd를 받아서 protocol의 x 함수를 호출하는 식으로 해도 될 듯
                    // protocol.send_controller_state() 같은...
                    _ = protocol.process_cmd() => {},
                    _ = shutdown_tx.closed() => break,
                }
            }
        });

        let connection_handle = tokio::spawn(async move {
            // spawn this
            protocol.write_empty_report();

            // then wait
            protocol.wait_for_connection();
        });

        join!(run_loop_handle, write_loop_handle, connection_handle);

        transport.pause().await;
        drop(transport_handle);

        self.transport.closed().await;
    }
}
