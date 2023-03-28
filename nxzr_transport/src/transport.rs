use crate::sock::hci::{Datagram, SocketAddr};
use std::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::{sync::watch, time::sleep};

pub enum TransportResult {}

#[derive(Debug)]
pub struct NxzrTransport {
    sig_reading: watch::Sender<bool>,
    sig_writing: watch::Sender<bool>,
    is_closing: AtomicBool,
    sig_closed: watch::Sender<bool>,
    write_window_dg: Datagram,
    write_lock_dg: Datagram,
}

impl NxzrTransport {
    pub async fn new(sa: SocketAddr) -> Self {
        Self {
            sig_reading: watch::channel(false).0,
            sig_writing: watch::channel(false).0,
            sig_closed: watch::channel(false).0,
            is_closing: AtomicBool::new(false),
            write_window_dg: Datagram::bind(sa).await.unwrap(),
            write_lock_dg: Datagram::bind(sa).await.unwrap(),
        }
    }

    // todo: add shutdown signal and use tokio select to cooperate with rx?
    async fn reading(&self) {
        let mut rx = self.sig_reading.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    fn pause_read(&self) {
        self.sig_reading.send(false).unwrap();
    }

    fn resume_read(&self) {
        self.sig_reading.send(true).unwrap();
    }

    pub async fn read(self: Pin<&Self>) -> &[u8] {
        self.reading().await;
        // read data from socket
    }

    pub fn write(self: Pin<&Self>, buf: impl AsRef<[u8]>) {
        let buf = buf.as_ref();
    }

    pub fn is_closed(&self) -> bool {
        let mut rx = self.sig_closed.subscribe();
        *rx.borrow()
    }

    pub async fn closed(self: Pin<&Self>) {
        let mut rx = self.sig_closed.subscribe();
        while !*rx.borrow() {
            rx.changed().await.unwrap();
        }
    }

    pub async fn close(self: Pin<&Self>) {
        if self.is_closing.load(Ordering::Acquire) {
            return;
        }
        self.is_closing.store(true, Ordering::Release);
        self.pause_read();
        sleep(Duration::from_millis(1000)).await;
        self.sig_closed.send(true).unwrap();
    }
}
