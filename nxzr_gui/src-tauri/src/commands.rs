use crate::{config, installer, state::AppState, util, AppError};
use std::path::Path;
use tauri::Manager;
use tokio::{process::Command, sync::mpsc};

#[tauri::command]
pub fn window_ready(window: tauri::Window, name: String) -> Result<(), AppError> {
    window
        .get_window(name.as_str())
        .ok_or(AppError::WindowReadyFailed)?
        .show()?;
    Ok(())
}

#[tauri::command]
pub async fn cancel_task(
    state: tauri::State<'_, AppState>,
    task_label: String,
) -> Result<(), AppError> {
    state.cancel_task(&task_label).await?;
    Ok(())
}

#[tauri::command]
pub fn log(kind: String, message: String) {
    match kind.as_str() {
        "info" => tracing::info!("[renderer]: {}", message),
        "warn" => tracing::warn!("[renderer]: {}", message),
        "error" => tracing::error!("[renderer]: {}", message),
        _ => tracing::debug!("[renderer]: {}", message),
    }
}

#[tauri::command]
pub async fn open_log_window(handle: tauri::AppHandle) -> Result<(), AppError> {
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
pub struct SubscribeLoggingResponse {
    logs: Vec<String>,
    task_label: String,
}

#[tauri::command]
pub async fn subscribe_logging(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> Result<SubscribeLoggingResponse, AppError> {
    let task_label = "logging".to_owned();
    let mut logs: Option<Vec<String>> = None;
    state
        .register_task(&task_label, async {
            logs = Some(state.logging.logs().await);
            let mut log_rx = state.logging.events().await?;
            let handle = tokio::spawn({
                let logging = state.logging.clone();
                async move {
                    while let Some(event) = log_rx.recv().await {
                        let log_string = event.to_string();
                        logging.push_log(log_string.as_str()).await;
                        window.emit("logging:log", log_string).unwrap();
                    }
                    Ok::<(), AppError>(())
                }
            });
            Ok(handle)
        })
        .await?;
    Ok(SubscribeLoggingResponse {
        logs: logs.unwrap_or(Vec::new()),
        task_label,
    })
}

#[derive(Clone, serde::Serialize)]
pub struct GetAppDirsResponse {
    config_dir: String,
    data_dir: String,
}

#[tauri::command]
pub async fn reveal_log_folder() -> Result<(), AppError> {
    let app_dirs = util::get_app_dirs().ok_or(anyhow::anyhow!("failed to resolve app dirs"))?;
    let dir = app_dirs
        .data_dir()
        .join(Path::new(config::LOG_FOLDER_NAME))
        .to_str()
        .ok_or(anyhow::anyhow!("failed to resolve app dirs"))?
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
pub async fn check_1_setup_installed() -> Result<(), AppError> {
    installer::check_setup_installed().await?;
    Ok(())
}

#[tauri::command]
pub async fn check_2_wslconfig(handle: tauri::AppHandle) -> Result<(), AppError> {
    let kernel_path = handle
        .path_resolver()
        .resolve_resource(config::WSL_KERNEL_IMAGE_NAME)
        .ok_or(anyhow::anyhow!("failed to resolve kernel image path"))?;
    installer::check_wslconfig(kernel_path.as_path()).await?;
    Ok(())
}

#[tauri::command]
pub async fn check_3_agent_registered() -> Result<(), AppError> {
    installer::check_agent_registered().await?;
    Ok(())
}

#[tauri::command]
pub async fn install_1_program_setup(handle: tauri::AppHandle) -> Result<(), AppError> {
    let (output_tx, mut output_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(line) = output_rx.recv().await {
            tracing::trace!("[installer] install_1_program_setup: {}", line);
        }
    });
    installer::install_program_setup(Some(output_tx)).await?;
    Ok(())
}

#[tauri::command]
pub async fn install_2_ensure_wslconfig(handle: tauri::AppHandle) -> Result<(), AppError> {
    let kernel_path = handle
        .path_resolver()
        .resolve_resource(config::WSL_KERNEL_IMAGE_NAME)
        .ok_or(anyhow::anyhow!("failed to resolve kernel image path"))?;
    let (output_tx, mut output_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(line) = output_rx.recv().await {
            tracing::trace!("[installer] install_2_ensure_wslconfig: {}", line);
        }
    });
    installer::ensure_wslconfig(kernel_path.as_path(), Some(output_tx)).await?;
    Ok(())
}

#[tauri::command]
pub async fn install_3_register_agent(handle: tauri::AppHandle) -> Result<(), AppError> {
    // FIXME: implement me
    Err(AppError::TaskNotFound)
}
