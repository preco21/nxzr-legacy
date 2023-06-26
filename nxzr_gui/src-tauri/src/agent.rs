use command_group::AsyncGroupChild;
use nxzr_shared::{
    event::{self, SubscriptionReq},
    setup_event,
};
use std::{future::Future, sync::Arc};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, oneshot, watch, Mutex};

use crate::wsl;

#[derive(Debug, Error)]
pub enum AgentManagerError {
    #[error("wsl instance already launched")]
    WslInstanceAlreadyLaunched,
    #[error(transparent)]
    WslError(#[from] wsl::WslError),
    #[error(transparent)]
    Event(#[from] event::EventError),
}

#[derive(Debug)]
pub struct AgentManager {
    wsl_instance_tx: watch::Sender<Option<AsyncGroupChild>>,
    msg_tx: mpsc::Sender<Event>,
    event_sub_tx: mpsc::Sender<SubscriptionReq<Event>>,
    sig_close_tx: broadcast::Sender<()>,
    closed_tx: mpsc::Sender<()>,
}

impl AgentManager {
    pub async fn new() -> Result<Self, AgentManagerError> {
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let (sig_close_tx, mut sig_close_rx) = broadcast::channel(1);

        let (msg_tx, msg_rx) = mpsc::channel(256);
        let (event_sub_tx, event_sub_rx) = mpsc::channel(1);
        Event::handle_events(msg_rx, event_sub_rx)?;

        // Inner를 따로 만들어야 할 수도 있다 async task 때문에...
        Ok(Self {
            wsl_instance_tx: watch::channel(None).0,
            msg_tx,
            event_sub_tx,
            sig_close_tx,
            closed_tx,
        })
    }

    pub async fn launch_wsl_instance(&self) -> Result<(), AgentManagerError> {
        let rx = self.wsl_instance_tx.subscribe();
        if rx.borrow().is_some() {
            return Err(AgentManagerError::WslInstanceAlreadyLaunched);
        }
        let child = wsl::spawn_wsl_shell_process().await?;
        self.wsl_instance_tx.send_replace(Some(child));
        Ok(())
    }

    pub async fn wsl_ready(&self) {
        let mut rx = self.wsl_instance_tx.subscribe();
        while rx.borrow().is_some() {
            rx.changed().await.unwrap();
        }
    }

    // FIXME: 페이즈2
    // FIXME: 아예 다른 struct로 분리하는 것도 고려해보기
    pub async fn launch_agent_daemon() -> Result<(), AgentManagerError> {
        // 1. wsl이 준비되었는지 확인, wsl 준비가 안되었으면 에러, 기다리기 없음
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

    pub async fn wait(&self) -> impl Future<Output = ()> {
        let closed_tx = self.closed_tx.clone();
        async move { closed_tx.closed().await }
    }

    pub fn close(&self) {
        let _ = self.sig_close_tx.send(());
    }
}

impl Drop for AgentManager {
    fn drop(&mut self) {
        self.close();
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
