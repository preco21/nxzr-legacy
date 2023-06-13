use crate::config;
use std::io::{self, SeekFrom};
use std::path::Path;
use std::process::Stdio;
use tempfile::TempDir;
use thiserror::Error;
use tokio::fs::{self, File};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tokio::sync::mpsc;
use tokio::task::JoinError;
use tokio::time;

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

pub fn trim_string_whitespace(input: String) -> String {
    input
        .trim()
        .lines()
        .map(|part| {
            part.trim()
                .split_inclusive(char::is_whitespace)
                .filter(|part| !part.trim().is_empty())
                .collect()
        })
        .collect::<Vec<String>>()
        .join("\n")
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
    #[error("join error: {0}")]
    JoinError(#[from] JoinError),
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

pub async fn spawn_system_command(
    mut command: Command,
) -> Result<(Child, BufReader<ChildStdout>, BufReader<ChildStderr>), SystemCommandError> {
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| SystemCommandError::CommandFailed("failed to get stdout".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| SystemCommandError::CommandFailed("failed to get stderr".to_string()))?;
    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);
    Ok((child, stdout_reader, stderr_reader))
}

pub async fn run_powershell_script(
    script: &str,
    args: Option<Vec<String>>,
    privileged: bool,
) -> Result<(), SystemCommandError> {
    // Create a file from embedded install script to temp directory.
    let dir = TempDir::new()?;
    if privileged {
        // Current version of the PowerShell does not allow you to redirect
        // "standard output" of the elevated spawned script process.
        //
        // So here we are using a hack to retrieve the output of the script. The
        // idea is that, using "Start-Transcript" to log output into a file,
        // then the program continuously reads it until the script finishes.
        let log_path = dir.path().join("output.txt");
        let log_path_str = log_path.to_str().ok_or_else(|| {
            SystemCommandError::CommandFailed("failed to convert log file path".to_string())
        })?;
        #[rustfmt::skip]
        let wrapped_script = trim_string_whitespace(format!(r#"
            Start-Transcript -Force -Path {log_path_str} | Out-Null
            {script}
            Stop-Transcript | Out-Null
        "#));
        let wrapper_script_path = dir.path().join("script-runas.ps1");
        create_file_from_raw(&wrapper_script_path, wrapped_script.as_bytes()).await?;
        let wrapped_path_str = wrapper_script_path.to_str().ok_or_else(|| {
            SystemCommandError::CommandFailed(
                "failed to convert wrapper script file path".to_string(),
            )
        })?;
        tracing::info!(
            "running PowerShell script temporarily created at: {}",
            wrapped_path_str
        );
        let joined_args = match args {
            Some(args) => format!(" {}", args.join(" ")),
            None => "".to_string(),
        };
        let cmd_str = format!(
            "$p = Start-Process -FilePath 'powershell.exe' -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"{wrapped_path_str}\"{joined_args}' -Wait -Verb RunAs -WindowStyle Hidden -PassThru; if ($p.ExitCode -ne 0) {{ throw \"process exited with non-zero status code\" }}"
        );
        // Touch the log file.
        File::create(&log_path).await?;
        // Spawn a task to observe logs.
        let (close_tx, close_rx) = mpsc::channel::<()>(1);
        let handle = tokio::spawn(async move {
            let mut bytes: Vec<u8> = vec![];
            let mut position: usize = 0;
            loop {
                tokio::select! {
                    res = async {
                        // HACK: In Windows, it seems a file does not read from
                        // the most latest changes when reusing the file handle
                        // that is already open.
                        //
                        // So, here we are just re-opening a file every loop
                        // round as a hack.
                        //
                        // For performance and resource implications, this
                        // function will only be called for the setup, so it
                        // will be just fine for the case.
                        let mut log_file = File::open(&log_path).await?;
                        log_file.seek(SeekFrom::Start(position as u64)).await?;
                        bytes.truncate(0);
                        position += log_file.read_to_end(&mut bytes).await?;
                        let content = String::from_utf8_lossy(&bytes[..]);
                        // FIXME: to return stream + handle instead
                        if !content.is_empty() {
                            print!("{}", content);
                        }
                        time::sleep(time::Duration::from_millis(400)).await;
                        Ok::<(), SystemCommandError>(())
                    } => res?,
                    _ = close_tx.closed() => break,
                }
            }
            Ok::<(), SystemCommandError>(())
        });
        let ret = tokio::select! {
            res = handle => res.map_err(|err| err.into()).and_then(|x| x),
            res = run_system_command({
                let mut cmd = Command::new("powershell.exe");
                cmd.args(&[
                    "-NonInteractive",
                    "-NoLogo",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    cmd_str.as_str(),
                ]);
                cmd
            }) => res,
        };
        // Cleanup background tasks.
        drop(close_rx);
        ret
    } else {
        let script_path = dir.path().join("script.ps1");
        let script_path_str = script_path.to_str().ok_or_else(|| {
            SystemCommandError::CommandFailed("failed to convert script file path".to_string())
        })?;
        create_file_from_raw(&script_path, script.as_bytes()).await?;
        tracing::info!(
            "running PowerShell script temporarily created at: {}",
            script_path_str
        );
        let mut cmd = Command::new("powershell.exe");
        cmd.args(&[
            "-NonInteractive",
            "-NoLogo",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_path_str,
        ]);
        let (mut child, stdout_reader, stderr_reader) = spawn_system_command(cmd).await?;
        let mut combined_lines = stdout_reader.chain(stderr_reader).lines();
        let handle = tokio::spawn(async move {
            let status = child.wait().await?;
            if !status.success() {
                return Err(SystemCommandError::CommandFailed(format!(
                    "process exited with non-zero status code: {}",
                    status
                        .code()
                        .map(|code| code.to_string())
                        .unwrap_or("n/a".to_string())
                )));
            }
            Ok::<(), SystemCommandError>(())
        });
        while let Some(line) = combined_lines.next_line().await? {
            // FIXME: to return stream + handle instead of println.
            println!("{}", line);
        }
        handle.await??;
        Ok(())
    }
}