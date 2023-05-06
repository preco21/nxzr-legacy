use crate::controller::protocol::{self, Protocol, ProtocolConfig};
use crate::controller::state::ControllerState;
use crate::event::setup_event;
use crate::{Error, Result};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use strum::{Display, IntoStaticStr};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time;

pub trait Transport:
    TransportRead + TransportWrite + TransportPause + Clone + Send + Sync + 'static
{
}

pub use crate::controller::protocol::TransportRead;
pub use crate::controller::protocol::TransportWrite;

pub trait TransportPause {
    fn pause(&self);
}

#[derive(Debug)]
pub(crate) struct StateSendReq {
    ready_tx: oneshot::Sender<()>,
}

#[derive(Debug)]
pub struct ProtocolControl {
    protocol: Arc<Protocol>,
    state_send_tx: mpsc::Sender<StateSendReq>,
    term_tx: mpsc::Sender<()>,
    closed_tx: mpsc::Sender<()>,
    event_sub_tx: mpsc::Sender<SubscriptionReq>,
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
        let (internal_close_tx, mut internal_close_rx) = broadcast::channel(1);
        let (state_send_tx, state_send_rx) = mpsc::channel(1);
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        let mut set = JoinSet::<Result<()>>::new();
        // Setup protocol events relay
        let events_relay_fut = create_task(
            ProtocolControlTask::setup_events_relay(protocol.clone(), msg_tx.clone()),
            internal_close_tx.clone(),
        );
        // Setup protocol reader task
        let protocol_reader_fut = create_task(
            ProtocolControlTask::setup_reader(transport.clone(), protocol.clone()),
            internal_close_tx.clone(),
        );
        // Setup protocol writer task
        let protocol_writer_fut = create_task(
            ProtocolControlTask::setup_writer(transport.clone(), protocol.clone(), state_send_rx),
            internal_close_tx.clone(),
        );
        // Setup protocol connection handler
        let protocol_conn_handler_fut = {
            let transport = transport.clone();
            let protocol = protocol.clone();
            let msg_tx = msg_tx.clone();
            let mut internal_close_rx = internal_close_tx.subscribe();
            async move {
                let (connected_tx, connected_rx) = mpsc::channel::<()>(1);
                let empty_report_sender = {
                    let protocol = protocol.clone();
                    tokio::spawn(async move {
                        // Send empty input reports 10 times up until the host decides to reply.
                        for _ in 0..10 {
                            tokio::select! {
                                res = protocol.send_empty_input_report(&transport) => {
                                    if let Err(err) = res {
                                        // NOTE: Sending empty input report may fail if there's some socket options
                                        // that are misconfigured like `SO_SNDBUF` to `0`.
                                        //
                                        // In such a case, we simply dispatch warnings to events then continue.
                                        let _ = msg_tx.send(Event::Warning(Error::Protocol(err)));
                                    }
                                    time::sleep(Duration::from_millis(1000)).await;
                                },
                                _ = connected_tx.closed() => break,
                            }
                        }
                    })
                };
                tokio::select! {
                    _ = protocol.wait_for_connection() => {},
                    _ = internal_close_rx.recv() => {},
                }
                drop(connected_rx);
                empty_report_sender.await.unwrap();
                Result::<()>::Ok(())
            }
        };
        set.spawn(events_relay_fut);
        set.spawn(protocol_reader_fut);
        set.spawn(protocol_writer_fut);
        set.spawn(protocol_conn_handler_fut);
        let (term_tx, term_rx) = mpsc::channel(1);
        // Task cleanup handling
        tokio::spawn({
            let transport = transport.clone();
            async move {
                tokio::select! {
                    _ = close_tx.closed() => {
                        transport.pause();
                        let _ = internal_close_tx.send(());
                    },
                    _ = internal_close_rx.recv() => {
                        transport.pause();
                    },
                }
                // This will allow the support methods to get notified when the
                // actual close happens on either of shutdown channels resolved.
                drop(term_rx);
                while let Some(res) = set.join_next().await {
                    if let Ok(inner) = res {
                        if let Err(err) = inner {
                            let _ = msg_tx.send(Event::Critical(err));
                        }
                    }
                }
                drop(closed_rx);
            }
        });
        protocol.establish_connection();
        Ok((
            Self {
                protocol,
                term_tx,
                closed_tx,
                event_sub_tx,
                state_send_tx,
            },
            ProtocolHandle {
                _close_rx: close_rx,
            },
        ))
    }

    // Update controller state in-place and wait for it to complete.
    pub async fn update_controller_state(
        &self,
        f: impl FnOnce(&mut ControllerState),
    ) -> Result<()> {
        self.protocol.ready_for_write().await;
        let (ready_tx, ready_rx) = oneshot::channel();
        let fut = async {
            self.protocol.modify_controller_state(f).await;
            let _ = self.state_send_tx.send(StateSendReq { ready_tx }).await;
            let _ = ready_rx.await;
        };
        tokio::select! {
            _ = fut => {}
            _ = self.term_tx.closed() => {},
        }
        Ok(())
    }

    // Listen for the protocol control events.
    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>> {
        Event::subscribe(&mut self.event_sub_tx.clone())
            .await
            .map_err(|err| Error::from(err))
    }

    // Wait for the internal tasks to exit completely.
    pub fn closed(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
    }
}

pub(crate) struct ProtocolControlTask {}

impl ProtocolControlTask {
    pub async fn setup_events_relay(
        protocol: Arc<Protocol>,
        msg_tx: mpsc::UnboundedSender<Event>,
    ) -> Result<()> {
        let mut rx = protocol.events().await?;
        loop {
            if let Some(orig) = rx.recv().await {
                let evt = match orig {
                    protocol::Event::Error(err) => Event::Warning(err.into()),
                    protocol::Event::Log(log) => Event::Log(log),
                };
                let _ = msg_tx.send(evt);
            }
        }
    }

    pub async fn setup_reader(transport: impl Transport, protocol: Arc<Protocol>) -> Result<()> {
        loop {
            protocol.process_read(&transport).await?;
        }
    }

    pub async fn setup_writer(
        transport: impl Transport,
        protocol: Arc<Protocol>,
        mut ctrl_state_send_req_rx: mpsc::Receiver<StateSendReq>,
    ) -> Result<()> {
        protocol.ready_for_write().await;
        loop {
            // Collect all pending waiters before proceed to write for batching.
            let mut pending_subs: Vec<oneshot::Sender<()>> = vec![];
            loop {
                match ctrl_state_send_req_rx.try_recv() {
                    Ok(StateSendReq { ready_tx }) => {
                        pending_subs.push(ready_tx);
                    }
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(_) => {}
                };
            }
            let ready_fut = if !pending_subs.is_empty() {
                Some(async move {
                    for ready_tx in pending_subs {
                        let _ = ready_tx.send(());
                    }
                })
            } else {
                None
            };
            protocol.process_write(&transport, ready_fut).await?;
        }
    }
}

fn create_task(
    fut: impl Future<Output = Result<()>>,
    close_tx: broadcast::Sender<()>,
) -> impl Future<Output = Result<()>> {
    let mut close_rx = close_tx.subscribe();
    async move {
        tokio::select! {
            res = fut => match res {
                Ok(_) => {},
                Err(err) => {
                    let _ = close_tx.send(());
                    return Err(err)
                }
            },
            _ = close_rx.recv() => {},
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Log(protocol::LogType),
    Warning(Error),
    Critical(Error),
}

#[derive(Debug)]
pub struct SubscriptionReq {
    tx: mpsc::UnboundedSender<Event>,
    ready_tx: oneshot::Sender<()>,
}

impl Event {
    setup_event!();
}
