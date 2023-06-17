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
    task_label: String,
) -> Result<(), AppError> {
    state.cancel_task(&task_label).await?;
    Ok(())
}

#[tauri::command]
pub fn log(kind: String, message: String) {
    match kind.as_str() {
        "info" => tracing::info!("[console.log]: {}", message),
        "warn" => tracing::warn!("[console.log]: {}", message),
        "error" => tracing::error!("[console.log]: {}", message),
        _ => tracing::debug!("[console.log]: {}", message),
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
