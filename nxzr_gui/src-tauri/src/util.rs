use crate::config;
use command_group::{AsyncCommandGroup, AsyncGroupChild};
use multiinput::{DeviceType, RawEvent, RawInputManager};
use serde_json::json;
use std::{
    io::{self, SeekFrom},
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use tauri::Manager;
use tempfile::TempDir;
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
    process::{ChildStderr, ChildStdout, Command},
    sync::mpsc,
    task::JoinError,
    time::{self, Duration, Instant},
};
use tracing_subscriber::fmt::MakeWriter;

pub fn get_resource(handle: &tauri::AppHandle, name: &str) -> Option<PathBuf> {
    handle
        .path_resolver()
        .resolve_resource(format!("resources/{}", name))
}

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

#[derive(Debug, thiserror::Error)]
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
    let output = command.kill_on_drop(true).output().await?;
    if !output.status.success() {
        let err_str = std::str::from_utf8(&output.stderr).unwrap_or("N/A");
        return Err(SystemCommandError::CommandFailed(err_str.to_owned()));
    }
    let out_str = std::str::from_utf8(&output.stdout).unwrap_or("N/A");
    Ok(out_str.to_owned())
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
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .group_spawn()?;
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

pub async fn run_powershell_script_privileged(
    script: &str,
    args: Option<Vec<String>>,
    output_tx: Option<mpsc::UnboundedSender<String>>,
) -> Result<(), SystemCommandError> {
    // Create a file from embedded install script to temp directory.
    let dir = TempDir::new()?;
    // Current version of the PowerShell does not allow you to redirect
    // "standard output" of the elevated spawned script process.
    //
    // So here we are using a hack to retrieve the output of the script. The
    // idea is that, using "Start-Transcript" to log output into a file,
    // then the program continuously reads it until the script finishes.
    let log_path = dir.path().join("script-output.txt");
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
    if let Some(output_tx) = output_tx {
        let stream_read_handle = tokio::spawn(async move {
            let mut bytes: Vec<u8> = vec![];
            let mut position: usize = 0;
            loop {
                let mut log_file = File::open(&log_path).await?;
                log_file.seek(SeekFrom::Start(position as u64)).await?;
                bytes.truncate(0);
                position += log_file.read_to_end(&mut bytes).await?;
                let content = String::from_utf8_lossy(&bytes[..]);
                if !content.is_empty() {
                    let _ = output_tx.send(content.into());
                }
                time::sleep(Duration::from_millis(400)).await;
            }
            #[allow(unreachable_code)]
            Ok::<(), SystemCommandError>(())
        });
        let res = run_system_command({
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
        .await;
        stream_read_handle.abort();
        let _ = stream_read_handle.await;
        res?;
    } else {
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
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn run_powershell_script(
    script: &str,
    args: Option<Vec<String>>,
    output_tx: Option<mpsc::UnboundedSender<String>>,
) -> Result<(), SystemCommandError> {
    // Create a file from embedded install script to temp directory.
    let dir = TempDir::new()?;
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
    let (child, stdout_reader, stderr_reader) = spawn_system_command(cmd).await?;
    if let Some(output_tx) = output_tx {
        let mut combined_lines = stdout_reader.chain(stderr_reader).lines();
        let stream_read_handle = tokio::spawn(async move {
            while let Some(line) = combined_lines.next_line().await? {
                let _ = output_tx.send(line);
            }
            Ok::<(), SystemCommandError>(())
        });
        let output = child.wait_with_output().await?;
        stream_read_handle.abort();
        let _ = stream_read_handle.await;
        if !output.status.success() {
            return Err(SystemCommandError::CommandFailed(
                std::str::from_utf8(&output.stderr)?.to_owned(),
            ));
        }
    } else {
        let output = child.wait_with_output().await?;
        if !output.status.success() {
            return Err(SystemCommandError::CommandFailed(
                std::str::from_utf8(&output.stderr)?.to_owned(),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct TracingJsonLogData {
    pub timestamp: String,
    pub level: String,
    pub fields: TracingJsonLogDataFields,
    pub target: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct TracingJsonLogDataFields {
    pub message: String,
}

pub fn parse_tracing_json_log_data(json: &str) -> Result<TracingJsonLogData, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn format_tracing_json_log_data(data: &TracingJsonLogData) -> String {
    format!(
        "[{}] [{}] [{}]: {}",
        data.timestamp, data.level, data.target, data.fields.message
    )
}

pub fn forward_trace_json_log_data(raw: &str) {
    match parse_tracing_json_log_data(raw) {
        Ok(data) => match data.level.as_str() {
            "ERROR" => tracing::error!("[child]: {}", format_tracing_json_log_data(&data)),
            "WARN" => tracing::warn!("[child]: {}", format_tracing_json_log_data(&data)),
            "INFO" => tracing::info!("[child]: {}", format_tracing_json_log_data(&data)),
            "DEBUG" => tracing::debug!("[child]: {}", format_tracing_json_log_data(&data)),
            "TRACE" => tracing::trace!("[child]: {}", format_tracing_json_log_data(&data)),
            _ => tracing::info!("[child]: {}", format_tracing_json_log_data(&data)),
        },
        Err(_) => tracing::error!("[child raw]: {}", raw),
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

pub fn register_mouse_event_emitter(app: tauri::AppHandle) {
    let mut manager = RawInputManager::new().unwrap();
    manager.register_devices(DeviceType::Mice);
    let mut acc_x = 0;
    let mut acc_y = 0;
    let mut now = Instant::now();
    tokio::task::spawn_blocking(move || loop {
        let elapsed = now.elapsed();
        if let Some(event) = manager.get_event() {
            match event {
                RawEvent::MouseMoveEvent(_, x, y) => {
                    acc_x += x;
                    acc_y += y;
                    if elapsed > Duration::from_millis(1) {
                        let _ =
                            app.emit_all("raw_input:mousemove", json!({ "x": acc_x, "y": acc_y }));
                        acc_x = 0;
                        acc_y = 0;
                        now = Instant::now();
                    }
                }
                _ => {}
            }
        }
        if elapsed >= Duration::from_millis(1) {
            let _ = app.emit_all("raw_input:mousemove", json!({ "x": acc_x, "y": acc_y }));
            acc_x = 0;
            acc_y = 0;
            now = Instant::now();
        }
    });
}
