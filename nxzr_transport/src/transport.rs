use crate::sock::hci::{Datagram, SocketAddr};
use futures::Future;
use std::{sync::Arc, time::Duration};
use tokio::{
    join,
    sync::{mpsc, watch},
    time::sleep,
};

pub enum TransportResult {}

#[derive(Debug)]
pub struct NxzrTransport {
    write_window_dg: Datagram,
    write_lock_dg: Datagram,
    reading_tx: watch::Sender<bool>,
    closed_tx: mpsc::Sender<()>,
}

impl NxzrTransport {
    pub async fn register() -> (Arc<Self>, NxzrTransportHandle) {
        let (close_tx, close_rx) = mpsc::channel(1);
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let s = Self {
            write_window_dg: Datagram::bind(SocketAddr { dev_id: 0 }).await.unwrap(),
            write_lock_dg: Datagram::bind(SocketAddr { dev_id: 0 }).await.unwrap(),
            reading_tx: watch::channel(false).0,
            closed_tx,
        };
        let s = Arc::new(s);
        let s1 = s.clone();
        let sc1 = close_tx.clone();
        let h1 = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = s1.monitor_window() => {},
                    _ = sc1.closed() => break,
                }
            }
        });
        let s2 = s.clone();
        let sc2 = close_tx.clone();
        let h2 = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = s2.monitor_lock() => {},
                    _ = sc2.closed() => break,
                }
            }
        });
        tokio::spawn(async move {
            let _ = join!(h1, h2);
            drop(closed_rx);
        });
        (
            s,
            NxzrTransportHandle {
                _close_rx: close_rx,
            },
        )
    }

    async fn monitor_window(&self) {
        sleep(Duration::from_millis(1000)).await;
        println!("monitor window");
    }

    async fn monitor_lock(&self) {
        sleep(Duration::from_millis(500)).await;
        println!("monitor lock");
    }

    // todo: add shutdown signal and use tokio select to cooperate with rx?
    // async fn reading(&self) {
    //     let mut rx = self.sig_reading.subscribe();
    //     while !*rx.borrow() {
    //         rx.changed().await.unwrap();
    //     }
    // }

    // fn pause_read(&self) {
    //     self.sig_reading.send(false).unwrap();
    // }

    // fn resume_read(&self) {
    //     self.sig_reading.send(true).unwrap();
    // }

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
