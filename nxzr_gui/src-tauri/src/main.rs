// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nxzr_shared::event::EventError;
use state::{AppState, LoggingEvent};
use tauri::Manager;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod state;
mod util;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to show window on ready")]
    WindowReadyFailed,
    #[error("task already running")]
    TaskAlreadyRunning,
    #[error("no such task found")]
    TaskNotFound,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error(transparent)]
    EventError(#[from] EventError),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bootstrap().await?;

    let Some(app_dirs) = util::get_app_dirs() else {
        return Err(anyhow::anyhow!("failed to resolve app dirs"));
    };
    let module_filter =
        tracing_subscriber::filter::Targets::new().with_target("nxzr_gui", tracing::Level::TRACE);
    let (log_out_tx, log_out_rx) = mpsc::channel(1024);
    let log_chan_writer = util::TracingChannelWriter::new(log_out_tx);
    let file_appender = tracing_appender::rolling::hourly(app_dirs.data_dir(), "output.log");
    let (log_file_appender, _guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::registry()
        .with(module_filter)
        .with(
            tracing_subscriber::fmt::Layer::default()
                .event_format(tracing_subscriber::fmt::format().json())
                .with_writer(log_chan_writer),
        )
        .with(
            tracing_subscriber::fmt::Layer::default()
                .event_format(tracing_subscriber::fmt::format().compact())
                .with_writer(log_file_appender),
        );
    tracing::subscriber::set_global_default(subscriber)?;

    let (log_sub_tx, log_sub_rx) = mpsc::channel(1);
    LoggingEvent::handle_events(log_out_rx, log_sub_rx)?;

    // let (async_proc_input_tx, async_proc_input_rx) = mpsc::channel(1);
    let app_state = state::AppState::new(log_sub_tx);

    tauri::async_runtime::set(tokio::runtime::Handle::current());
    tauri::Builder::default()
        .manage(app_state)
        // FIXME: find good way to specify this
        .invoke_handler(tauri::generate_handler![
            commands::window_ready,
            // js2rs,
            // greet,
            subscribe_logging,
            cancel_task,
            commands::open_logs_window,
        ])
        .setup(|app| {
            // let app_handle = app.handle();
            // tokio::spawn(async move { async_process_model(async_proc_input_rx, app_handle).await });

            #[cfg(debug_assertions)]
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|err| anyhow::anyhow!("error while running application: {}", err))?;

    Ok(())
}

#[derive(serde::Serialize)]
struct SubscribeLoggingResponse {
    logs: Vec<String>,
    task_name: String,
}

#[tauri::command]
async fn subscribe_logging(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> Result<SubscribeLoggingResponse, AppError> {
    let task_name = "log".to_string();
    if state.is_task_running(&task_name).await {
        return Err(AppError::TaskAlreadyRunning);
    }
    let mut log_rx = state.logging.events().await?;
    let logs = state.logging.logs().await;
    let task_handle = tokio::spawn({
        let logging = state.logging.clone();
        async move {
            while let Some(event) = log_rx.recv().await {
                let log_string = event.to_string();
                logging.push_log(log_string.as_str()).await;
                window.emit("log", log_string).unwrap();
            }
            Ok::<(), AppError>(())
        }
    });
    state.add_task(&task_name, task_handle).await;
    Ok(SubscribeLoggingResponse { logs, task_name })
}

#[tauri::command]
async fn cancel_task(task_name: String, state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    state.cancel_task(&task_name).await?;
    Ok(())
}

// #[tauri::command]
// fn greet(name: &str) -> String {
//     format!("Hello, {}! You've been greeted from Rust!", name)
// }

// async fn async_process_model(
//     mut input_rx: mpsc::Receiver<String>,
//     handle: tauri::AppHandle,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     while let Some(input) = input_rx.recv().await {
//         let output = input;
//         rs2js(output, &handle);
//     }
//     Ok(())
// }

// fn rs2js<R: tauri::Runtime>(message: String, manager: &impl Manager<R>) {
//     tracing::info!(?message, "rs2js");
//     manager
//         .emit_all("rs2js", format!("rs: {}", message))
//         .unwrap();
// }

// #[tauri::command]
// async fn js2rs(message: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
//     tracing::info!(?message, "js2rs");
//     let async_proc_input_tx = state.inner.lock().await;
//     async_proc_input_tx
//         .send(message)
//         .await
//         .map_err(|e| e.to_string())
// }

/// Bootstraps the program.
///
/// You can put whatever setup logic to this routine, however this function will
/// always be called at the application startup and the main routine will wait
/// until it's complete.
///
/// Which means you should not put any long-running tasks here.
#[tracing::instrument(target = "setup")]
pub async fn bootstrap() -> anyhow::Result<()> {
    let Some(dirs) = util::get_app_dirs() else {
        return Err(anyhow::anyhow!("failed to resolve app dirs"));
    };
    // Create new global config dirs.
    if !util::directory_exists(dirs.config_dir()).await {
        util::mkdir_p(dirs.config_dir()).await?;
    }
    if !util::directory_exists(dirs.data_dir()).await {
        util::mkdir_p(dirs.data_dir()).await?;
    }
    Ok(())
}
