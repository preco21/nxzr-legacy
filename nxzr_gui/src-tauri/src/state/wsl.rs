use crate::support::wsl;
use nxzr_shared::shutdown::Shutdown;
use std::sync::Arc;
use tokio::{sync::watch, task::JoinHandle};

#[derive(Debug, thiserror::Error)]
pub enum WslManagerError {
    #[error("wsl instance already launched")]
    WslInstanceAlreadyLaunched,
    #[error(transparent)]
    WslError(#[from] wsl::WslError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct WslManager {
    wsl_instance_tx: Arc<watch::Sender<Option<JoinHandle<Result<(), WslManagerError>>>>>,
    shutdown: Shutdown,
}

impl WslManager {
    pub async fn new(shutdown: Shutdown) -> Result<Self, WslManagerError> {
        Ok(Self {
            wsl_instance_tx: Arc::new(watch::channel(None).0),
            shutdown,
        })
    }

    pub async fn launch_wsl_anchor_instance(
        &self,
        window: tauri::Window,
    ) -> Result<(), WslManagerError> {
        if self.wsl_instance_tx.borrow().is_some() {
            return Err(WslManagerError::WslInstanceAlreadyLaunched);
        }
        tracing::info!("launching WSL process...");
        let mut child = wsl::spawn_wsl_bare_shell().await?;
        let handle = tokio::spawn({
            let shutdown = self.shutdown.clone();
            let wsl_instance_tx = self.wsl_instance_tx.clone();
            let window = window.clone();
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
                window.emit("wsl:status_update", ()).unwrap();
                Ok::<_, WslManagerError>(())
            }
        });
        self.wsl_instance_tx.send_replace(Some(handle));
        window.emit("wsl:status_update", ()).unwrap();
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
}
