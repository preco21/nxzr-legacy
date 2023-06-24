use crate::{config, util};
use std::path::Path;
use thiserror::Error;
use tokio::{fs, sync::mpsc, time};

const INSTALL_DEPS_SCRIPT: &str = include_str!("scripts/install-deps.ps1");
const SETUP_WSLCONFIG_SCRIPT: &str = include_str!("scripts/setup-wslconfig.ps1");

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
    #[error("failed to unregister requested distro from WSL")]
    WslDistroUnregisterFailed,
    #[error("the agent is not registered as a WSL distro")]
    AgentNotRegistered,
    #[error("the agent is not properly configured to work with WSL 2")]
    AgentWslVersionMismatch,
    #[error("failed to register agent as a WSL distro")]
    AgentWslRegistrationFailed,
    #[error("failed to resolve app dirs")]
    AppDirResolveFailed,
    #[error("failed to convert path")]
    PathConvertFailed,
    #[error(transparent)]
    SystemCommandError(#[from] util::SystemCommandError),
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

pub async fn check_wslconfig(kernel_path: &Path) -> Result<(), InstallerError> {
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
    let ini_conf = ini::Ini::load_from_str_noescape(&wslconfig_content)
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
    let actual_path = kernel_path
        .to_str()
        .ok_or(InstallerError::PathConvertFailed)?
        .replace("\\", "\\\\");
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
pub async fn install_program_setup() -> Result<(), InstallerError> {
    let (output_tx, mut output_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(line) = output_rx.recv().await {
            tracing::trace!("[installer] install_1_program_setup: {}", line);
        }
    });
    util::run_powershell_script_privileged(INSTALL_DEPS_SCRIPT, None, Some(output_tx)).await?;
    Ok(())
}

pub async fn ensure_wslconfig(kernel_path: &Path) -> Result<(), InstallerError> {
    let actual_path = kernel_path
        .to_str()
        .ok_or(InstallerError::WslConfigMalformed)?
        .replace("\\", "\\\\")
        .to_owned();
    let (output_tx, mut output_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(line) = output_rx.recv().await {
            tracing::trace!("[installer] install_2_ensure_wslconfig: {}", line);
        }
    });
    util::run_powershell_script(
        SETUP_WSLCONFIG_SCRIPT,
        Some(vec!["-KernelPath".to_owned(), actual_path]),
        Some(output_tx),
    )
    .await?;
    Ok(())
}

pub async fn register_agent(agent_archive_path: &Path) -> Result<(), InstallerError> {
    let wsl = wslapi::Library::new()?;
    // This routine is generally unreachable because the checker will ensure
    // that the agent is not registered yet.
    //
    // However, to make sure there's no clutter around agent registration, we
    // just blindly check and unregister it here as if there was no check held
    // in advance.
    if wsl.is_distribution_registered(config::WSL_AGENT_NAME) {
        tracing::info!("agent distro found, unregistering...");
        // TODO: unregister
        util::run_system_command({
            let mut cmd = tokio::process::Command::new("wsl.exe");
            cmd.args(&["--terminate", config::WSL_AGENT_NAME]);
            cmd
        })
        .await
        .map_err(|_err| InstallerError::WslDistroUnregisterFailed)?;
        util::run_system_command({
            let mut cmd = tokio::process::Command::new("wsl.exe");
            cmd.args(&["--unregister", config::WSL_AGENT_NAME]);
            cmd
        })
        .await
        .map_err(|_err| InstallerError::WslDistroUnregisterFailed)?;
        // Wait for a few seconds to make sure the distro is properly unregistered.
        time::sleep(time::Duration::from_secs(8)).await;
    }
    let app_dirs = util::get_app_dirs().ok_or(InstallerError::AppDirResolveFailed)?;
    let install_dir = app_dirs
        .data_dir()
        .join(Path::new(config::WSL_AGENT_INSTALL_DIR_NAME))
        .to_str()
        .ok_or(InstallerError::PathConvertFailed)?
        .to_owned();
    tracing::info!("installing agent distro in: {}", &install_dir);
    let agent_archive_path = agent_archive_path
        .to_str()
        .ok_or(InstallerError::PathConvertFailed)?
        .to_owned();
    tracing::info!("agent archive path: {}", &agent_archive_path);
    util::run_system_command({
        let mut cmd = tokio::process::Command::new("wsl.exe");
        cmd.args(&[
            "--import",
            config::WSL_AGENT_NAME,
            &install_dir,
            &agent_archive_path,
        ]);
        cmd
    })
    .await
    .map_err(|_err| InstallerError::AgentWslRegistrationFailed)?;
    time::sleep(time::Duration::from_secs(1)).await;
    Ok(())
}
