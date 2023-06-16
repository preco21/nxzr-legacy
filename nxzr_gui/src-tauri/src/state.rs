use std::collections::HashMap;
use tokio::{
    sync::{broadcast, mpsc, oneshot, Mutex},
    task,
};

pub struct AppState {
    logging: LoggingState,
    task_handles: Mutex<HashMap<uuid::Uuid, task::JoinHandle<Result<(), Error>>>>,
}

impl AppState {
    pub fn new(log_sub_tx: broadcast::Sender<String>) -> Self {
        Self {
            logging: LoggingState {
                seen_buf: ringbuf::HeapRb::new(1024),
            },
            task_handles: Mutex::new(HashMap::new()),
        }
    }
}

pub struct LoggingState {
    seen_buf: ringbuf::HeapRb<String>,
}

#[derive(Debug, Clone)]
pub struct LoggingEvent(String);

impl LoggingEvent {
    setup_event!(LoggingEvent);
}
