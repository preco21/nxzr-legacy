use crate::semaphore::BoundedSemaphore;
use crate::sock::hci;
use crate::{Error, ErrorKind, InternalErrorKind, Result};
use futures::future::join_all;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use strum::{Display, IntoStaticStr};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::sleep;

const DEFAULT_FLOW_CONTROL: usize = 4;
const DEFAULT_READ_BUF_SIZE: usize = 50;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum TransportErrorKind {
    Terminated,
}

#[derive(Clone, Debug, Default)]
pub struct TransportConfig {
    num_flow_control: Option<usize>,
    read_buf_size: Option<usize>,
}

#[derive(Debug)]
pub struct Transport {
    inner: Arc<TransportInner>,
    closed_tx: mpsc::Sender<()>,
    event_sub_tx: mpsc::Sender<SubscriptionReq>,
}

impl Transport {
    pub async fn register(config: TransportConfig) -> Result<(Self, TransportHandle)> {
        let (close_tx, close_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let inner = Arc::new(TransportInner::new(config, close_tx).await?);
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        let mut handles = vec![];
        {
            // Handles writer lock timing.
            let inner = inner.clone();
            let msg_tx = msg_tx.clone();
            handles.push(tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = inner.terminated() => break,
                        res = inner.monitor_lock() => {
                            match res {
                                Ok(()) => {},
                                Err(err) => {
                                    let _ = msg_tx.send(Event::MonitorLockError(err));
                                },
                            }
                        },
                    }
                }
            }));
        }
        {
            // Handles writer window timing.
            let inner = inner.clone();
            let msg_tx = msg_tx.clone();
            handles.push(tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = inner.terminated() => break,
                        res = inner.monitor_window() => {
                            match res {
                                Ok(()) => {},
                                Err(err) => {
                                    let _ = msg_tx.send(Event::MonitorWindowError(err));
                                },
                            }
                        },
                    }
                }
            }));
        }
        tokio::spawn(async move {
            let _ = join_all(handles).await;
            drop(closed_rx);
        });
        Ok((
            Self {
                inner,
                closed_tx,
                event_sub_tx,
            },
            TransportHandle {
                _close_rx: close_rx,
            },
        ))
    }

    pub async fn read(&self) -> Result<&[u8]> {
        self.inner.read().await
    }

    pub async fn write(&self, buf: &[u8]) -> Result<()> {
        self.inner.write(buf).await
    }

    pub async fn pause(&self) {
        self.inner.pause_reading();
        self.inner.pause_writing();
    }

    pub async fn resume(&self) {
        self.inner.resume_reading();
        self.inner.resume_writing();
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>> {
        Event::subscribe(&mut self.event_sub_tx.clone()).await
    }

    pub fn closed(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
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
    reading_tx: watch::Sender<bool>,
    writing_tx: watch::Sender<bool>,
    flow_control: Arc<BoundedSemaphore>,
    read_buf_size: usize,
    term_tx: mpsc::Sender<()>,
}

impl TransportInner {
    pub async fn new(config: TransportConfig, term_tx: mpsc::Sender<()>) -> Result<Self> {
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
        let num_flow_control = match config.num_flow_control {
            Some(num) => num,
            None => DEFAULT_FLOW_CONTROL,
        };
        let read_buf_size = match config.read_buf_size {
            Some(num) => num,
            None => DEFAULT_READ_BUF_SIZE,
        };
        Ok(Self {
            write_window,
            write_lock,
            reading_tx: watch::channel(true).0,
            writing_tx: watch::channel(true).0,
            term_tx,
            flow_control: Arc::new(BoundedSemaphore::new(num_flow_control, num_flow_control)),
            read_buf_size,
        })
    }

    async fn monitor_window(&self) -> Result<()> {
        let mut buf = vec![0; 10 as _];
        self.write_window.recv(&mut buf).await?;
        let permits: u16 = u16::from(buf[6]) + u16::from(buf[7]) * 0x100;
        let _ = self.flow_control.add_permits(permits as usize);
        Ok(())
    }

    async fn monitor_lock(&self) -> Result<()> {
        let mut buf = vec![0; 10 as _];
        self.write_lock.recv(&mut buf).await?;
        if buf[5] < 5 {
            self.pause_writing();
            sleep(Duration::from_millis(1000)).await;
            self.resume_writing();
        }
        Ok(())
    }

    pub async fn reading(&self) -> Result<()> {
        let mut rx = self.reading_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    pub fn pause_reading(&self) {
        self.reading_tx.send_replace(false);
    }

    pub fn resume_reading(&self) {
        self.reading_tx.send_replace(true);
    }

    pub async fn writing(&self) -> Result<()> {
        let mut rx = self.writing_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await?;
        }
    }

    pub fn pause_writing(&self) {
        self.writing_tx.send_replace(false);
    }

    pub fn resume_writing(&self) {
        self.writing_tx.send_replace(true);
    }

    pub async fn read(&self) -> Result<&[u8]> {
        if self.is_terminated() {
            return Err(Error::new(ErrorKind::Transport(
                TransportErrorKind::Terminated,
            )));
        }
        self.reading().await;
        // TODO: ITR read
        Ok(&[])
    }

    pub async fn write(&self, buf: &[u8]) -> Result<()> {
        if self.is_terminated() {
            return Err(Error::new(ErrorKind::Transport(
                TransportErrorKind::Terminated,
            )));
        }
        self.writing().await;
        Ok(())
    }

    pub fn is_terminated(&self) -> bool {
        self.term_tx.is_closed()
    }

    pub fn terminated(&self) -> impl Future<Output = ()> {
        let term_tx = self.term_tx.clone();
        async move { term_tx.closed().await }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    MonitorLockError(Error),
    MonitorWindowError(Error),
}

#[derive(Debug)]
pub struct SubscriptionReq {
    tx: mpsc::UnboundedSender<Event>,
    ready_tx: oneshot::Sender<()>,
}

impl Event {
    pub fn handle_events(
        mut msg_rx: mpsc::UnboundedReceiver<Event>,
        mut sub_rx: mpsc::Receiver<SubscriptionReq>,
    ) -> Result<()> {
        tokio::spawn(async move {
            struct Subscription {
                tx: mpsc::UnboundedSender<Event>,
            }
            let mut subs: Vec<Subscription> = vec![];
            loop {
                tokio::select! {
                    msg = msg_rx.recv(), if subs.len() > 0 => {
                        match msg {
                            Some(evt) => {
                                subs.retain(|sub| sub.tx.send(evt.clone()).is_ok());
                            }
                            None => break,
                        }
                    },
                    sub_opts = sub_rx.recv() => {
                        match sub_opts {
                            Some(SubscriptionReq { tx, ready_tx }) => {
                                let _ = ready_tx.send(());
                                subs.push(Subscription { tx });
                            }
                            None => break,
                        };
                    },
                }
            }
        });
        Ok(())
    }

    pub async fn subscribe(
        sub_tx: &mut mpsc::Sender<SubscriptionReq>,
    ) -> Result<mpsc::UnboundedReceiver<Event>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel();
        sub_tx
            .send(SubscriptionReq { tx, ready_tx })
            .await
            .map_err(|_| Error::new(ErrorKind::Internal(InternalErrorKind::EventSubFailed)))?;
        ready_rx
            .await
            .map_err(|_| Error::new(ErrorKind::Internal(InternalErrorKind::EventSubFailed)))?;
        Ok(rx)
    }
}
