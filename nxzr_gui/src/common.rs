use crate::config;
use std::{io, path::Path};
use tempfile::TempDir;
use thiserror::Error;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub fn get_app_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from(config::QUALIFIER, config::ORGANIZATION, config::APP_NAME)
}

pub async fn mkdir_p<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<()> {
    if let Err(e) = fs::create_dir_all(path).await {
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e);
        }
    }
    Ok(())
}

pub async fn create_file_from_raw(path: &Path, raw_bytes: &[u8]) -> io::Result<()> {
    // A file handle created here will be unlinked after completing the routine
    // intentionally so that subsequent jobs can make progress on the file.
    let mut script_file = File::create(path).await?;
    script_file.write_all(raw_bytes).await?;
    Ok(())
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

pub async fn run_system_command(mut command: Command) -> Result<(), SystemCommandError> {
    let output = command.output().await?;
    if !output.status.success() {
        return Err(SystemCommandError::CommandFailed(
            std::str::from_utf8(&output.stderr)?.to_owned(),
        ));
    }
    Ok(())
}

pub async fn run_powershell_script(
    script: &str,
    privileged: bool,
) -> Result<(), SystemCommandError> {
    // Create a file from embedded install script to temp directory.
    let dir = TempDir::new()?;
    let script_path = dir.path().join("script.ps1");
    let Some(script_path_str) = script_path.to_str() else {
        return Err(SystemCommandError::CommandFailed("failed to convert script file path".to_string()));
    };
    create_file_from_raw(&script_path, script.as_bytes()).await?;
    tracing::info!(
        "running PowerShell script temporarily created at: {}",
        script_path_str
    );
    if privileged {
        // Current version of the PowerShell does not allow you to redirect
        // "standard output" of the elevated spawned script process.
        //
        // So here we are using a hack to retrieve the output of the script. The
        // idea is that, using "Start-Transcript" to log output into a file,
        // then the program continuously reads it until the script finishes.
        let uac_helper_logs_path = dir.path().join("output.ps1");
        let Some(logs_path_str) = uac_helper_logs_path.to_str() else {
            return Err(SystemCommandError::CommandFailed("failed to convert log file path".to_string()));
        };
        #[rustfmt::skip]
        let uac_helper_str = format!(r#"
            Set-StrictMode -Version Latest
            $ErrorActionPreference = "Stop"

            Start-Transcript -Path {logs_path_str}
            Start-Process -FilePath "powershell.exe" -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File {script_path_str}" -Wait -Verb RunAs
            Stop-Transcript

            powershell.exe -Command "&{{start-process powershell 'Start-Transcript -Path .\out.txt; write-host test; stop-transcript; read-host foobar' -Wait -Verb RunAs}}"
        "#);
        // FIXME: you can't do this, start transcript must be started from the final file...
        let wrapped_script_path = dir.path().join("uac-helper.ps1");
        create_file_from_raw(&wrapped_script_path, uac_helper_str.as_bytes()).await?;
        let Some(wrapped_path_str) = script_path.to_str() else {
            return Err(SystemCommandError::CommandFailed("failed to convert wrapper script file path".to_string()));
        };
        run_system_command({
            let mut cmd = Command::new("powershell.exe");
            cmd.args(&[
                "-NonInteractive",
                "-NoLogo",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                wrapped_path_str,
            ]);
            cmd
        })
        .await?;
        // common::run_system_command({
        //     let mut cmd = Command::new("powershell.exe");
        //     cmd.args(&[
        //         "-NoLogo",
        //         "-NonInteractive",
        //         "-NoProfile",
        //         "-ExecutionPolicy",
        //         "Bypass",
        //         "-WindowStyle",
        //         "Normal",
        //         "-File",
        //         str_path,
        //     ]);
        //     cmd
        // })
        // .await?;
    } else {
        // common::run_system_command({
        //     let mut cmd = Command::new("powershell.exe");
        //     cmd.args(&[
        //         "-NoLogo",
        //         "-NoProfile",
        //         "-NonInteractive",
        //         "-WindowStyle",
        //         "Normal",
        //         "-File",
        //         str_path,
        //     ]);
        //     cmd
        // })
        // .await?;
    }
    // FIXME: this will be closed immediately...
    run_system_command({
        let mut cmd = Command::new("powershell.exe");
        cmd.args(&[
            "-NoLogo",
            "-NonInteractive",
            "-WindowStyle",
            "Normal",
            "-File",
            script_path_str,
        ]);
        cmd
    })
    .await?;
    Ok(())
}
