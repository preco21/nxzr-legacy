pub use crate::controller::protocol::ControllerProtocolConfig as ProtocolConfig;
pub use crate::controller::protocol::TransportRead;
pub use crate::controller::protocol::TransportWrite;
use crate::controller::protocol::{
    self, ControllerProtocol, ControllerProtocolConfig, ControllerProtocolError,
};
use crate::controller::state::ControllerState;
use crate::event::{setup_event, EventError};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use strum::{Display, IntoStaticStr};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time;

#[derive(Clone, Error, Debug)]
pub enum ProtocolError {
    #[error("protocol is being closed, aborting the requested action, action: {0}")]
    ActionAbortedDueToClosing(String),
    #[error("internal error: {0}")]
    Internal(ProtocolInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum ProtocolInternalError {
    #[error("task join failed: {0}")]
    JoinError(String),
    #[error("event: {0}")]
    Event(EventError),
    #[error("controller protocol: {0}")]
    ControllerProtocol(ControllerProtocolError),
}

impl From<EventError> for ProtocolError {
    fn from(err: EventError) -> Self {
        Self::Internal(ProtocolInternalError::Event(err))
    }
}

impl From<ControllerProtocolError> for ProtocolError {
    fn from(err: ControllerProtocolError) -> Self {
        Self::Internal(ProtocolInternalError::ControllerProtocol(err))
    }
}

pub trait Transport:
    TransportRead + TransportWrite + TransportPause + Clone + Send + Sync + 'static
{
}

pub trait TransportPause {
    fn pause(&self);
}

#[derive(Debug)]
pub struct Protocol {
    protocol: Arc<ControllerProtocol>,
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

#[derive(Debug)]
pub(crate) struct StateSendReq {
    ready_tx: oneshot::Sender<()>,
}

impl Protocol {
    pub fn connect(
        transport: impl Transport,
        config: ControllerProtocolConfig,
    ) -> Result<(Self, ProtocolHandle), ProtocolError> {
        let protocol = Arc::new(ControllerProtocol::new(config)?);
        let (close_tx, close_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let (internal_close_tx, mut internal_close_rx) = broadcast::channel(1);
        let (state_send_tx, state_send_rx) = mpsc::channel(1);
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        let mut set = JoinSet::<Result<(), ProtocolError>>::new();
        // Setup protocol events relay
        let events_relay_fut = create_task(
            ProtocolTask::setup_events_relay(protocol.clone(), msg_tx.clone()),
            internal_close_tx.clone(),
        );
        // Setup protocol reader task
        let protocol_reader_fut = create_task(
            ProtocolTask::setup_reader(transport.clone(), protocol.clone()),
            internal_close_tx.clone(),
        );
        // Setup protocol writer task
        let protocol_writer_fut = create_task(
            ProtocolTask::setup_writer(transport.clone(), protocol.clone(), state_send_rx),
            internal_close_tx.clone(),
        );
        // Setup protocol connection handler
        let protocol_conn_handler_fut = {
            let transport = transport.clone();
            let protocol = protocol.clone();
            let mut internal_close_rx = internal_close_tx.subscribe();
            async move {
                let (connected_tx, connected_rx) = mpsc::channel::<()>(1);
                let empty_report_sender = {
                    let protocol = protocol.clone();
                    tokio::spawn(async move {
                        // Send empty input reports up to 10 times until the host decides to reply.
                        for _ in 0..10 {
                            tokio::select! {
                                res = protocol.send_empty_input_report(&transport) => {
                                    // Propagate errors immediately to the caller.
                                    res?;
                                    time::sleep(Duration::from_millis(1000)).await;
                                },
                                _ = connected_tx.closed() => break,
                            }
                        }
                        Result::<(), ProtocolError>::Ok(())
                    })
                };
                tokio::select! {
                    _ = protocol.wait_for_connection() => {},
                    _ = internal_close_rx.recv() => {},
                }
                // Notifies the sender task to close then wait for it until closed.
                drop(connected_rx);
                empty_report_sender.await.map_err(|err| {
                    ProtocolError::Internal(ProtocolInternalError::JoinError(err.to_string()))
                })??;
                Result::<(), ProtocolError>::Ok(())
            }
        };
        set.spawn(events_relay_fut);
        set.spawn(protocol_reader_fut);
        set.spawn(protocol_writer_fut);
        set.spawn(protocol_conn_handler_fut);
        let (term_tx, term_rx) = mpsc::channel(1);
        // Graceful shutdown
        tokio::spawn({
            let transport = transport.clone();
            let internal_close_tx = internal_close_tx.clone();
            let msg_tx = msg_tx.clone();
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
                let _ = msg_tx.send(Event::Log(LogType::Closing));
                // This will allow action methods to get notified when the
                // actual close happens on any of shutdown signals received.
                drop(term_rx);
            }
        });
        // Task coordinating and handling errors
        tokio::spawn(async move {
            while let Some(res) = set.join_next().await {
                match res {
                    Ok(Err(err)) => {
                        let _ = msg_tx.send(Event::Critical(err));
                        // Tell all other tasks to close due to other errors.
                        let _ = internal_close_tx.send(());
                    }
                    Err(err) => {
                        // Notify the caller that there's join error occurred,
                        // so that they can choose what to do next.
                        //
                        // Note that this kind of errors is normally not recoverable.
                        let _ = msg_tx.send(Event::Critical(ProtocolError::Internal(
                            ProtocolInternalError::JoinError(err.to_string()),
                        )));
                        // Tell all other tasks to close due to join error.
                        let _ = internal_close_tx.send(());
                    }
                    _ => {}
                }
            }
            // Mark the protocol is fully closed.
            drop(closed_rx);
            let _ = msg_tx.send(Event::Log(LogType::Closed));
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
    ) -> Result<(), ProtocolError> {
        self.protocol.writer_ready().await;
        let (ready_tx, ready_rx) = oneshot::channel();
        let fut = async {
            self.protocol.modify_controller_state(f).await;
            let _ = self.state_send_tx.send(StateSendReq { ready_tx }).await;
            let _ = ready_rx.await;
        };
        tokio::select! {
            _ = fut => {}
            _ = self.term_tx.closed() => {
                return Err(ProtocolError::ActionAbortedDueToClosing("update_controller_state".to_owned()))
            },
        }
        Ok(())
    }

    // Listen for the protocol control events.
    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>, ProtocolError> {
        Event::subscribe(&mut self.event_sub_tx.clone())
            .await
            .map_err(|err| ProtocolError::from(err))
    }

    // Wait for the internal tasks to exit completely.
    pub fn closed(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
    }
}

pub(crate) struct ProtocolTask {}

impl ProtocolTask {
    pub async fn setup_events_relay(
        protocol: Arc<ControllerProtocol>,
        msg_tx: mpsc::UnboundedSender<Event>,
    ) -> Result<(), ProtocolError> {
        let mut rx = protocol.events().await?;
        loop {
            if let Some(orig) = rx.recv().await {
                let evt = match orig {
                    protocol::Event::Error(err) => Event::Warning(err.into()),
                    protocol::Event::Log(log) => Event::Log(LogType::ControllerProtocol(log)),
                };
                let _ = msg_tx.send(evt);
            }
        }
    }

    pub async fn setup_reader(
        transport: impl Transport,
        protocol: Arc<ControllerProtocol>,
    ) -> Result<(), ProtocolError> {
        loop {
            protocol.process_read(&transport).await?;
        }
    }

    pub async fn setup_writer(
        transport: impl Transport,
        protocol: Arc<ControllerProtocol>,
        mut ctrl_state_send_req_rx: mpsc::Receiver<StateSendReq>,
    ) -> Result<(), ProtocolError> {
        protocol.writer_ready().await;
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
    fut: impl Future<Output = Result<(), ProtocolError>>,
    close_tx: broadcast::Sender<()>,
) -> impl Future<Output = Result<(), ProtocolError>> {
    let mut close_rx = close_tx.subscribe();
    async move {
        // All tasks are managed by `JoinSet` on from the caller. However, in
        // order to handle graceful shutdown, we leveraged `close_rx` signal to
        // let each task to finish on their own.
        //
        // If there's any of errors including `JoinError` and
        // ControllerProtocolError, etc... is raised, `close_rx` will receive a
        // signal to escape the running task.
        tokio::select! {
            res = fut => res,
            _ = close_rx.recv() => Result::<(), ProtocolError>::Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Log(LogType),
    Warning(ProtocolError),
    Critical(ProtocolError),
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Log(log) => write!(f, "event log: {:?}", log),
            Self::Warning(err) => write!(f, "event warn: {}", err.to_string()),
            Self::Critical(err) => write!(f, "event critical: {}", err.to_string()),
        }
    }
}

#[derive(Clone, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum LogType {
    Closing,
    Closed,
    ControllerProtocol(protocol::LogType),
}

#[derive(Debug)]
pub struct SubscriptionReq {
    tx: mpsc::UnboundedSender<Event>,
    ready_tx: oneshot::Sender<()>,
}

impl Event {
    setup_event!();
}
