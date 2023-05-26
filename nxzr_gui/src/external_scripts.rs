use anyhow::Result;
use std::{
    io::{self, Stderr, Stdout},
    path::Path,
    process::Stdio,
};
use tempfile::TempDir;
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};

const INSTALL_NXZR_SCRIPT: &str = include_str!("scripts/install-nxzr.ps1");

#[derive(Error, Debug)]
pub enum ExternalScriptError {
    #[error("failed to install NXZR: {0}")]
    NxzrInstallFailed(String),
    #[error("failed to setup config: {0}")]
    ConfigSetupFailed(String),
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("command failed: {0}")]
    Command(#[from] SystemCommandError),
}

#[tracing::instrument(target = "gui_scripts")]
pub async fn install_system_requirements() -> Result<(), ExternalScriptError> {
    tracing::info!("installing system requirements");
    // Create a file from embedded install script to temp directory.
    let dir = TempDir::new()?;
    let script_path = dir.path().join("install.ps1");
    let mut script_file = File::create(&script_path).await?;
    script_file
        .write_all(INSTALL_NXZR_SCRIPT.as_bytes())
        .await?;
    let Some(str_path) = script_path.to_str() else {
        return Err(ExternalScriptError::NxzrInstallFailed("unable to convert script file path".to_owned()));
    };
    drop(script_file);
    tracing::info!("created temporary install script at: {str_path}");
    tracing::info!("running command...");
    // FIXME: this will be closed immediately...
    run_system_command({
        let mut cmd = Command::new("powershell.exe");
        cmd.args(&[
            "-NoLogo",
            "-NonInteractive",
            "-WindowStyle",
            "Normal",
            "-File",
            str_path,
        ]);
        cmd
    })
    .await?;
    Ok(())
}

// 시작할 때 1번 호출, nxzr 설치 경로는 wsl내 고정
pub async fn prepare_daemon() -> Result<(), ExternalScriptError> {
    // 1. Check WSL installed.
    // 2. WSL version check if it's > 2
    // 3. check if usbipd installed (see below)
    // 4. check if kernel is there
    // 5. check wsl config is ready -- we cannot fix this as we need to shutdown the wsl if config is not properly setup.
    // 6. run nxzr_server setup --mode=config
    // 7. enable usbipd using usbipd_attach() below.
}

pub async fn install() -> Result<(), ExternalScriptError> {
    // FIXME: run install-nxzr.ps1 script for the moment...
}

// FIXME: move this into dedicated module
pub async fn usbipd_attach() -> Result<(), ExternalScriptError> {
    // 0. check if usbipd installed
    // A. usbipd query
    // B. usbipd attach
    // C. usbipd detach
    // ㄴ https://github.com/dorssel/usbipd-win/wiki/WSL-support
}

#[derive(Error, Debug)]
pub enum SystemCommandError {
    #[error("failed to execute a command: {0}")]
    CommandFailed(String),
    #[error("utf8: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

async fn run_system_command(mut command: Command) -> Result<(), SystemCommandError> {
    let output = command.output().await?;
    if !output.status.success() {
        return Err(SystemCommandError::CommandFailed(
            std::str::from_utf8(&output.stderr)?.to_owned(),
        ));
    }
    Ok(())
}
