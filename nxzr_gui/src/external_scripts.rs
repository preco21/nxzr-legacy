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

// // One for windows
// pub async fn prepare_system_requirements() -> Result<(), Error> {
//     // 1. check wsl installed
//     // 2. check if system can run wsl -> vm requirements (이건 그냥 체크되나? 1에서?)
//     // 3. wsl version check if it's v2
//     // 4. check usbipd installed -> maybe just include the binary
//     // ㄴ https://github.com/dorssel/usbipd-win/wiki/WSL-support
//     // 5. check wsl config is ready -> otherwise, install one
//     // ㄴ check /etc/wsl.conf is ready -> otherwise, set one and restart vm (wait 8 sec)
//     // 6. disable windows bt
//     // 7. enable usbipd
//     Ok(())
// }

// // One for linux
// pub async fn ensure_system_requirements() -> Result<(), SysCheckError> {
//     // sudo systemctl daemon-reload
//     // sudo systemctl restart bluetooth
// }

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
