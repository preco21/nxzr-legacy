use crate::{config, util};
use thiserror::Error;
use tokio::{sync::mpsc, time};

const WSL_FULL_REFRESH_SCRIPT: &str = include_str!("scripts/full-refresh-wsl.ps1");

#[derive(Debug, Error)]
pub enum WslError {
    #[error("WSL shutdown failed")]
    WslShutdownFailed,
    #[error("WSL distribution warm up failed")]
    WslDistroWarmUpFailed,
    #[error(transparent)]
    SystemCommandError(#[from] util::SystemCommandError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub async fn shutdown_wsl() -> Result<(), WslError> {
    util::run_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&["--shutdown"]);
        cmd
    })
    .await
    .map_err(|_err| WslError::WslShutdownFailed)?;
    // We must wait for 8 seconds to make sure that the WSL is shutdown completely.
    // Please refer the document for more details: https://learn.microsoft.com/en-us/windows/wsl/wsl-config#the-8-second-rule
    time::sleep(time::Duration::from_secs(8)).await;
    Ok(())
}

pub async fn ensure_agent_distro_running() -> Result<(), WslError> {
    let output = util::run_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&["-d", config::WSL_AGENT_NAME, "--", "echo", "ok"]);
        cmd
    })
    .await
    .map_err(|err| WslError::SystemCommandError(err))?;
    if output.is_empty() {
        return Err(WslError::WslDistroWarmUpFailed);
    }
    Ok(())
}

pub async fn full_refresh_wsl() -> Result<(), WslError> {
    let (output_tx, mut output_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(line) = output_rx.recv().await {
            tracing::trace!("[wsl] full WSL refresh: {}", line);
        }
    });
    util::run_powershell_script_privileged(WSL_FULL_REFRESH_SCRIPT, None, Some(output_tx)).await?;
    Ok(())
}
