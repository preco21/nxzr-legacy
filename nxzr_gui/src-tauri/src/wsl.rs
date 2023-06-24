use crate::util;
use thiserror::Error;
use tokio::time;

#[derive(Debug, Error)]
pub enum WslError {
    #[error("WSL shutdown failed")]
    WslShutdownFailed,
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
