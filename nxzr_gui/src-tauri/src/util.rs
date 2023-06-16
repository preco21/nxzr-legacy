use crate::config;
use std::{io, path::Path};
use tokio::{fs, sync::mpsc};
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

pub async fn mkdir_p<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<()> {
    if let Err(e) = fs::create_dir_all(path).await {
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e);
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct TracingChannelWriter<T: From<String> + Clone> {
    writer_tx: mpsc::Sender<T>,
}

impl<T: From<String> + Clone> TracingChannelWriter<T> {
    pub fn new(writer_tx: mpsc::Sender<T>) -> Self {
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
