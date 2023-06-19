use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};

use crate::{config, util};

#[derive(Debug, Error)]
pub enum InstallerError {
    #[error("WSL is not available")]
    WslNotAvailable,
    #[error("failed to resolve WSL configuration")]
    WslConfigResolveFailed,
    #[error("WSL configuration is malformed")]
    MalformedWslConfig,
    #[error("usbipd not available")]
    UsbipdNotAvailable,
    #[error("nxzr-agent is not installed")]
    NxzrAgentNotInstalled,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub async fn check_installed() -> Result<(), InstallerError> {
    // 1. Checks if WSL is available.
    util::run_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&["--version"]);
        cmd
    })
    .await
    .map_err(|_err| InstallerError::WslNotAvailable)?;
    // 2. Checks if `.wslconfig` is properly configured in the user folder.
    let wslconfig_dir = directories::UserDirs::new()
        .ok_or(InstallerError::WslConfigResolveFailed)?
        .home_dir()
        .join("./.wslconfig");
    if !util::file_exists(wslconfig_dir.as_path()).await {
        return Err(InstallerError::WslConfigResolveFailed);
    };
    // FIXME: use ini parser?
    // extract to function
    let wslconfig_file = File::open(wslconfig_dir).await?;
    let reader = BufReader::new(wslconfig_file);
    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        if line.contains("usbipd") {}
    }
    // FIXME: check if wslconfig has pattern

    let wsl_config = util::read_file(wslconfig_dir.as_path()).await?;

    // 3. Checks if `usbipd` is available.
    util::run_system_command({
        let mut cmd = tokio::process::Command::new("usbipd.exe");
        cmd.args(&["--version"]);
        cmd
    })
    .await
    .map_err(|_err| InstallerError::UsbipdNotAvailable)?;
    // 4. Checks if `nxzr-agent` as a WSL distro is properly installed.
    let wsl = wslapi::Library::new()?;
    if !wsl.is_distribution_registered(config::WSL_AGENT_NAME) {
        return Err(InstallerError::NxzrAgentNotInstalled);
    }
    Ok(())
}
