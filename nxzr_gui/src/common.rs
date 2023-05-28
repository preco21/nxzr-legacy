use crate::config;
use std::{io, path::Path};
use tokio::fs;

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
