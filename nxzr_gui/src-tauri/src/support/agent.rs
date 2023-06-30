use super::wsl;
use crate::{config, util};
use command_group::AsyncGroupChild;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error(transparent)]
    WslError(#[from] wsl::WslError),
    #[error(transparent)]
    SystemCommandError(#[from] util::SystemCommandError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub async fn run_agent_check(server_exec_path: &Path) -> Result<(), AgentError> {
    let path = wsl::convert_windows_path_to_wsl(&server_exec_path).await?;
    let output = util::run_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&["-d", config::WSL_DISTRO_NAME, "--", path.as_str(), "check"]);
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

pub async fn spawn_wsl_agent_daemon(
    server_exec_path: &Path,
) -> Result<AsyncGroupChild, AgentError> {
    let path = wsl::convert_windows_path_to_wsl(&server_exec_path).await?;
    let (child, stdout, stderr) = util::spawn_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&["-d", config::WSL_DISTRO_NAME, "--", path.as_str(), "run"]);
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

pub async fn kill_dangling_agent() -> Result<(), AgentError> {
    // Quietly kill the existing agent daemon process.
    let _ = util::run_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&[
            "-d",
            config::WSL_DISTRO_NAME,
            "--",
            "pkill",
            "-f",
            config::WSL_SERVER_EXEC_NAME,
        ]);
        cmd
    })
    .await;
    Ok(())
}
