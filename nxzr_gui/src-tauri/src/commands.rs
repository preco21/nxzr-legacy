use crate::{state::AppState, AppError};
use tauri::Manager;

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
    task_name: String,
) -> Result<(), AppError> {
    state.cancel_task(&task_name).await?;
    Ok(())
}

#[tauri::command]
pub fn log(kind: String, message: String) {
    match kind.as_str() {
        "info" => tracing::info!("JS: {}", message),
        "warn" => tracing::warn!("JS: {}", message),
        "error" => tracing::error!("JS: {}", message),
        _ => tracing::debug!("JS: {}", message),
    }
}

#[tauri::command]
pub async fn open_logs_window(handle: tauri::AppHandle) -> Result<(), AppError> {
    let logs_window = tauri::WindowBuilder::from_config(
        &handle,
        tauri::utils::config::WindowConfig {
            label: "logs".to_string(),
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
    logs_window.open_devtools();

    Ok(())
}

#[derive(Clone, serde::Serialize)]
pub struct SubscribeLoggingResponse {
    logs: Vec<String>,
    task_name: String,
}

#[tauri::command]
pub async fn subscribe_logging(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> Result<SubscribeLoggingResponse, AppError> {
    let task_name = "logging".to_string();
    state.reserve_task(&task_name).await?;
    let logs = state.logging.logs().await;
    let mut log_rx = state.logging.events().await?;
    let task_handle = tokio::spawn({
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
    state.add_task(&task_name, task_handle).await;
    Ok(SubscribeLoggingResponse { logs, task_name })
}
