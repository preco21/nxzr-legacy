use crate::config;
use std::{io, path::Path};
use tokio::fs;

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
pub struct TracingChannelWriter {
    writer_tx: mpsc::UnboundedSender<String>,
}

impl TracingChannelWriter {
    pub fn new(writer_tx: mpsc::UnboundedSender<String>) -> Self {
        Self { writer_tx }
    }
}

impl io::Write for TracingChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let json = String::from_utf8_lossy(buf).into_owned();
        let _ = self.writer_tx.send(json);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for TracingChannelWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
