use crate::controller::{
    protocol::{
        ControllerProtocol, ControllerProtocolError, Event as ProtocolEvent,
        LogType as ProtocolLogType,
    },
    report::subcommand::Subcommand,
    state::ControllerState,
};
use nxzr_shared::{
    event::{EventError, SubscriptionReq},
    setup_event,
};
use std::future::Future;
use std::sync::Arc;
use strum::{Display, IntoStaticStr};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time::{self, Duration};

// Re-exports subset of internal protocol module exports.
pub use crate::controller::protocol::{
    ControllerProtocolConfig as ProtocolConfig, TransportRead, TransportWrite,
};

#[derive(Clone, Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("protocol is being closed, aborting the requested action")]
    ActionAbortedDueToClosing,
    #[error("internal error: {0}")]
    Internal(ProtocolInternalError),
}

#[derive(Clone, Debug, thiserror::Error)]
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

#[derive(Debug, Clone)]
pub struct Protocol {
    inner: Arc<ProtocolInner>,
}

impl Protocol {
    pub async fn connect(
        transport: impl Transport,
        config: ProtocolConfig,
    ) -> Result<(Self, ProtocolHandle), ProtocolError> {
        let (closing_tx, closing_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let (sig_close_tx, mut sig_close_rx) = broadcast::channel(1);

        let (state_send_tx, state_send_rx) = mpsc::channel(1);

        let (msg_tx, msg_rx) = mpsc::channel(256);
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;

        let inner = Arc::new(ProtocolInner::new(
            config,
            state_send_tx,
            event_sub_tx,
            closing_tx,
            closed_tx,
        )?);

        // Relay inner protocol events.
        //
        // Please note that, this task must survive even after the protocol
        // tasks panic, so it can notify the caller what went wrong by sending
        // the events that has raised from the protocol handler itself.
        let mut inner_event_rx = inner.protocol.events().await?;
        tokio::spawn({
            let msg_tx = msg_tx.clone();
            async move {
                loop {
                    if let Some(orig) = inner_event_rx.recv().await {
                        let evt = match orig {
                            ProtocolEvent::Warning(err) => Event::Warning(err.into()),
                            ProtocolEvent::Log(log) => Event::Log(log.into()),
                        };
                        let _ = msg_tx.try_send(evt);
                    }
                }
            }
        });

        // Although we use [JoinSet] which is already capable of managing
        // shutdown for all tasks that belongs to, it will not handle graceful
        // shutdown. So, we're employing another close signal to let each task
        // to finish it on their own.
        //
        // When there's an critical error including [JoinError],
        // [ControllerProtocolError] and etc... is raised, a close signal is
        // sent and each task will receive the signal, so that they will escape
        // the running task.
        let mut set = JoinSet::<Result<(), ProtocolError>>::new();
        // Setup protocol reader task
        let reader_fut = {
            let mut sig_close_rx = sig_close_tx.subscribe();
            let fut = setup_reader(inner.protocol.clone(), transport.clone());
            async move {
                tokio::select! {
                    res = fut => res,
                    _ = sig_close_rx.recv() => return Ok::<(), ProtocolError>(()),
                }
            }
        };
        // Setup protocol writer task
        let writer_fut = {
            let mut sig_close_rx = sig_close_tx.subscribe();
            let fut = setup_writer(inner.protocol.clone(), transport.clone(), state_send_rx);
            async move {
                tokio::select! {
                    res = fut => res,
                    _ = sig_close_rx.recv() => return Ok::<(), ProtocolError>(()),
                }
            }
        };
        // Setup protocol connection handler
        let conn_handler_fut = {
            let transport = transport.clone();
            let controller_protocol = inner.protocol.clone();
            async move {
                let (connected_tx, connected_rx) = mpsc::channel::<()>(1);
                // Please note that sending blank reports after the initial
                // connection until the host to finish reply (at `writer_ready`
                // point) is very important because otherwise, the host will not
                // send any further responses after last sending `spi_read`
                // command.
                let blank_report_sender = tokio::spawn(setup_blank_report_sender(
                    controller_protocol.clone(),
                    transport,
                    connected_tx,
                ));
                tokio::select! {
                    _ = controller_protocol.writer_ready() => {
                        // Allow the task to send one last command.
                        time::sleep(Duration::from_millis(1000)).await;
                    },
                    _ = sig_close_rx.recv() => {},
                }
                // Notifies the sender task to close then wait for it until closed.
                drop(connected_rx);
                blank_report_sender.await.map_err(|err| {
                    ProtocolError::Internal(ProtocolInternalError::JoinError(err.to_string()))
                })??;
                Ok::<(), ProtocolError>(())
            }
        };
        set.spawn(reader_fut);
        set.spawn(writer_fut);
        set.spawn(conn_handler_fut);

        // Close handling and graceful shutdown
        let (close_tx, close_rx) = mpsc::channel(1);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    res = set.join_next() => {
                        // When all tasks in set are closed ok, break the loop.
                        let Some(inner) = res else {
                            break;
                        };
                        match inner {
                            Ok(Err(err)) => {
                                let _ = msg_tx.try_send(Event::Error(err));
                                let _ = sig_close_tx.send(());
                                break;
                            }
                            Err(err) => {
                                // [JoinError] is usually occurred when there's panic in spawned tasks in the set.
                                // In such case, we immediately abort the protocol tasks then bail.
                                let _ = msg_tx.try_send(Event::Error(ProtocolError::Internal(
                                    ProtocolInternalError::JoinError(err.to_string()),
                                )));
                                let _ = sig_close_tx.send(());
                                break;
                            }
                            _ => {}
                        }
                    },
                    _ = close_tx.closed() => {
                        let _ = sig_close_tx.send(());
                        break;
                    },
                }
            }
            let _ = msg_tx.try_send(Event::Log(LogType::Closing));
            // Trigger a pause for transport.
            transport.pause();
            // Drop the `closing_rx` so that, relevant action methods to get
            // notified when closing happens.
            drop(closing_rx);
            // Wait for the rest of the tasks to finish.
            while let Some(res) = set.join_next().await {
                match res {
                    Ok(Err(err)) => {
                        let _ = msg_tx.try_send(Event::Error(err));
                    }
                    Err(err) => {
                        // Notify the caller that there's join error occurred,
                        // so that they can choose what to do next.
                        //
                        // Note that this kind of errors is normally not recoverable.
                        let _ = msg_tx.try_send(Event::Error(ProtocolError::Internal(
                            ProtocolInternalError::JoinError(err.to_string()),
                        )));
                    }
                    _ => {}
                }
            }
            let _ = msg_tx.try_send(Event::Log(LogType::Closed));
            // Mark the protocol is fully closed.
            drop(closed_rx);
        });

        // Mark connection established.
        inner.protocol.establish_connection();

        Ok((
            Self { inner },
            ProtocolHandle {
                _close_rx: close_rx,
            },
        ))
    }

    // Update controller state in-place and wait for it to complete.
    pub async fn update_controller_state<T>(
        &self,
        f: impl FnOnce(&mut ControllerState) -> T,
    ) -> Result<T, ProtocolError> {
        self.inner.update_controller_state(f).await
    }

    // Listen for the protocol control events.
    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>, ProtocolError> {
        self.inner.events().await
    }

    // Wait for the internal tasks to exit completely.
    pub fn closed(&self) -> impl Future<Output = ()> {
        self.inner.closed()
    }
}

#[derive(Debug)]
pub(crate) struct ProtocolInner {
    pub protocol: Arc<ControllerProtocol>,
    state_send_tx: mpsc::Sender<StateSendReq>,
    event_sub_tx: mpsc::Sender<SubscriptionReq<Event>>,
    closing_tx: mpsc::Sender<()>,
    closed_tx: mpsc::Sender<()>,
}

#[derive(Debug)]
pub(crate) struct StateSendReq {
    ready_tx: oneshot::Sender<()>,
}

impl ProtocolInner {
    pub fn new(
        config: ProtocolConfig,
        state_send_tx: mpsc::Sender<StateSendReq>,
        event_sub_tx: mpsc::Sender<SubscriptionReq<Event>>,
        closing_tx: mpsc::Sender<()>,
        closed_tx: mpsc::Sender<()>,
    ) -> Result<Self, ProtocolError> {
        let protocol = Arc::new(ControllerProtocol::new(config)?);
        Ok(Self {
            protocol,
            state_send_tx,
            closing_tx,
            closed_tx,
            event_sub_tx,
        })
    }

    pub async fn update_controller_state<T>(
        &self,
        f: impl FnOnce(&mut ControllerState) -> T,
    ) -> Result<T, ProtocolError> {
        self.protocol.writer_ready().await;
        let (ready_tx, ready_rx) = oneshot::channel();
        let fut = async {
            let ret = self.protocol.modify_controller_state(f).await;
            let _ = self.state_send_tx.send(StateSendReq { ready_tx }).await;
            let _ = ready_rx.await;
            ret
        };
        tokio::select! {
            ret = fut => Ok(ret),
            _ = self.closing_tx.closed() => Err(ProtocolError::ActionAbortedDueToClosing),
        }
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>, ProtocolError> {
        Event::subscribe(&mut self.event_sub_tx.clone())
            .await
            .map_err(|err| ProtocolError::from(err))
    }

    pub fn closed(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
    }
}

async fn setup_reader(
    protocol: Arc<ControllerProtocol>,
    transport: impl Transport,
) -> Result<(), ProtocolError> {
    loop {
        protocol.process_read(&transport).await?;
    }
}

async fn setup_writer(
    protocol: Arc<ControllerProtocol>,
    transport: impl Transport,
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

async fn setup_blank_report_sender(
    protocol: Arc<ControllerProtocol>,
    transport: impl Transport,
    connected_tx: mpsc::Sender<()>,
) -> Result<(), ProtocolError> {
    // Send blank input reports up to 10 times until the host decides to reply.
    for _ in 0..10 {
        tokio::select! {
            res = protocol.send_blank_input_report(&transport) => {
                // Propagate errors immediately to the caller if any.
                res?;
                time::sleep(Duration::from_millis(1000)).await;
            },
            _ = connected_tx.closed() => break,
        }
    }
    Ok::<(), ProtocolError>(())
}

pub struct ProtocolHandle {
    _close_rx: mpsc::Receiver<()>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Log(LogType),
    Error(ProtocolError),
    Warning(ProtocolError),
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Log(log) => write!(f, "[log]: {:?}", log),
            Self::Error(err) => write!(f, "[error]: {}", err.to_string()),
            Self::Warning(err) => write!(f, "[warn]: {}", err.to_string()),
        }
    }
}

#[derive(Clone, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum LogType {
    Closing,
    Closed,
    PairingEnded,
    SubcommandReceived(Subcommand),
}

impl From<ProtocolLogType> for LogType {
    fn from(log_type: ProtocolLogType) -> Self {
        match log_type {
            ProtocolLogType::PairingEnded => Self::PairingEnded,
            ProtocolLogType::SubcommandReceived(subcommand) => Self::SubcommandReceived(subcommand),
        }
    }
}

impl Event {
    setup_event!(Event);
}
