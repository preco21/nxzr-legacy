use bytes::Buf;
use thiserror::Error;
use tokio::{io::BufReader, process::Command};

use crate::sock::Address;

#[derive(Clone, Error, Debug)]
pub enum HelperError {
    #[error("failed to execute a command: {0}")]
    CommandFailed(String),
    #[error("internal error: {0}")]
    Internal(HelperInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum HelperInternalError {
    #[error("utf8: {0}")]
    Utf8Error(std::str::Utf8Error),
    #[error("io: {0}")]
    Io(std::io::ErrorKind),
    #[error("bluer: {0}")]
    Bluer(bluer::ErrorKind),
}

impl From<std::str::Utf8Error> for HelperError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Internal(HelperInternalError::Utf8Error(err))
    }
}

impl From<std::io::Error> for HelperError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(HelperInternalError::Io(err.kind()))
    }
}

impl From<bluer::Error> for HelperError {
    fn from(err: bluer::Error) -> Self {
        Self::Internal(HelperInternalError::Bluer(err.kind))
    }
}

#[tracing::instrument(target = "helper")]
pub async fn set_adapter_address(
    adapter_name: String,
    address: Address,
) -> Result<(), HelperError> {
    tracing::info!(
        "resetting the bluetooth adapter ({:?}) with address `{:?}`",
        adapter_name,
        address
    );
    // Set the bluetooth adapter address by adapter name.
    //
    // The user will need to install `apt-get install bluez-utils`.
    run_command({
        let mut cmd = Command::new("bdaddr");
        cmd.args(&["-i", adapter_name.as_ref(), address.to_string().as_ref()]);
        cmd
    })
    .await?;
    // Reset the bluetooth adapter by running `hciconfig`.
    run_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&[adapter_name.as_ref(), "reset"]);
        cmd
    })
    .await?;
    // Restart bluetooth service.
    systemctl::restart("bluetooth.service")?;
    Ok(())
}

pub async fn run_command(mut command: Command) -> Result<(), HelperError> {
    let output = command.output().await?;
    if !output.status.success() {
        return Err(HelperError::CommandFailed(
            std::str::from_utf8(output.stderr.as_ref())?.to_owned(),
        ));
    }
    Ok(())
}
