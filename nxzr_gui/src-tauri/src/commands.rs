use crate::AppError;
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
