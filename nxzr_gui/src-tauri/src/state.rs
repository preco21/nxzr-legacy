use crate::AppError;
use nxzr_shared::{event::SubscriptionReq, setup_event};
use ringbuf::Rb;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    task,
};

pub struct AppState {
    pub logging: Arc<LoggingState>,
    pub task_handles: Mutex<HashMap<String, task::JoinHandle<Result<(), AppError>>>>,
}

impl AppState {
    pub fn new(log_sub_tx: mpsc::Sender<SubscriptionReq<LoggingEvent>>) -> Self {
        Self {
            logging: Arc::new(LoggingState {
                seen_buf: Mutex::new(ringbuf::HeapRb::new(1024)),
                log_sub_tx,
            }),
            task_handles: Mutex::new(HashMap::new()),
        }
    }

    pub async fn is_task_running(&self, task_name: &str) -> bool {
        let task_handles = self.task_handles.lock().await;
        task_handles.contains_key(task_name)
    }

    pub async fn add_task(
        &self,
        task_name: &str,
        join_handle: task::JoinHandle<Result<(), AppError>>,
    ) {
        let mut task_handles = self.task_handles.lock().await;
        task_handles.insert(task_name.to_string(), join_handle);
    }

    pub async fn cancel_task(&self, task_name: &str) -> Result<(), AppError> {
        let mut task_handles = self.task_handles.lock().await;
        if let Some(join_handle) = task_handles.remove(task_name) {
            join_handle.abort();
            Ok(())
        } else {
            Err(AppError::TaskNotFound)
        }
    }
}

pub struct LoggingState {
    seen_buf: Mutex<ringbuf::HeapRb<String>>,
    log_sub_tx: mpsc::Sender<SubscriptionReq<LoggingEvent>>,
}

impl LoggingState {
    pub async fn push_log(&self, event: &str) {
        let mut seen_buf = self.seen_buf.lock().await;
        seen_buf.push_overwrite(event.to_string());
    }

    pub async fn logs(&self) -> Vec<String> {
        let seen_buf = self.seen_buf.lock().await;
        seen_buf.iter().cloned().collect::<Vec<_>>()
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<LoggingEvent>, AppError> {
        Ok(LoggingEvent::subscribe(&mut self.log_sub_tx.clone()).await?)
    }
}

#[derive(Debug, Clone)]
pub struct LoggingEvent(String);

impl LoggingEvent {
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl LoggingEvent {
    setup_event!(LoggingEvent);
}

impl From<String> for LoggingEvent {
    fn from(value: String) -> Self {
        Self(value)
    }
}
