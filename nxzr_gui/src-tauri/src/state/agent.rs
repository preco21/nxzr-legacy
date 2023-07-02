use crate::support::agent;
use nxzr_shared::shutdown::Shutdown;
use std::{path::Path, sync::Arc};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::{self, Duration},
};
use tokio_retry::{strategy::FixedInterval, Retry};
use tonic::transport::{Channel, Endpoint, Error as TonicError};

use super::WslManager;

#[derive(Debug, thiserror::Error)]
pub enum AgentManagerError {
    #[error("wsl instance is not ready")]
    WslInstanceNotReady,
    #[error("agent instance already launched")]
    AgentInstanceAlreadyLaunched,
    #[error("failed to shutdown agent instance: the instance is not launched or unavailable")]
    UnableToShutdownAgentInstance,
    #[error(transparent)]
    AgentError(#[from] agent::AgentError),
    #[error(transparent)]
    Tonic(#[from] TonicError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct AgentManager {
    agent_instance: Arc<Mutex<Option<AgentInstance>>>,
    shutdown: Shutdown,
}

pub type AgentInstance = (Channel, mpsc::Sender<oneshot::Sender<()>>);

pub struct SwitchConnection {}

impl AgentManager {
    pub fn new(shutdown: Shutdown) -> Self {
        Self {
            agent_instance: Arc::new(Mutex::new(None)),
            shutdown,
        }
    }

    pub async fn launch_agent_daemon(
        &self,
        server_exec_path: &Path,
        wsl_manager: Arc<WslManager>,
        window: tauri::Window,
    ) -> Result<(), AgentManagerError> {
        if !wsl_manager.is_wsl_ready() {
            return Err(AgentManagerError::WslInstanceNotReady);
        }
        let mut agent_instance = self.agent_instance.lock().await;
        if agent_instance.is_some() {
            return Err(AgentManagerError::AgentInstanceAlreadyLaunched);
        }
        tracing::info!("launching agent daemon process...");
        // Sometimes, the agent daemon process is not terminated properly, so here we are trying to kill the dangling process.
        agent::kill_dangling_agent().await?;
        let mut child = agent::spawn_wsl_agent_daemon(server_exec_path).await?;
        let try_connect = || async move {
            let channel = Endpoint::from_static("http://[::1]:50052")
                .connect()
                .await?;
            Ok::<Channel, TonicError>(channel)
        };
        let channel = Retry::spawn(FixedInterval::from_millis(1000).take(3), try_connect).await?;
        let (req_terminate_tx, mut req_terminate_rx) = mpsc::channel::<oneshot::Sender<()>>(1);
        tokio::spawn({
            let shutdown = self.shutdown.clone();
            let agent_instance = self.agent_instance.clone();
            async move {
                let _shutdown_guard = shutdown.drop_guard();
                let sig_close_tx = tokio::select! {
                    Some(tx) = req_terminate_rx.recv() => {
                        let _ = child.kill();
                        Some(tx)
                    },
                    _ = child.wait() => None,
                    _ = shutdown.recv_shutdown() => {
                        let _ = child.kill();
                        None
                    },
                };
                // Wait for seconds loosely to make sure the agent daemon is terminated.
                let _ = time::timeout(Duration::from_millis(3000), child.wait()).await;
                tracing::info!("terminating agent daemon process...");
                let mut agent_instance = agent_instance.lock().await;
                let _ = agent_instance.take();
                if let Some(notify_tx) = sig_close_tx {
                    let _ = notify_tx.send(());
                }
                window.emit("agent:status_update", ()).unwrap();
                Ok::<_, AgentManagerError>(())
            }
        });
        agent_instance.replace((channel, req_terminate_tx));
        Ok(())
    }

    pub async fn terminate_agent_daemon(&self) -> Result<(), AgentManagerError> {
        let (term_complete_tx, term_complete_rx) = oneshot::channel();
        let agent_instance = self.agent_instance.lock().await;
        let Some((.., req_terminate_tx)) = agent_instance.as_ref() else {
            return Err(AgentManagerError::UnableToShutdownAgentInstance);
        };
        let _ = req_terminate_tx.send(term_complete_tx).await;
        drop(agent_instance);
        let _ = term_complete_rx.await;
        Ok(())
    }

    pub async fn is_agent_daemon_ready(&self) -> bool {
        self.agent_instance.lock().await.is_some()
    }

    pub async fn get_device_status(&self) -> Result<(), AgentManagerError> {
        unimplemented!()
    }

    pub async fn connect_switch(&self) -> Result<(), AgentManagerError> {
        // todo: spawn a task to monitor the switch connection
        unimplemented!()
    }

    pub async fn reconnect_switch(&self) -> Result<(), AgentManagerError> {
        // todo: spawn a task to monitor the switch connection
        unimplemented!()
    }

    pub async fn get_protocol_state(&self) -> Result<(), AgentManagerError> {
        unimplemented!()
    }

    pub async fn create_button_control_stream(&self) -> Result<(), AgentManagerError> {
        unimplemented!()
    }
}
