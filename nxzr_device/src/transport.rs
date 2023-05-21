use crate::semaphore::BoundedSemaphore;
use crate::session::PairedSession;
use crate::sock::hci;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use std::future::Future;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;
use tokio::time;
use tracing::Instrument;

const DEFAULT_FLOW_CONTROL_PERMITS: usize = 4;
const DEFAULT_READ_BUF_SIZE: usize = 50;

#[derive(Clone, Error, Debug)]
pub enum TransportError {
    #[error("operation called when transport is closed")]
    OperationWhileClosed,
    #[error("hci socket `monitor lock` closed by peer")]
    MonitorLockClosed,
    #[error("hci socket `monitor window` closed by peer")]
    MonitorWindowClosed,
    #[error("reader remote closed by peer")]
    ReaderClosed,
    #[error("writer remote closed by peer")]
    WriterClosed,
    #[error("internal error: {0}")]
    Internal(TransportInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum TransportInternalError {
    #[error("io: {kind}; {message}")]
    Io {
        kind: std::io::ErrorKind,
        message: String,
    },
    #[error("semaphore acquire failed")]
    SemaphoreFailed,
}

impl From<std::io::Error> for TransportError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(TransportInternalError::Io {
            kind: err.kind(),
            message: err.to_string(),
        })
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

impl nxzr_core::protocol::Transport for Transport {}

#[async_trait]
impl nxzr_core::protocol::TransportRead for Transport {
    async fn read(&self) -> std::io::Result<BytesMut> {
        self.read().await
    }
}

#[async_trait]
impl nxzr_core::protocol::TransportWrite for Transport {
    async fn write(&self, buf: Bytes) -> std::io::Result<()> {
        self.write(buf).await
    }
}

impl nxzr_core::protocol::TransportPause for Transport {
    fn pause(&self) {
        self.pause();
    }
}

impl Transport {
    #[tracing::instrument(target = "transport")]
    pub async fn register(
        paired_session: PairedSession,
        config: TransportConfig,
    ) -> Result<(Self, TransportHandle), TransportError> {
        let (close_tx, close_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let inner = Arc::new(TransportInner::new(paired_session, config, closed_tx).await?);
        let mut set = JoinSet::new();
        // Handle writer lock timing.
        set.spawn({
            let inner = inner.clone();
            async move {
                loop {
                    if let Err(err) = inner.monitor_lock().await {
                        tracing::error!("failed to receive data from write lock socket: {}", err);
                    }
                }
            }
            .instrument(tracing::info_span!("transport_worker"))
        });
        // Handle writer window timing.
        set.spawn({
            let inner = inner.clone();
            async move {
                loop {
                    if let Err(err) = inner.monitor_window().await {
                        tracing::error!("failed to receive data from write window socket: {}", err);
                    }
                }
            }
            .instrument(tracing::info_span!("transport_worker"))
        });
        tokio::spawn({
            let inner = inner.clone();
            async move {
                close_tx.closed().await;
                tracing::info!("close signal received, terminating transport.");
                // Generally, it's recommended to pause from caller before it
                // ended up here. We are assuming that the user may not be able
                // to call [TransportInner::pause()] anyway.
                inner.pause();
                set.shutdown().await;
                tracing::info!("transport terminated");
                drop(closed_rx);
            }
            .instrument(tracing::info_span!("transport_worker"))
        });
        Ok((
            Self { inner },
            TransportHandle {
                _close_rx: close_rx,
            },
        ))
    }

    // We are exposing `Result<T, std::io::Error>` type here in order to
    // conveniently interoperate with `Protocol` from `nxzr_core`.
    pub async fn read(&self) -> Result<BytesMut, std::io::Error> {
        self.inner
            .read()
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
    }

    // We are exposing `Result<T, std::io::Error>` type here in order to
    // conveniently interoperate with `Protocol` from `nxzr_core`.
    pub async fn write(&self, buf: Bytes) -> Result<(), std::io::Error> {
        self.inner
            .write(buf)
            .await
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
    }

    pub fn pause(&self) {
        self.inner.pause();
    }

    pub async fn resume(&self) {
        self.inner.resume();
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
    // Control socket must always be dropped with interrupt socket like a pair even if it's unused.
    session: PairedSession,
    running_tx: watch::Sender<bool>,
    writing_tx: watch::Sender<bool>,
    write_sem: Arc<BoundedSemaphore>,
    read_buf_size: usize,
    closed_tx: mpsc::Sender<()>,
}

impl TransportInner {
    #[tracing::instrument]
    pub async fn new(
        paired_session: PairedSession,
        config: TransportConfig,
        closed_tx: mpsc::Sender<()>,
    ) -> Result<Self, TransportError> {
        tracing::info!("initializing a transport.");
        // FIXME: disabled due to invalid value os error 22
        // Reset `SO_SNDBUF` of the given client sockets.
        // paired_session.itr_client().reset_sndbuf()?;
        // paired_session.ctl_client().reset_sndbuf()?;
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
        Ok(Self {
            write_window,
            write_lock,
            session: paired_session,
            running_tx: watch::channel(true).0,
            writing_tx: watch::channel(true).0,
            write_sem: Arc::new(BoundedSemaphore::new(num_flow_control, num_flow_control)),
            read_buf_size,
            closed_tx,
        })
    }

    async fn monitor_window(&self) -> Result<(), TransportError> {
        let mut buf: Vec<u8> = vec![0; 10 as _];
        match self.write_window.recv(&mut buf).await {
            Ok(0) => return Err(TransportError::MonitorWindowClosed),
            Ok(_) => {}
            Err(err) => return Err(err.into()),
        };
        let permits: u16 = u16::from(buf[6]) + u16::from(buf[7]) * 0x100;
        let _ = self.write_sem.add_permits(permits as usize);
        Ok(())
    }

    async fn monitor_lock(&self) -> Result<(), TransportError> {
        let mut buf: Vec<u8> = vec![0; 10 as _];
        match self.write_lock.recv(&mut buf).await {
            Ok(0) => return Err(TransportError::MonitorLockClosed),
            Ok(_) => {}
            Err(err) => return Err(err.into()),
        };
        if buf[5] < 5 {
            self.pause_write();
            time::sleep(time::Duration::from_millis(1000)).await;
            self.resume_write();
        }
        Ok(())
    }

    pub async fn read(&self) -> Result<BytesMut, TransportError> {
        if self.is_closed() {
            return Err(TransportError::OperationWhileClosed);
        }
        self.running().await;
        let mut buf = BytesMut::with_capacity(self.read_buf_size);
        buf.resize(self.read_buf_size, 0);
        match self.session.itr_client().recv(&mut buf).await {
            Ok(0) => Err(TransportError::ReaderClosed),
            Ok(_) => Ok(buf),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn write(&self, buf: Bytes) -> Result<(), TransportError> {
        if self.is_closed() {
            return Err(TransportError::OperationWhileClosed);
        }
        self.running().await;
        self.write_sem.acquire_forget().await?;
        self.writable().await;
        // Writing a buffer in length more than MTU may fail, however, L2CAP's
        // [SeqPacket] socket seems allows writing buf regardless of the MTU length.
        match self.session.itr_client().send(&buf).await {
            Ok(0) => Err(TransportError::WriterClosed),
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
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

    fn is_closed(&self) -> bool {
        self.closed_tx.is_closed()
    }

    pub fn closed(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
    }
}
