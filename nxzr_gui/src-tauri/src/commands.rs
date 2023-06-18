use crate::{config, state::AppState, util, AppError};
use std::path::Path;
use tauri::Manager;
use tokio::process::Command;

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
            label: "log".to_string(),
            url: tauri::WindowUrl::App("index-log.html".into()),
            title: "NXZR - Logs".to_string(),
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
    let task_label = "logging".to_string();
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
pub fn get_app_dirs() -> Result<GetAppDirsResponse, AppError> {
    let app_dirs = util::get_app_dirs().ok_or(anyhow::anyhow!("failed to resolve app dirs"))?;
    Ok(GetAppDirsResponse {
        config_dir: app_dirs
            .config_dir()
            .to_str()
            .ok_or(anyhow::anyhow!("failed to resolve app dirs"))?
            .to_string(),
        data_dir: app_dirs
            .data_dir()
            .to_str()
            .ok_or(anyhow::anyhow!("failed to resolve app dirs"))?
            .to_string(),
    })
}

#[tauri::command]
pub async fn reveal_in_file_explorer(path: String) -> Result<(), AppError> {
    util::run_system_command({
        let mut cmd = Command::new("explorer.exe");
        cmd.args(&["/select", &path]);
        cmd
    })
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn open_log_folder() -> Result<(), AppError> {
    let app_dirs = util::get_app_dirs().ok_or(anyhow::anyhow!("failed to resolve app dirs"))?;
    let dir = app_dirs
        .data_dir()
        .join(Path::new(config::LOG_FOLDER_NAME))
        .to_str()
        .ok_or(anyhow::anyhow!("failed to resolve app dirs"))?
        .to_string();
    tracing::info!("dir: {}", &dir);
    util::run_system_command({
        let mut cmd = Command::new("explorer.exe");
        cmd.args(&[&dir]);
        cmd
    })
    .await?;
    Ok(())
}
