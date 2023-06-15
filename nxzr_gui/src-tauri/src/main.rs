// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tokio::sync::{mpsc, Mutex};
use tracing_subscriber::prelude::*;

mod commands;

struct AsyncProcInputTx {
    inner: Mutex<mpsc::Sender<String>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let module_filter =
        tracing_subscriber::filter::Targets::new().with_target("nxzr_gui", tracing::Level::TRACE);
    let subscriber = tracing_subscriber::registry().with(module_filter).with(
        tracing_subscriber::fmt::Layer::default()
            .event_format(tracing_subscriber::fmt::format().json()),
    );
    tracing::subscriber::set_global_default(subscriber)?;

    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let (async_proc_input_tx, async_proc_input_rx) = mpsc::channel(1);

    tauri::Builder::default()
        .manage(AsyncProcInputTx {
            inner: Mutex::new(async_proc_input_tx),
        })
        .invoke_handler(tauri::generate_handler![js2rs, greet])
        .setup(|app| {
            let app_handle = app.handle();
            tokio::spawn(async move { async_process_model(async_proc_input_rx, app_handle).await });

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

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

async fn async_process_model(
    mut input_rx: mpsc::Receiver<String>,
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    while let Some(input) = input_rx.recv().await {
        let output = input;
        rs2js(output, &app_handle);
    }
    Ok(())
}

fn rs2js<R: tauri::Runtime>(message: String, manager: &impl Manager<R>) {
    tracing::info!(?message, "rs2js");
    manager
        .emit_all("rs2js", format!("rs: {}", message))
        .unwrap();
}

#[tauri::command]
async fn js2rs(message: String, state: tauri::State<'_, AsyncProcInputTx>) -> Result<(), String> {
    tracing::info!(?message, "js2rs");
    let async_proc_input_tx = state.inner.lock().await;
    async_proc_input_tx
        .send(message)
        .await
        .map_err(|e| e.to_string())
}
