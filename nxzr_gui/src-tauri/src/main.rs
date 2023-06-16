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
        .invoke_handler(tauri::generate_handler![
            commands::window_ready,
            commands::cancel_task,
            commands::subscribe_logging,
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
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            tracing::info!("{}, {:?}, {}", app.package_info().name, argv, cwd);
            app.emit_all("single-instance", Payload { args: argv, cwd })
                .unwrap();
        }))
        .run(tauri::generate_context!())
        .map_err(|err| anyhow::anyhow!("error while running application: {}", err))?;

    Ok(())
}

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
