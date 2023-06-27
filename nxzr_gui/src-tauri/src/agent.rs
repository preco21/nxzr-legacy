use std::sync::Arc;

use nxzr_shared::{
    event::{self, SubscriptionReq},
    setup_event,
};
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};

use crate::{shutdown::Shutdown, wsl};

#[derive(Debug, Error)]
pub enum AgentManagerError {
    #[error("wsl instance already launched")]
    WslInstanceAlreadyLaunched,
    #[error("wsl instance is not ready")]
    WslInstanceNotReady,
    #[error(transparent)]
    WslError(#[from] wsl::WslError),
    #[error(transparent)]
    Event(#[from] event::EventError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct AgentManager {
    wsl_instance_tx: Arc<watch::Sender<Option<JoinHandle<Result<(), AgentManagerError>>>>>,
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
            msg_tx,
            event_sub_tx,
            shutdown,
        })
    }

    pub async fn launch_wsl_instance(&self) -> Result<(), AgentManagerError> {
        if self.wsl_instance_tx.borrow().is_some() {
            return Err(AgentManagerError::WslInstanceAlreadyLaunched);
        }
        tracing::info!("launching WSL process...");
        let mut child = wsl::spawn_wsl_shell_process().await?;
        let handle = tokio::spawn({
            let shutdown = self.shutdown.clone();
            let wsl_instance_tx = self.wsl_instance_tx.clone();
            async move {
                let _shutdown_guard = shutdown.guard();
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

    pub async fn launch_agent_daemon(&self) -> Result<(), AgentManagerError> {
        if !self.is_wsl_ready() {
            return Err(AgentManagerError::WslInstanceNotReady);
        }

        // 2. nxzr_server agent check 진행 -> 실패하면 이벤트 발생, bail out
        // 3. nxzr_server agent 핸들 잡고 있는 데몬 task 생성
        // 4. multiplex 사용할 수 있도록 즉시 connect하여 channel 상태에 저장
        // ㄴ 만약 connect에서 터지면, 각 레벨에서 알아서 처리 (이벤트 등으로 ui 에 오류 표시 등)
        // use tonic::transport::Endpoint;
        // let channel = Endpoint::from_static("http://[::1]:50052")
        //   .connect()
        //   .await?;
        unimplemented!()
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
