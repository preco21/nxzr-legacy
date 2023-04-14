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
    BeingClosed,
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
    pub async fn register(config: TransportConfig) -> Result<(Self, TransportHandle)> {
        let (close_tx, close_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let inner = Arc::new(TransportInner::new(config, close_tx, closed_tx).await?);
        let mut handles = vec![];
        {
            // Handles writer lock timing.
            let inner = inner.clone();
            let msg_tx = msg_tx.clone();
            handles.push(tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = inner.closing() => break,
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
                        _ = inner.closing() => break,
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
            Self { inner },
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
        self.inner.pause();
    }

    pub async fn resume(&self) {
        self.inner.resume();
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>> {
        self.inner.events()
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
    active_tx: watch::Sender<bool>,
    writing_tx: watch::Sender<bool>,
    flow_control: Arc<BoundedSemaphore>,
    read_buf_size: usize,
    close_tx: mpsc::Sender<()>,
    closed_tx: mpsc::Sender<()>,
    event_sub_tx: mpsc::Sender<SubscriptionReq>,
}

impl TransportInner {
    pub async fn new(
        config: TransportConfig,
        close_tx: mpsc::Sender<()>,
        closed_tx: mpsc::Sender<()>,
    ) -> Result<Self> {
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
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;
        Ok(Self {
            write_window,
            write_lock,
            active_tx: watch::channel(true).0,
            writing_tx: watch::channel(true).0,
            close_tx,
            closed_tx,
            event_sub_tx,
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
            self.pause_write();
            sleep(Duration::from_millis(1000)).await;
            self.resume_write();
        }
        Ok(())
    }

    pub async fn active(&self) -> Result<()> {
        let mut rx = self.active_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    pub fn pause(&self) {
        self.active_tx.send_replace(false);
    }

    pub fn resume(&self) {
        self.active_tx.send_replace(true);
    }

    pub async fn writable(&self) -> Result<()> {
        let mut rx = self.writing_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await?;
        }
    }

    pub fn pause_write(&self) {
        self.writing_tx.send_replace(false);
    }

    pub fn resume_write(&self) {
        self.writing_tx.send_replace(true);
    }

    pub async fn read(&self) -> Result<&[u8]> {
        if self.is_closing() {
            return Err(Error::new(ErrorKind::Transport(
                TransportErrorKind::BeingClosed,
            )));
        }
        self.active().await;
        // TODO: ITR read
        Ok(&[])
    }

    pub async fn write(&self, buf: &[u8]) -> Result<()> {
        if self.is_closing() {
            return Err(Error::new(ErrorKind::Transport(
                TransportErrorKind::BeingClosed,
            )));
        }
        self.active().await;
        self.writable().await;
        Ok(())
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>> {
        Event::subscribe(&mut self.event_sub_tx.clone()).await
    }

    pub fn is_closing(&self) -> bool {
        self.close_tx.is_closed()
    }

    pub fn closing(&self) -> impl Future<Output = ()> {
        let close_tx = self.close_tx.clone();
        async move { close_tx.closed().await }
    }

    pub fn closed(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    MonitorLockError(Error),
    MonitorWindowError(Error),
}

#[derive(Debug)]
pub(crate) struct SubscriptionReq {
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
            .map_err(|_| {
                Error::new(ErrorKind::Internal(
                    InternalErrorKind::EventSubscriptionFailed,
                ))
            })?;
        ready_rx.await.map_err(|_| {
            Error::new(ErrorKind::Internal(
                InternalErrorKind::EventSubscriptionFailed,
            ))
        })?;
        Ok(rx)
    }
}
