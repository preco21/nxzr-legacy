use thiserror::Error;
use tokio::process::Command;

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
    #[error("io: {kind} {message}")]
    Io {
        kind: std::io::ErrorKind,
        message: String,
    },
}

impl From<std::str::Utf8Error> for HelperError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Internal(HelperInternalError::Utf8Error(err))
    }
}

impl From<std::io::Error> for HelperError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(HelperInternalError::Io {
            kind: err.kind(),
            message: err.to_string(),
        })
    }
}

#[tracing::instrument(target = "helper")]
pub(crate) async fn set_adapter_address(
    adapter_name: &str,
    address: bluer::Address,
) -> Result<(), HelperError> {
    tracing::info!(
        "resetting Bluetooth adapter ({}) with address \"{:?}\".",
        adapter_name,
        address
    );
    // Set Bluetooth adapter address by adapter name.
    //
    // The user will need to install `apt-get install bluez-utils`.
    run_command({
        let mut cmd = Command::new("bdaddr");
        cmd.args(&["-i", adapter_name, address.to_string().as_ref()]);
        cmd
    })
    .await?;
    // Reset the bluetooth adapter by running `hciconfig`.
    run_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&[adapter_name, "reset"]);
        cmd
    })
    .await?;
    // Restart Bluetooth service.
    restart_bluetooth_service()?;
    Ok(())
}

#[tracing::instrument(target = "helper")]
pub(crate) async fn set_device_class(adapter_name: &str, class: u32) -> Result<u32, HelperError> {
    let str_class: String = format!("0x{:X}", class);
    tracing::info!(
        "setting device class of adapter {:?} to {:?}.",
        adapter_name,
        str_class.as_str()
    );
    run_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&[adapter_name, "class", str_class.as_str()]);
        cmd
    })
    .await?;
    Ok(class)
}

pub fn restart_bluetooth_service() -> Result<(), HelperError> {
    systemctl::restart("bluetooth.service")?;
    Ok(())
}

pub(crate) async fn run_command(mut command: Command) -> Result<(), HelperError> {
    let output = command.output().await?;
    if !output.status.success() {
        return Err(HelperError::CommandFailed(
            std::str::from_utf8(output.stderr.as_ref())?.to_owned(),
        ));
    }
    Ok(())
}
