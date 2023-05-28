use anyhow::Result;
use std::{
    fs,
    io::{self, Stderr, Stdout},
    path::Path,
    process::Stdio,
};
use tempfile::TempDir;
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};

use crate::common;

const INSTALL_NXZR_SCRIPT: &str = include_str!("scripts/install-nxzr.ps1");

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("setup failed")]
    SetUpFailed,

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
pub async fn install_system_requirements() -> Result<(), BootstrapError> {
    tracing::info!("installing system requirements");
    // Create a file from embedded install script to temp directory.
    let dir = TempDir::new()?;
    let script_path = dir.path().join("install.ps1");
    let mut script_file = File::create(&script_path).await?;
    script_file
        .write_all(INSTALL_NXZR_SCRIPT.as_bytes())
        .await?;
    let Some(str_path) = script_path.to_str() else {
        return Err(BootstrapError::NxzrInstallFailed("unable to convert script file path".to_owned()));
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

/// Bootstraps the program.
///
/// You can put whatever setup logic to this routine, however this function will
/// always be called at application startup and the main routine will wait until
/// it's complete.
///
/// Which means you should not put any long-running tasks here.
pub async fn bootstrap_program() -> Result<(), BootstrapError> {
    let Some(dirs) = common::get_app_dirs() else {
        return Err(BootstrapError::SetUpFailed);
    };
    // Create new global config dirs.
    common::mkdir_p(dirs.cache_dir()).await?;
    common::mkdir_p(dirs.config_dir()).await?;
    common::mkdir_p(dirs.data_dir()).await?;
    Ok(())
}

/// These scripts are responsible checking / installing infrastructures and
/// system requirements that is required to run NXZR for the current system.

/// Step 1. Checks for WSL infrastructure installation.
pub async fn step_1_check_wsl_infrastructure() {
    // 1. WSL 설치 여부
    // 2. usbipd-win 설치 여부
}

/// Step 1. Installs WSL infrastructure for the current system.
pub async fn step_1_install_wsl_infrastructure() {
    // install-nxzr.ps1 실행
    // 후 아마 재시작 필요, bios Virtualization 활성화 필요
}

/// Step 2. Checks for NXZR agent installation and its configuration.
pub async fn step_2_check_nxzr_agent() {
    // nxzr 폴더 커널 확인
    // wsl.conf 확인
    // WSL 이미지 존재 확인
    // WSL 켜서 드라이버 확인
}

/// Step 2. Installs the NXZR agent for the current system.
pub async fn step_2_install_nxzr_agent() {
    // wsl.conf 생성
    // wsl 이미지 생성
    //
}

// 1. WSL 설치
// ㄴ

// 2.

// 시작할 때 1번 호출, nxzr 설치 경로는 wsl내 고정
pub async fn prepare_daemon() -> Result<(), BootstrapError> {
    // 6. run nxzr_server setup --mode=config
    // 7. enable usbipd using usbipd_attach() below.
}

pub async fn check_wsl_installed() {
    // 1. Check WSL installed.
    // 2. WSL version check if it's > 2
    // 3. check if nxzr-agent-v1 WSL image is ready
    // 4. check if usbipd installed (see below)}
    // 5. check if kernel is there
    // 6. check wsl config is ready -- we cannot fix this as we need to shutdown the wsl if config is not properly setup.
    // 7. check if bluetooth driver is ready
}

pub async fn check_nxzr_installed() {
    // 1. check if nxzr_server is ready
    // 2. run "nxzr_server check" command
}

pub async fn install_wsl() {
    // FIXME: run install-nxzr.ps1 script for the moment...
    // Restart computer...
}

pub async fn install_ubuntu() -> Result<(), BootstrapError> {
    // # FIXME: Create an image with Ubuntu. (set password...)
    // # FIXME:
}

pub async fn install_nxzr() {
    // install nxzr_server command
    // run nxzr_server install
    // restart wsl
    // run nxzr_server setup
}

// FIXME: move this into dedicated module
pub async fn usbipd_attach() -> Result<(), BootstrapError> {
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
