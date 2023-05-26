use std::io;
use tempfile::TempDir;
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt};

const INSTALL_NXZR_SCRIPT: &str = include_str!("scripts/install-nxzr.ps1");

#[derive(Error, Debug)]
pub enum ExternalScriptError {
    #[error("failed to install server: {0}")]
    ServerInstallFailed(String),
    #[error("failed to setup config: {0}")]
    ConfigSetupFailed(String),
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

pub async fn install_system_requirements() -> Result<(), ExternalScriptError> {
    // Create a file from embedded install script to temp directory.
    let dir = TempDir::new()?;
    println!("{:?}", dir);
    let script_file_path = dir.path().join("install.ps1");
    let mut file = File::create(script_file_path).await?;
    file.write_all(INSTALL_NXZR_SCRIPT.as_bytes()).await?;
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
