use crate::error::Error;

#[tauri::command]
pub async fn open_logs_window(handle: tauri::AppHandle) -> Result<(), Error> {
    let logs_window = tauri::WindowBuilder::from_config(
        &handle,
        tauri::utils::config::WindowConfig {
            label: "logs".to_string(),
            url: tauri::WindowUrl::App("index-log.html".into()),
            title: "NXZR - Logs".to_string(),
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
