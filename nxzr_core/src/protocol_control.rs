use crate::controller::protocol::{Protocol, ProtocolConfig, TransportCombined};
use crate::{Error, ErrorKind, Result};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;

pub trait Transport: TransportCombined + Clone + Send + Sync + 'static {
    fn pause(&self);
}

#[derive(Debug)]
pub(crate) struct ControllerStateSendReq {
    ready_tx: oneshot::Sender<()>,
}

#[derive(Debug)]
pub struct ProtocolControl {
    protocol: Arc<Protocol>,
    closed_tx: mpsc::Sender<()>,
}

pub struct ProtocolHandle {
    _close_rx: mpsc::Receiver<()>,
}

impl Drop for ProtocolHandle {
    fn drop(&mut self) {
        // Required for drop order
    }
}

impl ProtocolControl {
    pub fn connect(
        transport: impl Transport,
        config: ProtocolConfig,
    ) -> Result<(Self, ProtocolHandle)> {
        let protocol = Arc::new(Protocol::new(config)?);
        let (close_tx, close_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let (internal_close_tx, _) = broadcast::channel(1);
        let (ctrl_state_send_req_tx, mut ctrl_state_send_req_rx) = mpsc::channel(1);
        let mut set = JoinSet::<Result<()>>::new();
        // Setup protocol reader task
        set.spawn(create_task(
            ProtocolControlTask::setup_reader(transport.clone(), protocol.clone()),
            internal_close_tx.clone(),
            internal_close_tx.subscribe(),
        ));
        // Setup protocol writer task
        set.spawn(create_task(
            ProtocolControlTask::setup_writer(
                transport.clone(),
                protocol.clone(),
                ctrl_state_send_req_rx,
            ),
            internal_close_tx.clone(),
            internal_close_tx.subscribe(),
        ));
        // Setup protocol connection handler
        set.spawn(create_task(
            ProtocolControlTask::setup_writer(
                transport.clone(),
                protocol.clone(),
                ctrl_state_send_req_rx,
            ),
            internal_close_tx.clone(),
            internal_close_tx.subscribe(),
        ));
        // Task cleanup handling
        tokio::spawn({
            let transport = transport.clone();
            let internal_close_tx = internal_close_tx.clone();
            let mut internal_close_rx = internal_close_tx.clone().subscribe();
            async move {
                tokio::select! {
                    _ = close_tx.closed() => {},
                    _ = internal_close_rx.recv() => {},
                }
                transport.pause();
                let _ = internal_close_tx.send(());
                // FIXME: Send if there's any error stuff...
                while let Some(_) = set.join_next().await {}
                drop(closed_rx);
            }
        });
        protocol.establish_connection();
        Ok((
            Self {
                protocol,
                closed_tx,
            },
            ProtocolHandle {
                _close_rx: close_rx,
            },
        ))
    }

    pub fn send_controller_state(&self) {
        // Handler for `ControllerState` updates
        // let protocol = protocol.clone();
        // let will_close_tx = will_close_tx.clone();
        // let mut will_close_rx = will_close_tx.subscribe();
        // handles.push(tokio::spawn(async move {
        //     protocol.ready_for_write().await;
        //     loop {
        //         let (ready_tx, ready_rx) = oneshot::channel();
        //         let fut = async {
        //             // FIXME: Test
        //             protocol
        //                 .modify_controller_state(|state| {
        //                     if let Err(err) = state
        //                         .button_state_mut()
        //                         .set_button(crate::controller::state::button::ButtonKey::A, true)
        //                     {
        //                         println!("{}", err);
        //                     };
        //                 })
        //                 .await;
        //             controller_state_req_tx
        //                 .send(ControllerStateReq { ready_tx })
        //                 .await
        //                 .unwrap();
        //             ready_rx.await.unwrap();
        //         };
        //         tokio::select! {
        //             _ = fut => {}
        //             _ = will_close_rx.recv() => break,
        //         }
        //     }
        // }));
    }

    pub fn events(&self) {
        // TODO: 로그는 함수로 빼는게 나을 것 같은데? .events() 그대로 expose
        // {
        //     // Run logger thread.
        //     let protocol = protocol.clone();
        //     handles.push(tokio::spawn(async move {
        //         loop {
        //             tokio::select! {
        //                 event = protocol.events() => {
        //                     // Revisit
        //                     println!("{:?}", event);
        //                 },
        //                 _ = close_tx.closed() => break,
        //             }
        //         }
        //     }));
        // }
    }

    // 기본적으로 얘가 Err, Ok 같은거 다뤄 줘야 함 + 그럼에도, shutdown 시그널에 의해
    // spawn 여기에서 하는 것도 okz
    // handle return도 ok
    // 만약 에러 발생하면 여기서 터쳐야
    // 에러 발생했을 때는 transport pause 순서 중요하지 않음...
    // 터지면 transport pause 후 날려버려야

    // FIXME: 얘 밖으로 뺴기
    // 얘는 transport만 없으면 여러번 호출될 수 있음, 근데 있으면 터짐
    pub async fn run(&self) {
        // halt -> 일단 멈춤
        // closed -> closed 상태 표기?

        // ^^^ 일단 이 상태가 휘발성 이어야 함
        // must sync with absence of transport
        // should really we store this in inner state...?

        // 애초에 루프에서 while transport.alive() 같은 느낌으로... 처리하면 될 것 같은데

        // TODO: 이 함수가 background task를 spawn도 하면서, shutdown 시그널도 받고, shutdown 할 수 있는 handle도 반환하고, shutdown 시그널을 내보낼 수도 있어야 함

        // Here we've used terminal channels for shutdown-handling because it's
        // more versatile and reliable than just polling `None` transport with
        // something like, e.g. `while let Some(t) = inner.transport() {}`.
        //
        // This is more plausible since just polling the `None` variant will not
        // break the running task handle while tasks in tokio::select! are still
        // running.
        //
        // Also, we don't directly use transport's closing signals (e.g.
        // `t.closing()`, `t.closed()`) as we need to decouple the logic in
        // different contexts and streamline shutdown signal handling.

        // FIXME: Ping to start writer thread -- ready_for_write() 기다렸다가 시작하기 write는

        // FIXME: protocol.process_read(), protocol.process_write() 시 error 처리하기
        // {
        //     // Handles shutdown sequence.
        //     let inner = self.inner.clone();
        //     handles.push(tokio::spawn(async move {
        //         tokio::select! {
        //             // _ = inner.transport().closed() => {

        //             // }
        //             _ = close_tx.closed() => {}

        //         }

        //         // TODO: call will close to shutdown all the handles

        //         // ㄴ No, put halt_tx in inner, call close_connection() to set it.
        //         // to streamline
        //     }));
        // }

        // ^^^ 애초에 이 spawn 로직을 new로 옮기고, started_tx로 이걸 처리할까?

        // graceful shutdown 처리
        // 1. transport.pause 처리 여기에서 해야 함 (handle 날릴때, 내부 에러 발생시 둘 다)
        // 2. 내부 에러 발생시 handle 자동으로 날리게 해야 하나 고민이네...
        // 3. 외부에다가 signal 주고 받고 하는거 너무 좀 플로우가...
        // 애초에 handle이 필요한지 모르겠어

        // check for join errors then if there's an error return Err()
        //
    }
}

pub(crate) struct ProtocolControlTask {}

impl ProtocolControlTask {
    pub async fn setup_reader(transport: impl Transport, protocol: Arc<Protocol>) -> Result<()> {
        loop {
            protocol.process_read(&transport).await?;
        }
    }

    pub async fn setup_writer(
        transport: impl Transport,
        protocol: Arc<Protocol>,
        mut ctrl_state_send_req_rx: mpsc::Receiver<ControllerStateSendReq>,
    ) -> Result<()> {
        protocol.ready_for_write().await;
        loop {
            let ctrl_state_ready_tx = match ctrl_state_send_req_rx.try_recv() {
                Ok(ControllerStateSendReq { ready_tx }) => Some(async move {
                    let _ = ready_tx.send(());
                }),
                Err(_) => None,
            };
            protocol
                .process_write(&transport, ctrl_state_ready_tx)
                .await?;
        }
    }

    pub async fn setup_connection_handler(
        transport: impl Transport,
        protocol: Arc<Protocol>,
    ) -> Result<()> {
        let (connected_tx, connected_rx) = mpsc::channel::<()>(1);
        // 이것만 따로 때서 만들어야 할까?
        let empty_report_sender = {
            let protocol = protocol.clone();
            tokio::spawn(async move {
                // Send empty input reports for 10 times until the host decides to reply.
                for _ in 0..10 {
                    tokio::select! {
                        // FIXME: interval로 바꿔?
                        _ = protocol.send_empty_input_report(&transport) => {},
                        _ = connected_tx.closed() => break,
                    }
                }
            })
        };
        protocol.wait_for_connection().await;
        // FIXME: how to make sure this to be called always?
        drop(connected_rx);
        empty_report_sender.await.unwrap();
        Ok(())
    }
}

fn create_task(
    fut: impl Future<Output = Result<()>>,
    close_tx: broadcast::Sender<()>,
    mut close_rx: broadcast::Receiver<()>,
) -> impl Future<Output = Result<()>> {
    async move {
        tokio::select! {
            res = fut => {
                match res {
                    Ok(_) => {},
                    Err(err) => {
                        let _ = close_tx.send(());
                        return Err(err)
                    }
                }
            },
            _ = close_rx.recv() => {},
        }
        Ok(())
    }
}
