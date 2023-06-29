use std::future::Future;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Shutdown {
    shutdown_tx: mpsc::Sender<()>,
    shutdown_complete_tx: mpsc::WeakSender<()>,
}

impl Shutdown {
    /// Creates a new shutdown signal.
    pub fn new(shutdown_tx: mpsc::Sender<()>, shutdown_complete_tx: mpsc::Sender<()>) -> Self {
        Self {
            shutdown_tx,
            shutdown_complete_tx: shutdown_complete_tx.downgrade(),
        }
    }

    /// Returns true if the shutdown signal has been received.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_tx.is_closed()
    }

    /// Returns a future that completes when the shutdown signal is received.
    pub fn recv_shutdown(&self) -> impl Future<Output = ()> {
        let shutdown_tx = self.shutdown_tx.clone();
        async move { shutdown_tx.closed().await }
    }

    /// Creates a new shutdown guard. Drop it to signal shutdown.
    pub fn drop_guard(&self) -> mpsc::Sender<()> {
        self.shutdown_complete_tx.clone().upgrade().unwrap()
    }
}
