// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use agent::AgentManagerError;
use installer::InstallerError;
use nxzr_shared::{event::EventError, shutdown::Shutdown};
use state::{AppState, LoggingEvent};
use std::{path::Path, sync::Arc};
use tauri::Manager;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tracing_subscriber::prelude::*;
use usbipd::UsbipdError;
use util::SystemCommandError;
use wsl::WslError;

mod agent;
mod commands;
mod config;
mod installer;
mod state;
mod usbipd;
mod util;
mod wsl;

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
    #[error(transparent)]
    SystemCommandError(#[from] SystemCommandError),
    #[error(transparent)]
    InstallerError(#[from] InstallerError),
    #[error(transparent)]
    AgentManagerError(#[from] AgentManagerError),
    #[error(transparent)]
    WslError(#[from] WslError),
    #[error(transparent)]
    UsbipdError(#[from] UsbipdError),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
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

    let app_dirs = util::get_app_dirs().ok_or(anyhow::anyhow!("failed to resolve app dirs"))?;
    let module_filter =
        tracing_subscriber::filter::Targets::new().with_target("nxzr_gui", tracing::Level::TRACE);
    let (log_out_tx, log_out_rx) = mpsc::channel(1024);
    let log_chan_writer = util::TracingChannelWriter::new(Arc::new(log_out_tx));
    let file_appender = tracing_appender::rolling::hourly(
        app_dirs.data_dir().join(Path::new(config::LOG_FOLDER_NAME)),
        "output.log",
    );
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
                .event_format(tracing_subscriber::fmt::format().compact()),
        )
        .with(
            tracing_subscriber::fmt::Layer::default()
                .event_format(tracing_subscriber::fmt::format().compact())
                .with_writer(log_file_appender),
        );
    tracing::subscriber::set_global_default(subscriber)?;

    let (log_sub_tx, log_sub_rx) = mpsc::channel(1);
    LoggingEvent::handle_events(log_out_rx, log_sub_rx)?;

    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);
    let (sig_shutdown_tx, mut sig_shutdown_rx) = mpsc::channel::<oneshot::Sender<()>>(1);
    tokio::spawn({
        // Retain the final shutdown complete signal so that prevents the
        // [WeakSender] from dropping immediately.
        let shutdown_complete_tx = shutdown_complete_tx.clone();
        async move {
            let tx = sig_shutdown_rx.recv().await.unwrap();
            drop(shutdown_rx);
            drop(shutdown_complete_tx);
            let _ = shutdown_complete_rx.recv().await;
            let _ = tx.send(());
        }
    });
    let shutdown = Shutdown::new(shutdown_tx, shutdown_complete_tx);

    let agent_manager = Arc::new(agent::AgentManager::new(shutdown.clone()).await?);
    let app_state = AppState::new(agent_manager, log_sub_tx, shutdown);
    tauri::async_runtime::set(tokio::runtime::Handle::current());
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::window_ready,
            commands::cancel_task,
            commands::log,
            commands::open_log_window,
            commands::subscribe_logging,
            commands::reveal_log_folder,
            commands::check_1_setup_installed,
            commands::check_2_wslconfig,
            commands::check_3_agent_registered,
            commands::install_1_program_setup,
            commands::install_2_ensure_wslconfig,
            commands::install_3_register_agent,
            commands::list_hid_adapters,
            commands::attach_hid_adapter,
            commands::detach_hid_adapter,
            commands::shutdown_wsl,
            commands::full_refresh_wsl,
            commands::launch_wsl_anchor_instance,
            commands::run_wsl_agent_check,
            commands::launch_agent_daemon,
        ])
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::Destroyed => {
                // Closes all remaining windows when the main window is closed.
                let window = event.window();
                if window.label() == "main" {
                    let windows = window.app_handle().windows();
                    for (_, window) in windows.iter() {
                        window.close().unwrap();
                    }
                }
            }
            _ => {}
        })
        .setup(|app| {
            // Handle kill signals.
            tokio::spawn({
                let windows = app.windows();
                async move {
                    let _ = tokio::signal::ctrl_c().await;
                    tracing::info!("kill signal received, closing all windows");
                    for window in windows.values() {
                        if window.is_closable().unwrap() {
                            window.close().unwrap();
                        }
                    }
                }
            });
            // Enable devtools.
            #[cfg(debug_assertions)]
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            tracing::info!("{}, {:?}, {}", app.package_info().name, argv, cwd);
        }))
        .build(tauri::generate_context!())
        .map_err(|err| anyhow::anyhow!("error while running application: {}", err))?
        .run(move |app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                // Gracefully shutdown the application.
                api.prevent_exit();
                tokio::task::block_in_place({
                    let app_handle = app_handle.clone();
                    let sig_shutdown_tx = sig_shutdown_tx.clone();
                    move || {
                        let (tx, rx) = oneshot::channel();
                        // Send shutdown request.
                        let _ = sig_shutdown_tx.blocking_send(tx);
                        // Wait for the all tasks to complete.
                        let _ = rx.blocking_recv();
                        // Manually exit the application.
                        app_handle.exit(0);
                    }
                })
            }
            _ => {}
        });

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
