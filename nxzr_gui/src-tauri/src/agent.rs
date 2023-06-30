use crate::wsl;
use nxzr_shared::{
    event::{self, SubscriptionReq},
    setup_event,
    shutdown::Shutdown,
};
use std::{path::Path, sync::Arc};
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
    time,
};
use tonic::transport::{Channel, Endpoint};

#[derive(Debug, Error)]
pub enum AgentManagerError {
    #[error("wsl instance already launched")]
    WslInstanceAlreadyLaunched,
    #[error("wsl instance is not ready")]
    WslInstanceNotReady,
    #[error("agent instance already launched")]
    AgentInstanceAlreadyLaunched,
    #[error(transparent)]
    WslError(#[from] wsl::WslError),
    #[error(transparent)]
    Tonic(#[from] tonic::transport::Error),
    #[error(transparent)]
    Event(#[from] event::EventError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct AgentManager {
    wsl_instance_tx: Arc<watch::Sender<Option<JoinHandle<Result<(), AgentManagerError>>>>>,
    agent_instance_tx:
        Arc<watch::Sender<Option<(JoinHandle<Result<(), AgentManagerError>>, Channel)>>>,
    msg_tx: mpsc::Sender<Event>,
    event_sub_tx: mpsc::Sender<SubscriptionReq<Event>>,
    shutdown: Shutdown,
}

impl AgentManager {
    pub async fn new(shutdown: Shutdown) -> Result<Self, AgentManagerError> {
        let (msg_tx, msg_rx) = mpsc::channel(256);
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;

        Ok(Self {
            wsl_instance_tx: Arc::new(watch::channel(None).0),
            agent_instance_tx: Arc::new(watch::channel(None).0),
            msg_tx,
            event_sub_tx,
            shutdown,
        })
    }

    pub async fn launch_wsl_anchor_instance(&self) -> Result<(), AgentManagerError> {
        if self.wsl_instance_tx.borrow().is_some() {
            return Err(AgentManagerError::WslInstanceAlreadyLaunched);
        }
        tracing::info!("launching WSL process...");
        let mut child = wsl::spawn_wsl_bare_shell().await?;
        let handle = tokio::spawn({
            let shutdown = self.shutdown.clone();
            let wsl_instance_tx = self.wsl_instance_tx.clone();
            async move {
                let _shutdown_guard = shutdown.drop_guard();
                tokio::select! {
                    _ = shutdown.recv_shutdown() => {
                        let _ = child.kill();
                    },
                    _ = child.wait() => {},
                }
                tracing::info!("terminating WSL process...");
                wsl_instance_tx.send_replace(None);
                Ok::<_, AgentManagerError>(())
            }
        });
        self.wsl_instance_tx.send_replace(Some(handle));
        Ok(())
    }

    pub fn is_wsl_ready(&self) -> bool {
        self.wsl_instance_tx.borrow().is_some()
    }

    pub async fn wsl_ready(&self) {
        let mut rx = self.wsl_instance_tx.subscribe();
        while rx.borrow().is_none() {
            rx.changed().await.unwrap();
        }
    }

    pub async fn launch_agent_daemon(
        &self,
        server_exec_path: &Path,
    ) -> Result<(), AgentManagerError> {
        if !self.is_wsl_ready() {
            return Err(AgentManagerError::WslInstanceNotReady);
        }
        if self.agent_instance_tx.borrow().is_some() {
            return Err(AgentManagerError::AgentInstanceAlreadyLaunched);
        }
        tracing::info!("launching agent daemon process...");
        // FIXME: kill dangling if there's dangling child...
        let mut child = wsl::spawn_wsl_agent_daemon(server_exec_path).await?;
        // FIXME: Immediately connect to the agent daemon may fail... find a way to wait for the agent daemon to be ready.
        // FIXME: find out why the agent daemon is terminated immediately when there's duplicate agent daemon process.
        let channel = Endpoint::from_static("http://[::1]:50052")
            .connect()
            .await?;
        let handle = tokio::spawn({
            let shutdown = self.shutdown.clone();
            let agent_instance_tx = self.agent_instance_tx.clone();
            async move {
                let _shutdown_guard = shutdown.drop_guard();
                tokio::select! {
                    _ = shutdown.recv_shutdown() => {
                        let _ = child.kill();
                    },
                    _ = child.wait() => {},
                }
                // Wait for seconds loosely to make sure the agent daemon is terminated.
                let _ = time::timeout(time::Duration::from_millis(3000), child.wait()).await;
                tracing::info!("terminating agent daemon process...");
                agent_instance_tx.send_replace(None);
                Ok::<_, AgentManagerError>(())
            }
        });
        self.agent_instance_tx.send_replace(Some((handle, channel)));
        Ok(())
    }

    // FIXME: connection 관련 로직들은 따로 관리하는 것이 맞을까...?
    pub async fn connect_switch() -> Result<(), AgentManagerError> {
        unimplemented!()
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>, AgentManagerError> {
        Event::subscribe(&mut self.event_sub_tx.clone())
            .await
            .map_err(|err| AgentManagerError::from(err))
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    WslLaunch,
    WslTerminate,
}

impl Event {
    setup_event!(Event);
}
