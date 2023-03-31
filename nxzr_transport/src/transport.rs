use crate::sock::hci;
use crate::Result;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::join;
use tokio::sync::{mpsc, watch, Semaphore};
use tokio::time::sleep;

const MAX_FLOW_CONTROL: usize = 4;

pub struct TransportConfig {
    closed_tx: mpsc::Sender<()>,
}

#[derive(Debug)]
pub struct Transport {
    write_window: hci::Datagram,
    write_lock: hci::Datagram,
    reading_tx: watch::Sender<bool>,
    closed_tx: mpsc::Sender<()>,
    flow_control: Semaphore
}

impl Transport {
    pub async fn new(config: TransportConfig) -> Result<Self> {
        // Device ids must be targeting to the local machine
        let write_window = hci::Datagram::bind(hci::SocketAddr { dev_id: 0 }).await?;
        write_window.as_ref().set_filter(hci::Filter {
            type_mask: 1 << 0x04,
            event_mask: [1 << 0x13, 0],
            opcode: 0,
        })?;
        let write_lock = hci::Datagram::bind(hci::SocketAddr { dev_id: 0 }).await?;
        write_lock.as_ref().set_filter(hci::Filter {
            type_mask: 1 << 0x04,
            event_mask: [1 << 0x1b, 0],
            opcode: 0,
        })?;
        Ok(Self {
            write_window,
            write_lock,
            reading_tx: watch::channel(true).0,
            closed_tx: config.closed_tx,
            flow_control: Semaphore::new(MAX_FLOW_CONTROL)
        })
    }

    pub async fn register() -> Result<(Arc<Self>, NxzrTransportHandle)> {
        let (close_tx, close_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let s = Arc::new(Self::new(TransportConfig { closed_tx }).await?);
        let s_for_window = s.clone();
        let close_tx_for_window = close_tx.clone();
        let window_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = s_for_window.monitor_window() => {},
                    _ = close_tx_for_window.closed() => break,
                }
            }
        });
        let s_for_lock = s.clone();
        let close_tx_for_lock = close_tx.clone();
        let lock_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = s_for_lock.monitor_lock() => {},
                    _ = close_tx_for_lock.closed() => break,
                }
            }
        });
        tokio::spawn(async move {
            let _ = join!(window_handle, lock_handle);
            drop(closed_rx);
        });
        Ok((
            s,
            NxzrTransportHandle {
                _close_rx: close_rx,
            },
        ))
    }

    async fn monitor_window(&self) -> Result<()> {
        let mut buf = vec![0; 10 as _];
        self.write_window.recv(&mut buf).await?;
        self.write_window
        Ok(())
    }

    async fn monitor_lock(&self) -> Result<()> {
        let mut buf = vec![0; 10 as _];
        self.write_lock.recv(&mut buf).await?;
        if buf[5] < 5 {
            self.pause_read();
            sleep(Duration::from_millis(1000)).await;
            self.resume_read();
        }
        Ok(())
    }

    // todo: add shutdown signal and use tokio select to cooperate with rx?
    async fn reading(&self) {
        let mut rx = self.reading_tx.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    fn pause_read(&self) {
        self.reading_tx.send(false).unwrap();
    }

    fn resume_read(&self) {
        self.reading_tx.send(true).unwrap();
    }

    // pub async fn read(&self) -> &[u8] {
    //     self.reading().await;
    //     // read data from socket
    // }

    // pub fn write(&self, buf: impl AsRef<[u8]>) {
    //     let buf = buf.as_ref();
    // }

    // pub fn is_closed(&self) -> bool {
    //     let mut rx = self.sig_closed.subscribe();
    //     *rx.borrow()
    // }

    pub fn closed(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
    }
}

pub struct NxzrTransportHandle {
    _close_rx: mpsc::Receiver<()>,
}

impl Drop for NxzrTransportHandle {
    fn drop(&mut self) {
        // Required for drop order
    }
}
