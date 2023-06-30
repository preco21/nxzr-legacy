use crate::{
    config,
    state::{self, AppState},
    support::{
        agent, installer,
        usbipd::{self, AdapterInfo},
        wsl,
    },
    util,
};
use nxzr_shared::event;
use std::path::Path;
use tauri::Manager;
use tokio::{process::Command, sync::mpsc};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("failed to show window on ready")]
    WindowReadyFailed,
    #[error("failed to resolve app dirs")]
    UnableToResolveAppDirs,
    #[error("failed to resolve kernel image path")]
    KernelImageResolveFailed,
    #[error("failed to resolve agent archive path")]
    AgentArchiveResolveFailed,
    #[error("failed to resolve server exec path")]
    ServerExecResolveFailed,
    #[error(transparent)]
    AgentManagerError(#[from] state::AgentManagerError),
    #[error(transparent)]
    LoggingManagerError(#[from] state::LoggingManagerError),
    #[error(transparent)]
    InstallerError(#[from] installer::InstallerError),
    #[error(transparent)]
    WslError(#[from] wsl::WslError),
    #[error(transparent)]
    UsbipdError(#[from] usbipd::UsbipdError),
    #[error(transparent)]
    AgentError(#[from] agent::AgentError),
    #[error(transparent)]
    EventError(#[from] event::EventError),
    #[error(transparent)]
    SystemCommandError(#[from] util::SystemCommandError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
}

impl serde::Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[tauri::command]
pub fn window_ready(window: tauri::Window, name: String) -> Result<(), CommandError> {
    window
        .get_window(name.as_str())
        .ok_or(CommandError::WindowReadyFailed)?
        .show()?;
    Ok(())
}

#[tauri::command]
pub fn send_log(kind: String, message: String) {
    match kind.as_str() {
        "info" => tracing::info!("[renderer]: {}", message),
        "warn" => tracing::warn!("[renderer]: {}", message),
        "error" => tracing::error!("[renderer]: {}", message),
        _ => tracing::debug!("[renderer]: {}", message),
    }
}

#[tauri::command]
pub async fn open_log_window(handle: tauri::AppHandle) -> Result<(), CommandError> {
    if let Some(log_window) = handle.get_window("log") {
        log_window.set_focus()?;
        return Ok(());
    }
    let log_window = tauri::WindowBuilder::from_config(
        &handle,
        tauri::utils::config::WindowConfig {
            label: "log".into(),
            url: tauri::WindowUrl::App("index-log.html".into()),
            title: "NXZR - Logs".into(),
            visible: false,
            resizable: true,
            min_width: Some(800.0),
            min_height: Some(600.0),
            width: 800.0,
            height: 600.0,
            ..Default::default()
        },
    )
    .build()?;
    #[cfg(debug_assertions)]
    log_window.open_devtools();
    Ok(())
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SubscribeLoggingResponse {
    logs: Vec<String>,
}

#[tauri::command]
pub async fn subscribe_logging(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> Result<SubscribeLoggingResponse, CommandError> {
    let (log_tx, mut log_rx) = mpsc::channel(1);
    let logs = state.logging_manager.start_watch(log_tx).await?;
    tokio::spawn(async move {
        while let Some(event) = log_rx.recv().await {
            let log_string = event.to_string();
            window.emit("logging:log", log_string).unwrap();
        }
    });
    Ok(SubscribeLoggingResponse { logs })
}

#[tauri::command]
pub async fn unsubscribe_logging(state: tauri::State<'_, AppState>) -> Result<(), CommandError> {
    state.logging_manager.stop_watch().await?;
    Ok(())
}

#[tauri::command]
pub async fn reveal_log_folder() -> Result<(), CommandError> {
    let app_dirs = util::get_app_dirs().ok_or(CommandError::UnableToResolveAppDirs)?;
    let dir = app_dirs
        .data_dir()
        .join(Path::new(config::LOG_FOLDER_NAME))
        .to_str()
        .ok_or(CommandError::UnableToResolveAppDirs)?
        .to_owned();
    tracing::info!("opening log dir: {}", &dir);
    // Ignore errors when calling `explorer.exe` because it sometimes fails even
    // when the actual operation is success.
    let _ = util::run_system_command({
        let mut cmd = Command::new("explorer.exe");
        cmd.args(&[&dir]);
        cmd
    })
    .await;
    Ok(())
}

#[tauri::command]
pub async fn check_1_setup_installed() -> Result<(), CommandError> {
    installer::check_setup_installed().await?;
    Ok(())
}

#[tauri::command]
pub async fn check_2_wslconfig(handle: tauri::AppHandle) -> Result<(), CommandError> {
    let kernel_path = util::get_resource(&handle, config::WSL_KERNEL_IMAGE_NAME)
        .ok_or(CommandError::KernelImageResolveFailed)?;
    installer::check_wslconfig(kernel_path.as_path()).await?;
    Ok(())
}

#[tauri::command]
pub async fn check_3_agent_registered() -> Result<(), CommandError> {
    installer::check_agent_registered().await?;
    Ok(())
}

#[tauri::command]
pub async fn install_1_program_setup() -> Result<(), CommandError> {
    installer::install_program_setup().await?;
    Ok(())
}

#[tauri::command]
pub async fn install_2_ensure_wslconfig(handle: tauri::AppHandle) -> Result<(), CommandError> {
    let kernel_path = util::get_resource(&handle, config::WSL_KERNEL_IMAGE_NAME)
        .ok_or(CommandError::KernelImageResolveFailed)?;
    installer::ensure_wslconfig(kernel_path.as_path()).await?;
    Ok(())
}

#[tauri::command]
pub async fn install_3_register_agent(handle: tauri::AppHandle) -> Result<(), CommandError> {
    let agent_archive_path = util::get_resource(&handle, config::WSL_DISTRO_ARCHIVE_NAME)
        .ok_or(CommandError::AgentArchiveResolveFailed)?;
    installer::register_agent(&agent_archive_path).await?;
    Ok(())
}

// Usbipd
#[tauri::command]
pub async fn list_hid_adapters(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AdapterInfo>, CommandError> {
    state.agent_manager.wsl_ready().await;
    let adapters = usbipd::list_hid_adapters().await?;
    tracing::info!("hid adapters: {:?}", adapters);
    Ok(adapters)
}

#[tauri::command]
pub async fn attach_hid_adapter(hardware_id: String) -> Result<(), CommandError> {
    usbipd::attach_hid_adapter(&hardware_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn detach_hid_adapter(hardware_id: String) -> Result<(), CommandError> {
    usbipd::detach_hid_adapter(&hardware_id).await?;
    Ok(())
}

// Wsl
#[tauri::command]
pub async fn shutdown_wsl() -> Result<(), CommandError> {
    wsl::shutdown_wsl().await?;
    Ok(())
}

#[tauri::command]
pub async fn full_refresh_wsl() -> Result<(), CommandError> {
    wsl::full_refresh_wsl().await?;
    Ok(())
}

#[tauri::command]
pub async fn launch_wsl_anchor_instance(
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    // The instance may already be launched, we're ignoring it when it's the case.
    let res = state.agent_manager.launch_wsl_anchor_instance().await;
    if let Err(err) = res {
        tracing::error!("failed to launch wsl instance: {}", err);
    }
    Ok(())
}

#[tauri::command]
pub async fn run_wsl_agent_check(handle: tauri::AppHandle) -> Result<(), CommandError> {
    let server_exec_path = util::get_resource(&handle, config::WSL_SERVER_EXEC_NAME)
        .ok_or(CommandError::ServerExecResolveFailed)?;
    agent::run_agent_check(&server_exec_path).await?;
    Ok(())
}

#[tauri::command]
pub async fn launch_agent_daemon(
    handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let server_exec_path = util::get_resource(&handle, config::WSL_SERVER_EXEC_NAME)
        .ok_or(CommandError::ServerExecResolveFailed)?;
    let res = state
        .agent_manager
        .launch_agent_daemon(&server_exec_path)
        .await;
    if let Err(err) = res {
        tracing::error!("failed to launch wsl instance: {}", err);
    }
    Ok(())
}
