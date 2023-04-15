#[macro_use]
extern crate log;

use nxzr_core::controller::protocol::ProtocolInner;
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
        let protocol = ProtocolInner::new();

        let handle_error = async || {
            transport.pause();
            // log error
            // shutdown_rx
        };

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

        // 위에 로직 다 감싸서 Listener::run() 함수로 만들고 여기선 shutdown_fut + run select 돌리고
        // 여기선 shutdown_rx 날린 후, shutdown_complete_rx(closed 함수로 대체 가능?) await 하게 하는 방법도 있을 듯

        // transport.pause 해야 하는 부분은 cleanup 함수를 받도록 하거나 shutdown_rx 날리면 그때 함께 처리하는 식으로...
        // join_all 부분은 각 스레드에 shutdown_complete_* 를 주거나,

        // 메인 shutdown
        shutdown_fut.await;
        // 받으면 일단 transport pause
        transport.pause();

        // then initiate close sequence by sending shutdown signal to each thread
        drop(shutdown_rx);
        // 또는 .send(())

        // wait for all thread to exit
        // 이건 어째든 필요한데... shutdown_fut랑 함께 select 되어야 함
        join_all([run_loop_handle, write_loop_handle, connection_handle]).await;

        // finally drop transport
        drop(transport_handle);
        // wait for transport to close
        self.transport.closed().await;

        // after this, protocol, transport will drop
    }
}
