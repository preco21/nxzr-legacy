use crate::support::agent;
use command_group::AsyncGroupChild;
use nxzr_proto::{
    nxzr_client::NxzrClient, ConnectSwitchRequest, ControlStreamRequest, GetDeviceStatusRequest,
    GetDeviceStatusResponse, ImuControlReport, Position, StickControlReport,
};
use nxzr_shared::shutdown::Shutdown;
use std::{future::Future, path::Path, sync::Arc};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver},
        oneshot, Mutex,
    },
    time::{self, Duration},
};
use tokio_retry::{strategy::FixedInterval, Retry};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    transport::{Channel, Endpoint, Error as TonicError},
    Request, Status as TonicStatus,
};

use super::WslManager;

#[derive(Debug, thiserror::Error)]
pub enum AgentManagerError {
    #[error("wsl instance is not ready")]
    WslInstanceNotReady,
    #[error("agent instance already launched")]
    AgentInstanceAlreadyLaunched,
    #[error("failed to shutdown agent instance: the instance is not launched or unavailable")]
    UnableToShutdownAgentInstance,
    #[error("agent is not ready")]
    AgentNotReady,
    #[error("rpc failed: {code} {message}")]
    RpcFailed { code: tonic::Code, message: String },
    #[error(transparent)]
    AgentError(#[from] agent::AgentError),
    #[error(transparent)]
    Tonic(#[from] TonicError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<TonicStatus> for AgentManagerError {
    fn from(status: TonicStatus) -> Self {
        Self::RpcFailed {
            code: status.code(),
            message: status.message().to_owned(),
        }
    }
}

pub struct AgentManager {
    agent_instance: Arc<Mutex<Option<AgentInstance>>>,
    shutdown: Shutdown,
}

pub type AgentInstance = (Channel, mpsc::Sender<oneshot::Sender<()>>);

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
        agent::kill_agent().await?;
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
                        kill_agent_gracefully(&mut child).await;
                        Some(tx)
                    },
                    _ = shutdown.recv_shutdown() => {
                        kill_agent_gracefully(&mut child).await;
                        None
                    },
                    _ = child.wait() => None,
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

    pub async fn get_device_status(&self) -> Result<GetDeviceStatusResponse, AgentManagerError> {
        let mut client = self.agent_client().await?;
        let response: tonic::Response<GetDeviceStatusResponse> = client
            .get_device_status(Request::new(GetDeviceStatusRequest {}))
            .await?;
        let body = response.into_inner();
        Ok(body)
    }

    pub async fn connect_switch(&self, window: tauri::Window) -> Result<(), AgentManagerError> {
        let mut client = self.agent_client().await?;
        tokio::spawn({
            let shutdown = self.shutdown.clone();
            async move {
                let _shutdown_guard = shutdown.drop_guard();
                let mut stream = client
                    .connect_switch(Request::new(ConnectSwitchRequest {}))
                    .await?
                    .into_inner();
                while let Some(response) = stream.message().await? {
                    tracing::info!("switch connect: {:?}", response);
                }
                // FIXME: Handle errors properly
                Ok::<_, AgentManagerError>(())
            }
        });
        Ok(())
    }

    pub async fn reconnect_switch(&self) -> Result<(), AgentManagerError> {
        // todo: spawn a task to monitor the switch connection
        unimplemented!()
    }

    pub async fn get_protocol_state(&self) -> Result<(), AgentManagerError> {
        unimplemented!()
    }

    pub async fn run_control_stream(&self) -> Result<(), AgentManagerError> {
        let mut client = self.agent_client().await?;
        tokio::spawn({
            let shutdown = self.shutdown.clone();
            async move {
                let _shutdown_guard = shutdown.drop_guard();
                let (tx, rx) = mpsc::unbounded_channel();
                tokio::spawn(async move {
                    let mut interval = time::interval(Duration::from_secs(1));
                    loop {
                        interval.tick().await;
                        let _ = tx.send(ControlStreamRequest {
                            request_id: String::new(),
                            buttons: Vec::new(),
                            stick: Some(StickControlReport {
                                left_position: Some(Position { x: 0.0, y: 0.0 }),
                                right_position: Some(Position { x: 0.0, y: 0.0 }),
                            }),
                            imu: Some(ImuControlReport {
                                // FIXME: todo
                                ..Default::default()
                            }),
                        });
                    }
                });
                let outbound_stream = UnboundedReceiverStream::new(rx);
                let mut inbound_stream = client
                    .control_stream(Request::new(outbound_stream))
                    .await?
                    .into_inner();
                // FIXME: Handle errors properly
                Ok::<_, AgentManagerError>(())
            }
        });
        unimplemented!()
    }

    async fn agent_client(&self) -> Result<NxzrClient<Channel>, AgentManagerError> {
        let agent_instance = self.agent_instance.lock().await;
        let Some((channel, ..)) = agent_instance.as_ref() else {
            return Err(AgentManagerError::AgentNotReady);
        };
        Ok(NxzrClient::new(channel.clone()))
    }
}

fn kill_agent_gracefully<'a>(child: &'a mut AsyncGroupChild) -> impl Future<Output = ()> + 'a {
    async move {
        match child.id() {
            Some(_) => {
                let _ = agent::kill_agent().await;
            }
            None => {
                let _ = child.kill();
            }
        }
    }
}
