use nxzr_shared::{
    event::{self, SubscriptionReq},
    setup_event,
};
use ringbuf::Rb;
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum LoggingManagerError {
    #[error(transparent)]
    Event(#[from] event::EventError),
}

pub struct LoggingManager {
    seen_buf: Mutex<ringbuf::HeapRb<String>>,
    log_sub_tx: mpsc::Sender<SubscriptionReq<Event>>,
}

impl LoggingManager {
    pub fn new(log_out_rx: mpsc::Receiver<Event>) -> Result<Self, LoggingManagerError> {
        let (log_sub_tx, log_sub_rx) = mpsc::channel(1);
        Event::handle_events(log_out_rx, log_sub_rx)?;
        Ok(Self {
            seen_buf: Mutex::new(ringbuf::HeapRb::new(1024)),
            log_sub_tx,
        })
    }

    pub async fn push_log(&self, event: &str) {
        let mut seen_buf = self.seen_buf.lock().await;
        seen_buf.push_overwrite(event.to_string());
    }

    pub async fn full_logs(&self) -> Vec<String> {
        let seen_buf = self.seen_buf.lock().await;
        seen_buf.iter().cloned().collect::<Vec<_>>()
    }

    pub async fn logs(&self) -> Result<mpsc::UnboundedReceiver<Event>, LoggingManagerError> {
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
