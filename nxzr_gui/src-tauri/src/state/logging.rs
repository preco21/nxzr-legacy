use nxzr_shared::{
    event::{self, SubscriptionReq},
    setup_event,
};
use ringbuf::Rb;
use std::sync::Arc;
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    task::JoinHandle,
};

#[derive(Debug, thiserror::Error)]
pub enum LoggingManagerError {
    #[error("watch already started")]
    WatchAlreadyStarted,
    #[error("cannot stop watch that has not been started")]
    WatchNotStarted,
    #[error(transparent)]
    Event(#[from] event::EventError),
}

pub struct LoggingManager {
    log_watch_tx: Arc<Mutex<Option<JoinHandle<()>>>>,
    seen_buf: Arc<Mutex<ringbuf::HeapRb<String>>>,
    log_sub_tx: mpsc::Sender<SubscriptionReq<Event>>,
}

impl LoggingManager {
    pub fn new(log_out_rx: mpsc::Receiver<Event>) -> Result<Self, LoggingManagerError> {
        let (log_sub_tx, log_sub_rx) = mpsc::channel(1);
        Event::handle_events(log_out_rx, log_sub_rx)?;
        Ok(Self {
            log_watch_tx: Arc::new(Mutex::new(None)),
            seen_buf: Arc::new(Mutex::new(ringbuf::HeapRb::new(1024))),
            log_sub_tx,
        })
    }

    pub async fn start_watch(
        &self,
        log_tx: mpsc::Sender<String>,
    ) -> Result<Vec<String>, LoggingManagerError> {
        let mut log_watch = self.log_watch_tx.lock().await;
        if log_watch.is_some() {
            return Err(LoggingManagerError::WatchAlreadyStarted);
        }
        let logs = self.full_logs().await;
        let mut log_rx = self.watch_logs().await?;
        let handle = tokio::spawn({
            let seen_buf = self.seen_buf.clone();
            async move {
                while let Some(log) = log_rx.recv().await {
                    let log_string = log.to_string();
                    seen_buf.lock().await.push_overwrite(log_string.clone());
                    let _ = log_tx.send(log_string).await;
                }
            }
        });
        log_watch.replace(handle);
        Ok(logs)
    }

    pub async fn stop_watch(&self) -> Result<(), LoggingManagerError> {
        let mut log_watch = self.log_watch_tx.lock().await;
        if log_watch.is_none() {
            return Err(LoggingManagerError::WatchNotStarted);
        }
        let handle = log_watch.take().unwrap();
        handle.abort();
        let _ = handle.await;
        Ok(())
    }

    pub async fn push_log(&self, event: &str) {
        let mut seen_buf = self.seen_buf.lock().await;
        seen_buf.push_overwrite(event.to_string());
    }

    pub async fn full_logs(&self) -> Vec<String> {
        let seen_buf = self.seen_buf.lock().await;
        seen_buf.iter().cloned().collect::<Vec<_>>()
    }

    pub async fn watch_logs(&self) -> Result<mpsc::UnboundedReceiver<Event>, LoggingManagerError> {
        Ok(Event::subscribe(&mut self.log_sub_tx.clone()).await?)
    }
}

#[derive(Debug, Clone)]
pub struct Event(String);

impl Event {
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl Event {
    setup_event!(Event);
}

impl From<String> for Event {
    fn from(value: String) -> Self {
        Self(value)
    }
}
