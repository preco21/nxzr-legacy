use crate::AppError;
use nxzr_shared::{event::SubscriptionReq, setup_event};
use std::collections::HashMap;
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    task,
};

pub struct AppState {
    logging: LoggingState,
    task_handles: Mutex<HashMap<uuid::Uuid, task::JoinHandle<Result<(), AppError>>>>,
}

impl AppState {
    pub fn new(log_sub_tx: mpsc::Sender<SubscriptionReq<LoggingEvent>>) -> Self {
        Self {
            logging: LoggingState {
                seen_buf: ringbuf::HeapRb::new(1024),
                log_sub_tx,
            },
            task_handles: Mutex::new(HashMap::new()),
        }
    }
}

pub struct LoggingState {
    seen_buf: ringbuf::HeapRb<String>,
    log_sub_tx: mpsc::Sender<SubscriptionReq<LoggingEvent>>,
}

#[derive(Debug, Clone)]
pub struct LoggingEvent(String);

impl LoggingEvent {
    setup_event!(LoggingEvent);
}

impl From<String> for LoggingEvent {
    fn from(value: String) -> Self {
        Self(value)
    }
}
