// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use error::Error;
use std::collections::HashMap;
use tauri::Manager;
use tokio::{
    sync::{broadcast, mpsc, Mutex},
    task,
};
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod error;
mod setup;
mod state;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup::bootstrap().await?;

    let Some(app_dirs) = util::get_app_dirs() else {
        return Err(anyhow::anyhow!("failed to resolve app dirs"));
    };
    let module_filter =
        tracing_subscriber::filter::Targets::new().with_target("nxzr_gui", tracing::Level::TRACE);
    let (log_out_tx, mut log_out_rx) = mpsc::unbounded_channel();
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

    let (log_sub_tx, _log_sub_rx) = broadcast::channel(1024);
    tokio::spawn({
        let log_sub_tx = log_sub_tx.clone();
        async move {
            while let Some(log) = log_out_rx.recv().await {
                let _ = log_sub_tx.send(log);
            }
        }
    });

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

// #[tauri::command]
// fn subscribe_logging(window: tauri::Window, state: tauri::State<State>) {
//     tokio::spawn()
// }

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
