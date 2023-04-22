use crate::event::{setup_event, EventError};
use crate::semaphore::BoundedSemaphore;
use crate::sock::hci;
use bluer::l2cap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinSet;
use tokio::time::sleep;

const DEFAULT_FLOW_CONTROL_PERMITS: usize = 4;
const DEFAULT_READ_BUF_SIZE: usize = 50;

#[derive(Clone, Error, Debug)]
pub enum TransportError {
    #[error("operation called when transport is closed")]
    OperationWhileClosed,
    #[error("failed to receive packets from `monitor lock` hci socket")]
    MonitorLock,
    #[error("failed to receive packets from `monitor window` hci socket")]
    MonitorWindow,
    #[error("internal error: {0}")]
    Internal(TransportInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum TransportInternalError {
    #[error("io: {0}")]
    Io(std::io::ErrorKind),
    #[error("event: {0}")]
    Event(EventError),
    #[error("semaphore acquire failed")]
    SemaphoreFailed,
}

impl From<std::io::Error> for TransportError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(TransportInternalError::Io(err.kind()))
    }
}

impl From<EventError> for TransportError {
    fn from(err: EventError) -> Self {
        Self::Internal(TransportInternalError::Event(err))
    }
}

impl From<tokio::sync::AcquireError> for TransportError {
    fn from(_: tokio::sync::AcquireError) -> Self {
        Self::Internal(TransportInternalError::SemaphoreFailed)
    }
}

#[derive(Debug, Default)]
pub struct TransportConfig {
    num_flow_control: Option<usize>,
    read_buf_size: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Transport {
    inner: Arc<TransportInner>,
}

impl Transport {
    pub async fn register(
        itr_sock: l2cap::SeqPacket,
        ctl_sock: l2cap::SeqPacket,
        config: TransportConfig,
    ) -> Result<(Self, TransportHandle), TransportError> {
        let (close_tx, close_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let inner = Arc::new(TransportInner::new(itr_sock, ctl_sock, config, closed_tx).await?);
        let mut set = JoinSet::new();
        // Handle writer lock timing.
        set.spawn({
            let inner = inner.clone();
            async move {
                loop {
                    if let Err(err) = inner.monitor_lock().await {
                        let _ = inner.dispatch_event(Event::Error(err));
                    }
                }
            }
        });
        // Handle writer window timing.
        set.spawn({
            let inner = inner.clone();
            async move {
                loop {
                    if let Err(err) = inner.monitor_window().await {
                        let _ = inner.dispatch_event(Event::Error(err));
                    }
                }
            }
        });
        tokio::spawn({
            let inner = inner.clone();
            async move {
                close_tx.closed().await;
                // Generally, it's recommended to pause from caller before it
                // gets ended up here. We are assuming the user may not be able
                // to `.pause()` it anyway.
                inner.pause();
                set.shutdown().await;
                drop(closed_rx);
            }
        });
        Ok((
            Self { inner },
            TransportHandle {
                _close_rx: close_rx,
            },
        ))
    }

    // We are exposing `Result<T, std::io::Error>` type here in order to
    // conveniently interoperate with `ProtocolControl` from `nxzr_core`.
    pub async fn read(&self) -> Result<&[u8], std::io::Error> {
        self.inner
            .read()
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "transport error"))
    }

    // We are exposing `Result<T, std::io::Error>` type here in order to
    // conveniently interoperate with `ProtocolControl` from `nxzr_core`.
    pub async fn write(&self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.inner
            .write(buf)
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "transport error"))
    }

    pub async fn pause(&self) {
        self.inner.pause();
    }

    pub async fn resume(&self) {
        self.inner.resume();
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>, TransportError> {
        self.inner.events().await
    }

    pub fn closed(&self) -> impl Future<Output = ()> {
        self.inner.closed()
    }
}

impl AsRef<TransportInner> for Transport {
    fn as_ref(&self) -> &TransportInner {
        &self.inner
    }
}

pub struct TransportHandle {
    _close_rx: mpsc::Receiver<()>,
}

impl Drop for TransportHandle {
    fn drop(&mut self) {
        // Required for drop order
    }
}

#[derive(Debug)]
pub(crate) struct TransportInner {
    write_window: hci::Datagram,
    write_lock: hci::Datagram,
    itr_sock: l2cap::SeqPacket,
    ctl_sock: l2cap::SeqPacket,
    running_tx: watch::Sender<bool>,
    writing_tx: watch::Sender<bool>,
    write_sem: Arc<BoundedSemaphore>,
    read_buf_size: usize,
    closed_tx: mpsc::Sender<()>,
    event_sub_tx: mpsc::Sender<SubscriptionReq>,
    msg_tx: mpsc::UnboundedSender<Event>,
}

impl TransportInner {
    pub async fn new(
        itr_sock: l2cap::SeqPacket,
        ctl_sock: l2cap::SeqPacket,
        config: TransportConfig,
        closed_tx: mpsc::Sender<()>,
    ) -> Result<Self, TransportError> {
        // Device ids must be targeting to the local machine.
        let write_window = hci::Datagram::bind(hci::SocketAddr { dev_id: 0 }).await?;
        // 0x04 = HCI_EVT; 0x13 = Number of completed packets
        write_window.as_ref().set_filter(hci::Filter {
            type_mask: 1 << 0x04,
            event_mask: [1 << 0x13, 0],
            opcode: 0,
        })?;
        let write_lock = hci::Datagram::bind(hci::SocketAddr { dev_id: 0 }).await?;
        // 0x04 = HCI_EVT; 0x1b = Max slots change
        write_lock.as_ref().set_filter(hci::Filter {
            type_mask: 1 << 0x04,
            event_mask: [1 << 0x1b, 0],
            opcode: 0,
        })?;
        let num_flow_control = config
            .num_flow_control
            .unwrap_or(DEFAULT_FLOW_CONTROL_PERMITS);
        let read_buf_size = config.read_buf_size.unwrap_or(DEFAULT_READ_BUF_SIZE);
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        Ok(Self {
            write_window,
            write_lock,
            itr_sock,
            ctl_sock,
            running_tx: watch::channel(true).0,
            writing_tx: watch::channel(true).0,
            write_sem: Arc::new(BoundedSemaphore::new(num_flow_control, num_flow_control)),
            read_buf_size,
            closed_tx,
            event_sub_tx,
            msg_tx,
        })
    }

    async fn monitor_window(&self) -> Result<(), TransportError> {
        let mut buf = vec![0; 10 as _];
        self.write_window.recv(&mut buf).await?;
        let permits: u16 = u16::from(buf[6]) + u16::from(buf[7]) * 0x100;
        let _ = self.write_sem.add_permits(permits as usize);
        Ok(())
    }

    async fn monitor_lock(&self) -> Result<(), TransportError> {
        let mut buf = vec![0; 10 as _];
        self.write_lock.recv(&mut buf).await?;
        if buf[5] < 5 {
            self.pause_write();
            sleep(Duration::from_millis(1000)).await;
            self.resume_write();
        }
        Ok(())
    }

    pub async fn read(&self) -> Result<&[u8], TransportError> {
        if self.is_closed() {
            return Err(TransportError::OperationWhileClosed);
        }
        self.running().await;
        // TODO: ITR read
        Ok(&[])
    }

    pub async fn write(&self, buf: &[u8]) -> Result<(), TransportError> {
        if self.is_closed() {
            return Err(TransportError::OperationWhileClosed);
        }
        self.running().await;
        self.write_sem.acquire_forget().await?;
        self.writable().await;
        self.itr_sock.send(buf).await?;
        Ok(())
    }

    pub async fn running(&self) {
        let mut rx = self.running_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    pub fn pause(&self) {
        self.running_tx.send_replace(false);
    }

    pub fn resume(&self) {
        self.running_tx.send_replace(true);
    }

    pub async fn writable(&self) {
        let mut rx = self.writing_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    pub fn pause_write(&self) {
        self.writing_tx.send_replace(false);
    }

    pub fn resume_write(&self) {
        self.writing_tx.send_replace(true);
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>, TransportError> {
        Ok(Event::subscribe(&mut self.event_sub_tx.clone()).await?)
    }

    pub fn dispatch_event(&self, event: Event) {
        let _ = self.msg_tx.send(event);
    }

    fn is_closed(&self) -> bool {
        self.closed_tx.is_closed()
    }

    pub fn closed(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Error(TransportError),
}

#[derive(Debug)]
pub struct SubscriptionReq {
    tx: mpsc::UnboundedSender<Event>,
    ready_tx: oneshot::Sender<()>,
}

impl Event {
    setup_event!();
}
