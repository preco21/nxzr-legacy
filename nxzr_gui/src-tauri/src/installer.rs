use crate::{config, util};
use std::path::Path;
use thiserror::Error;
use tokio::fs;

#[derive(Debug, Error)]
pub enum InstallerError {
    #[error("WSL is not available")]
    WslNotAvailable,
    #[error("usbipd not available")]
    UsbipdNotAvailable,
    #[error("failed to resolve WSL configuration or is it missing?")]
    WslConfigResolveFailed,
    #[error("WSL configuration is malformed")]
    WslConfigMalformed,
    #[error(
        "the field `kernel` in WSL configuration does not match with the program's resource path"
    )]
    WslConfigFieldMismatch,
    #[error("the agent is not registered as a WSL distro")]
    AgentNotRegistered,
    #[error("the agent is not properly configured to work with WSL 2")]
    AgentWslVersionMismatch,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub async fn check_setup_installed() -> Result<(), InstallerError> {
    // Checks if WSL is available.
    util::run_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&["--version"]);
        cmd
    })
    .await
    .map_err(|_err| InstallerError::WslNotAvailable)?;
    // Checks if `usbipd` is available.
    util::run_system_command({
        let mut cmd = tokio::process::Command::new("usbipd.exe");
        cmd.args(&["--version"]);
        cmd
    })
    .await
    .map_err(|_err| InstallerError::UsbipdNotAvailable)?;
    Ok(())
}

pub async fn check_wslconfig(resource_path: &Path) -> Result<(), InstallerError> {
    // Checks if the `.wslconfig` exists in the user folder.
    let wslconfig_dir = directories::UserDirs::new()
        .ok_or(InstallerError::WslConfigResolveFailed)?
        .home_dir()
        .join(".wslconfig");
    if !util::file_exists(wslconfig_dir.as_path()).await {
        return Err(InstallerError::WslConfigResolveFailed);
    };
    // Checks if the `.wslconfig` is properly configured with specific fields.
    let wslconfig_raw = fs::read(wslconfig_dir.as_path()).await?;
    let wslconfig_content = String::from_utf8_lossy(&wslconfig_raw);
    let ini_conf = ini::Ini::load_from_str(&wslconfig_content)
        .map_err(|_err| InstallerError::WslConfigMalformed)?;
    let section = ini_conf
        .section(Some("wsl2"))
        .ok_or(InstallerError::WslConfigMalformed)?;
    let field_val = section
        .get("kernel")
        .ok_or(InstallerError::WslConfigMalformed)?;
    // The path to the binary must be provided by the caller itself, because it
    // cannot be known before the program is built.
    //
    // So, it will be injected from Tauri's `build.rs` script.
    let actual_path = resource_path
        .to_str()
        .ok_or(InstallerError::WslConfigMalformed)?;
    if field_val != actual_path {
        return Err(InstallerError::WslConfigFieldMismatch);
    }
    Ok(())
}

pub async fn check_agent_registered() -> Result<(), InstallerError> {
    // Checks if `nxzr-agent` as a WSL distro is properly installed.
    let wsl = wslapi::Library::new()?;
    if !wsl.is_distribution_registered(config::WSL_AGENT_NAME) {
        return Err(InstallerError::AgentNotRegistered);
    }
    // Checks for the distro's WSL version.
    let conf = wsl
        .get_distribution_configuration(config::WSL_AGENT_NAME)
        .map_err(|_err| InstallerError::AgentNotRegistered)?;
    if conf.version != 2 {
        return Err(InstallerError::AgentWslVersionMismatch);
    }
    Ok(())
}

/// These scripts are responsible for checking / installing infrastructures and
/// system requirements that is required to run NXZR for the current system.
pub async fn install_setup() {}

pub async fn ensure_wslconfig() {}

pub async fn register_agent() {}

pub async fn restart_wsl() {}
