use crate::{config, util};
use command_group::AsyncGroupChild;
use std::path::Path;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt},
    sync::mpsc,
    time::{self, Duration},
};

const WSL_FULL_REFRESH_SCRIPT: &str = include_str!("../scripts/full-refresh-wsl.ps1");

#[derive(Debug, thiserror::Error)]
pub enum WslError {
    #[error("WSL shutdown failed")]
    WslShutdownFailed,
    #[error("path conversion failed")]
    PathConversionFailed,
    #[error(transparent)]
    SystemCommandError(#[from] util::SystemCommandError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
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
    time::sleep(Duration::from_secs(8)).await;
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

pub async fn spawn_wsl_bare_shell() -> Result<AsyncGroupChild, WslError> {
    let (child, _stdout, _stderr) = util::spawn_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&["-d", config::WSL_AGENT_NAME]);
        cmd
    })
    .await?;
    Ok(child)
}

pub async fn run_wsl_agent_check(server_exec_path: &Path) -> Result<(), WslError> {
    let path = convert_windows_path_to_wsl(&server_exec_path).await?;
    let output = util::run_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&["-d", config::WSL_AGENT_NAME, "--", path.as_str(), "check"]);
        cmd
    })
    .await?;
    // Log `check` outputs from the WSL agent.
    for line in output.split("\n").filter(|l| !l.trim().is_empty()) {
        tracing::info!(
            "{}",
            util::format_tracing_json_log_data(&util::parse_tracing_json_log_data(line)?)
        )
    }
    Ok(())
}

pub async fn spawn_wsl_agent_daemon(server_exec_path: &Path) -> Result<AsyncGroupChild, WslError> {
    let path = convert_windows_path_to_wsl(&server_exec_path).await?;
    let (child, stdout, stderr) = util::spawn_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&["-d", config::WSL_AGENT_NAME, "--", path.as_str(), "run"]);
        cmd
    })
    .await?;
    let mut combined_lines = stdout.chain(stderr).lines();
    // Spawn a task to read the stdout/stderr of the child process for logging.
    tokio::spawn(async move {
        while let Some(line) = combined_lines.next_line().await.unwrap() {
            match util::parse_tracing_json_log_data(&line) {
                Ok(data) => {
                    tracing::info!("[child]: {}", util::format_tracing_json_log_data(&data))
                }
                Err(_) => tracing::info!("[child]: {}", line),
            }
        }
    });
    Ok(child)
}

pub async fn convert_windows_path_to_wsl(path: &Path) -> Result<String, WslError> {
    let output = util::run_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&[
            "-d",
            config::WSL_AGENT_NAME,
            "--",
            "wslpath",
            "-u",
            escape_wsl_path(path)
                .ok_or(WslError::PathConversionFailed)?
                .as_str(),
        ]);
        cmd
    })
    .await?;
    Ok(output.trim().into())
}

fn escape_wsl_path(path: &Path) -> Option<String> {
    path.to_str()
        .and_then(|s| Some(s.replace("\\", "\\\\").replace(" ", "\\ ")))
}
