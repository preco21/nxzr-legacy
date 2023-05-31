use anyhow::Result;
use std::io;
use tempfile::TempDir;
use thiserror::Error;

use crate::common;

const INSTALL_NXZR_SCRIPT: &str = include_str!("scripts/install-nxzr.ps1");

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("setup failed")]
    SetUpFailed,

    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("command failed: {0}")]
    Command(#[from] common::SystemCommandError),
}

#[tracing::instrument(target = "application_bootstrap")]
pub async fn install_system_requirements() -> Result<(), BootstrapError> {
    tracing::info!("installing system requirements");
    // Create a file from embedded install script to temp directory.
    let dir = TempDir::new()?;
    // let script_path = dir.path().join("install.ps1");
    // let mut script_file = File::create(&script_path).await?;
    // script_file
    //     .write_all(INSTALL_NXZR_SCRIPT.as_bytes())
    //     .await?;
    // drop(script_file);
    // tracing::info!("running command...");
    // // FIXME: this will be closed immediately...
    // common::run_system_command({
    //     let mut cmd = Command::new("powershell.exe");
    //     cmd.args(&[
    //         "-NoLogo",
    //         "-NonInteractive",
    //         "-WindowStyle",
    //         "Normal",
    //         "-File",
    //         str_path,
    //     ]);
    //     cmd
    // })
    // .await?;
    Ok(())
}

/// Bootstraps the program.
///
/// You can put whatever setup logic to this routine, however this function will
/// always be called at application startup and the main routine will wait until
/// it's complete.
///
/// Which means you should not put any long-running tasks here.
#[tracing::instrument(target = "application_bootstrap")]
pub async fn bootstrap_program() -> Result<(), BootstrapError> {
    let Some(dirs) = common::get_app_dirs() else {
        return Err(BootstrapError::SetUpFailed);
    };
    // Create new global config dirs.
    common::mkdir_p(dirs.config_dir()).await?;
    common::mkdir_p(dirs.data_dir()).await?;
    Ok(())
}

/// These scripts are responsible checking / installing infrastructures and
/// system requirements that is required to run NXZR for the current system.

// FIMXE: 1. import with wsl version 2...
// 2. make sure kernel image
// 3. import

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
    // nxzr data 폴더 커널 확인
    // wsl.conf 확인
    // WSL 이미지 존재 확인
    // WSL 켜서 드라이버 확인 (블루투스 agent 작동 여부 확인)
}

/// Step 2. Installs the NXZR agent for the current system.
pub async fn step_2_install_nxzr_agent() {
    // 커널 다운로드
    // wsl.conf 생성
    // wsl 이미지 생성 (비밀번호 설정)
    // NXZR Server 다운로드 및 적재
    // nxzr_server setup --install 실행
    // WSL 재시작
    // nxzr_server setup --config 실행
}

// 1. WSL 설치
// ㄴ

// 2.

// 시작할 때 1번 호출, nxzr 설치 경로는 wsl내 고정
pub async fn prepare_daemon() -> Result<(), BootstrapError> {
    // 6. run nxzr_server setup --mode=config
    // 7. enable usbipd using usbipd_attach() below.
    Ok(())
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
    Ok(())
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
    Ok(())
}
