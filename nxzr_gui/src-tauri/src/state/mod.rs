use crate::AppError;
use nxzr_shared::shutdown::Shutdown;
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::{sync::Mutex, task};

mod agent;
pub use agent::*;
mod logging;
pub use logging::*;

pub struct AppState {
    pub agent_manager: Arc<AgentManager>,
    pub logging_manager: Arc<LoggingManager>,
    task_handles: Arc<Mutex<HashMap<String, Option<task::JoinHandle<Result<(), AppError>>>>>>,
    shutdown: Shutdown,
}

impl AppState {
    pub fn new(
        agent_manager: Arc<AgentManager>,
        logging_manager: Arc<LoggingManager>,
        shutdown: Shutdown,
    ) -> Self {
        let task_handles: HashMap<String, Option<task::JoinHandle<Result<(), AppError>>>> =
            HashMap::new();
        let task_handles = Arc::new(Mutex::new(task_handles));
        // FIXME: revisit, this may not be necessary
        // Spawn a task to handle cleanup for task handles.
        tokio::spawn({
            let shutdown = shutdown.clone();
            let task_handles = task_handles.clone();
            async move {
                let _shutdown_guard = shutdown.drop_guard();
                shutdown.recv_shutdown().await;
                let mut task_handles = task_handles.lock().await;
                for value in task_handles.values_mut() {
                    if let Some(join_handle) = value.take() {
                        join_handle.abort();
                        let _ = join_handle.await;
                    }
                }
            }
        });
        Self {
            agent_manager,
            logging_manager,
            task_handles,
            shutdown,
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
