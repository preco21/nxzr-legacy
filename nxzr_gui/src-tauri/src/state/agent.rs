use crate::support::agent;
use nxzr_shared::{
    event::{self, SubscriptionReq},
    setup_event,
    shutdown::Shutdown,
};
use std::{path::Path, sync::Arc};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    task::JoinHandle,
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
    #[error(transparent)]
    AgentError(#[from] agent::AgentError),
    #[error(transparent)]
    Event(#[from] event::EventError),
    #[error(transparent)]
    Tonic(#[from] TonicError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct AgentManager {
    agent_instance: Arc<Mutex<Option<(JoinHandle<Result<(), AgentManagerError>>, Channel)>>>,
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
            agent_instance: Arc::new(Mutex::new(None)),
            msg_tx,
            event_sub_tx,
            shutdown,
        })
    }

    pub async fn launch_agent_daemon(
        &self,
        server_exec_path: &Path,
        wsl_manager: Arc<WslManager>,
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
        let handle = tokio::spawn({
            let shutdown = self.shutdown.clone();
            let agent_instance = self.agent_instance.clone();
            async move {
                let _shutdown_guard = shutdown.drop_guard();
                tokio::select! {
                    _ = shutdown.recv_shutdown() => {
                        let _ = child.kill();
                    },
                    _ = child.wait() => {},
                }
                // Wait for seconds loosely to make sure the agent daemon is terminated.
                let _ = time::timeout(Duration::from_millis(3000), child.wait()).await;
                tracing::info!("terminating agent daemon process...");
                let mut agent_instance = agent_instance.lock().await;
                let _ = agent_instance.take();
                Ok::<_, AgentManagerError>(())
            }
        });
        agent_instance.replace((handle, channel));
        Ok(())
    }

    // FIXME: connection 관련 로직들은 따로 관리하는 것이 맞을까...?
    pub async fn connect_switch() -> Result<(), AgentManagerError> {
        unimplemented!()
    }

    pub async fn events(&self) -> Result<mpsc::UnboundedReceiver<Event>, AgentManagerError> {
        Ok(Event::subscribe(&mut self.event_sub_tx.clone()).await?)
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
