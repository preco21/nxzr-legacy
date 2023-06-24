use crate::config;
use command_group::{AsyncCommandGroup, AsyncGroupChild};
use std::{
    io::{self, SeekFrom},
    path::Path,
    process::Stdio,
    sync::Arc,
};
use tempfile::TempDir;
use thiserror::Error;
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
    process::{ChildStderr, ChildStdout, Command},
    sync::mpsc,
    task::{JoinError, JoinSet},
    time,
};
use tracing_subscriber::fmt::MakeWriter;

pub fn get_app_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from(config::QUALIFIER, config::ORGANIZATION, config::APP_NAME)
}

pub async fn directory_exists<P: AsRef<Path> + ?Sized>(path: &P) -> bool {
    match fs::metadata(path).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

pub async fn file_exists<P: AsRef<Path> + ?Sized>(path: &P) -> bool {
    match fs::metadata(path).await {
        Ok(metadata) => metadata.is_file(),
        Err(_) => false,
    }
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

pub async fn create_file_from_raw_bytes(path: &Path, raw_bytes: &[u8]) -> io::Result<()> {
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

pub async fn run_system_command(mut command: Command) -> Result<String, SystemCommandError> {
    command.kill_on_drop(true);
    let output = command.output().await?;
    if !output.status.success() {
        return Err(SystemCommandError::CommandFailed(
            std::str::from_utf8(&output.stderr)?.to_owned(),
        ));
    }
    Ok(std::str::from_utf8(&output.stdout)?.to_owned())
}

pub async fn spawn_system_command(
    mut command: Command,
) -> Result<
    (
        AsyncGroupChild,
        BufReader<ChildStdout>,
        BufReader<ChildStderr>,
    ),
    SystemCommandError,
> {
    command.kill_on_drop(true);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = command.group_spawn()?;
    let stdout = child
        .inner()
        .stdout
        .take()
        .ok_or_else(|| SystemCommandError::CommandFailed("failed to get stdout".into()))?;
    let stderr = child
        .inner()
        .stderr
        .take()
        .ok_or_else(|| SystemCommandError::CommandFailed("failed to get stderr".into()))?;
    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);
    Ok((child, stdout_reader, stderr_reader))
}

pub struct RunPowershellScriptHandle {
    _close_rx: mpsc::Receiver<()>,
    _temp_dir_handle: TempDir,
}

pub async fn run_powershell_script(
    script: &str,
    args: Option<Vec<String>>,
    privileged: bool,
) -> Result<
    (
        mpsc::UnboundedReceiver<Result<String, SystemCommandError>>,
        RunPowershellScriptHandle,
    ),
    SystemCommandError,
> {
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
            SystemCommandError::CommandFailed("failed to convert log file path".into())
        })?;
        #[rustfmt::skip]
        let wrapped_script = trim_string_whitespace(format!(r#"
            Start-Transcript -Force -Path {log_path_str} | Out-Null
            {script}
            Stop-Transcript | Out-Null
        "#));
        let wrapper_script_path = dir.path().join("script-runas.ps1");
        create_file_from_raw_bytes(&wrapper_script_path, wrapped_script.as_bytes()).await?;
        let wrapped_path_str = wrapper_script_path.to_str().ok_or_else(|| {
            SystemCommandError::CommandFailed("failed to convert wrapper script file path".into())
        })?;
        tracing::info!(
            "running PowerShell script temporarily created at: {}",
            wrapped_path_str
        );
        let joined_args = match args {
            Some(args) => format!(" {}", args.join(" ")),
            None => "".into(),
        };
        tracing::info!("joined args: {}", joined_args);
        let cmd_str = format!(
            "$p = Start-Process -FilePath 'powershell.exe' -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"{wrapped_path_str}\"{joined_args}' -Wait -Verb RunAs -WindowStyle Hidden -PassThru; if ($p.ExitCode -ne 0) {{ throw \"process exited with non-zero status code\" }}"
        );
        // Touch the log file.
        File::create(&log_path).await?;
        // Spawn a task to observe logs.
        let (out_tx, out_rx) = mpsc::unbounded_channel();
        let (close_tx, close_rx) = mpsc::channel::<()>(1);
        let mut set: JoinSet<Result<(), SystemCommandError>> = JoinSet::new();
        set.spawn({
            let out_tx = out_tx.clone();
            async move {
                let mut bytes: Vec<u8> = vec![];
                let mut position: usize = 0;
                loop {
                    let mut log_file = File::open(&log_path).await?;
                    log_file.seek(SeekFrom::Start(position as u64)).await?;
                    bytes.truncate(0);
                    position += log_file.read_to_end(&mut bytes).await?;
                    let content = String::from_utf8_lossy(&bytes[..]);
                    if !content.is_empty() {
                        let _ = out_tx.send(Ok(content.into()));
                    }
                    time::sleep(time::Duration::from_millis(400)).await;
                }
            }
        });
        set.spawn(async move {
            run_system_command({
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
            })
            .await?;
            Ok(())
        });
        tokio::spawn(async move {
            tokio::select! {
                Some(res) = set.join_next() => {
                    match res {
                        Ok(Err(err)) => { let _ = out_tx.send(Err(err)); }
                        Err(err) => { let _ = out_tx.send(Err(err.into())); }
                        _ => {}
                    }
                },
                _ = close_tx.closed() => {},
            }
            tracing::info!("terminating the powershell script process.");
            set.shutdown().await;
        });
        Ok((
            out_rx,
            RunPowershellScriptHandle {
                _close_rx: close_rx,
                _temp_dir_handle: dir,
            },
        ))
    } else {
        let script_path = dir.path().join("script.ps1");
        let script_path_str = script_path.to_str().ok_or_else(|| {
            SystemCommandError::CommandFailed("failed to convert script file path".into())
        })?;
        create_file_from_raw_bytes(&script_path, script.as_bytes()).await?;
        tracing::info!(
            "running PowerShell script temporarily created at: {}",
            script_path_str
        );
        let mut cmd_args: Vec<String> = vec![
            "-NonInteractive".into(),
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            script_path_str.into(),
        ];
        if let Some(args) = args {
            cmd_args.extend(args);
        }
        let mut cmd = Command::new("powershell.exe");
        cmd.args(cmd_args);
        let (mut child, stdout_reader, stderr_reader) = spawn_system_command(cmd).await?;
        let mut combined_lines = stdout_reader.chain(stderr_reader).lines();
        let (close_tx, close_rx) = mpsc::channel(1);
        let (out_tx, out_rx) = mpsc::unbounded_channel();
        let mut set: JoinSet<Result<(), SystemCommandError>> = JoinSet::new();
        set.spawn({
            let out_tx = out_tx.clone();
            async move {
                while let Some(line) = combined_lines.next_line().await? {
                    let _ = out_tx.send(Ok(line));
                }
                Ok::<(), SystemCommandError>(())
            }
        });
        set.spawn(async move {
            let status = child.wait().await?;
            if !status.success() {
                return Err(SystemCommandError::CommandFailed(format!(
                    "process exited with non-zero status code: {}",
                    status
                        .code()
                        .map(|code| code.to_string())
                        .unwrap_or("n/a".to_owned())
                )));
            }
            Ok::<(), SystemCommandError>(())
        });
        tokio::spawn(async move {
            tokio::select! {
                Some(res) = set.join_next() => {
                    match res {
                        Ok(Err(err)) => { let _ = out_tx.send(Err(err)); }
                        Err(err) => { let _ = out_tx.send(Err(err.into())); }
                        _ => {}
                    }
                },
                _ = close_tx.closed() => {},
            }
            tracing::info!("terminating the powershell script process.");
            set.shutdown().await;
        });
        Ok((
            out_rx,
            RunPowershellScriptHandle {
                _close_rx: close_rx,
                _temp_dir_handle: dir,
            },
        ))
    }
}

#[derive(Debug, Clone)]
pub struct TracingChannelWriter<T: From<String> + Clone> {
    writer_tx: Arc<mpsc::Sender<T>>,
}

impl<T: From<String> + Clone> TracingChannelWriter<T> {
    pub fn new(writer_tx: Arc<mpsc::Sender<T>>) -> Self {
        Self { writer_tx }
    }
}

impl<T: From<String> + Clone> io::Write for TracingChannelWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let json = String::from_utf8_lossy(buf).into_owned();
        // Allow failure to send if the channel capacity is full.
        let _ = self.writer_tx.try_send(json.into());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a, T: From<String> + Clone> MakeWriter<'a> for TracingChannelWriter<T> {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
