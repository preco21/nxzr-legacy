use crate::{agent::AgentManager, AppError};
use nxzr_shared::{event::SubscriptionReq, setup_event};
use ringbuf::Rb;
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    task,
};

pub struct AppState {
    pub agent_manager: Arc<AgentManager>,
    pub logging: Arc<LoggingState>,
    pub task_handles: Arc<Mutex<HashMap<String, Option<task::JoinHandle<Result<(), AppError>>>>>>,
    pub shutdown_tx: mpsc::Sender<()>,
    pub shutdown_complete_tx: mpsc::WeakSender<()>,
}

impl AppState {
    pub fn new(
        agent_manager: Arc<AgentManager>,
        log_sub_tx: mpsc::Sender<SubscriptionReq<LoggingEvent>>,
        shutdown_tx: mpsc::Sender<()>,
        shutdown_complete_tx: mpsc::Sender<()>,
    ) -> Self {
        let task_handles: HashMap<String, Option<task::JoinHandle<Result<(), AppError>>>> =
            HashMap::new();
        let task_handles = Arc::new(Mutex::new(task_handles));
        // Spawn a task to handle cleanup for task handles.
        tokio::spawn({
            let shutdown_tx = shutdown_tx.clone();
            let shutdown_complete_tx = shutdown_complete_tx.clone();
            let task_handles = task_handles.clone();
            async move {
                shutdown_tx.closed().await;
                let mut task_handles = task_handles.lock().await;
                for value in task_handles.values_mut() {
                    if let Some(join_handle) = value.take() {
                        join_handle.abort();
                        let _ = join_handle.await;
                    }
                }
                drop(shutdown_complete_tx);
            }
        });
        Self {
            agent_manager,
            logging: Arc::new(LoggingState {
                seen_buf: Mutex::new(ringbuf::HeapRb::new(1024)),
                log_sub_tx,
            }),
            task_handles,
            shutdown_tx,
            shutdown_complete_tx: shutdown_complete_tx.downgrade(),
        }
    }

    pub async fn register_task(
        &self,
        task_label: &str,
        create_fut: impl Future<Output = Result<task::JoinHandle<Result<(), AppError>>, AppError>>,
    ) -> Result<(), AppError> {
        self.reserve_task(task_label).await?;
        match create_fut.await {
            Ok(join_handle) => {
                self.set_task(task_label, join_handle).await;
                Ok(())
            }
            Err(err) => {
                self.cancel_task(task_label).await?;
                Err(err)
            }
        }
    }

    async fn reserve_task(&self, task_label: &str) -> Result<(), AppError> {
        let mut task_handles = self.task_handles.lock().await;
        if task_handles.contains_key(task_label) {
            Err(AppError::TaskAlreadyRunning)
        } else {
            task_handles.insert(task_label.into(), None);
            Ok(())
        }
    }

    async fn set_task(
        &self,
        task_label: &str,
        join_handle: task::JoinHandle<Result<(), AppError>>,
    ) {
        let mut task_handles = self.task_handles.lock().await;
        task_handles.insert(task_label.into(), Some(join_handle));
    }

    pub async fn cancel_task(&self, task_label: &str) -> Result<(), AppError> {
        let mut task_handles = self.task_handles.lock().await;
        if let Some(Some(join_handle)) = task_handles.remove(task_label) {
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
