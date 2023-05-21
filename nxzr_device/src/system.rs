use crate::Address;
use thiserror::Error;
use tokio::process::Command;

#[derive(Clone, Error, Debug)]
pub enum SysCheckError {
    #[error("privilege error, the program is required to run as root user")]
    RootPrivilegeRequired,
    #[error("systemctl check failed")]
    SysctlFailed,
    #[error("Bluetooth service check failed: {0}")]
    BluetoothFailed(String),
    #[error("dbus service check failed: {0}")]
    DBusFailed(String),
    #[error("cli tool presence check failed: {0}")]
    CliToolFailed(String),
}

pub async fn check_system_requirements() -> Result<(), SysCheckError> {
    // Check if the program has been run as root.
    if sudo::check() != sudo::RunningAs::Root {
        return Err(SysCheckError::RootPrivilegeRequired);
    }
    // Check if the Bluetooth service is active.
    if !systemctl::exists("bluetooth.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::BluetoothFailed(
            "Bluetooth service does not exist.".to_owned(),
        ));
    };
    if !systemctl::is_active("bluetooth.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::BluetoothFailed(
            "Bluetooth service is not active.".to_owned(),
        ));
    }
    // FIXME: maybe this is not platform agnostic
    // Check if the `dbus` service is active
    if !systemctl::exists("dbus.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::DBusFailed(
            "dbus service does not exist.".to_owned(),
        ));
    };
    if !systemctl::is_active("dbus.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::DBusFailed(
            "dbus service is not active.".to_owned(),
        ));
    }
    // Check if `hciconfig` exists
    run_system_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&["--h"]);
        cmd
    })
    .await
    .map_err(|_| SysCheckError::CliToolFailed("hciconfig".to_owned()))?;

    // FIXME: check for bluetooth ctl?

    // FIXME: maybe this is not platform agnostic
    // Check if `bdaddr` exists
    // helper::run_command({
    //     let mut cmd = Command::new("bdaddr");
    //     cmd.args(&["-h"]);
    //     cmd
    // })
    // .await
    // .map_err(|_| SysCheckError::CliToolFailed("bdaddr".to_owned()))?;
    Ok(())
}

#[tracing::instrument(target = "helper")]
pub(crate) async fn set_adapter_address(
    adapter_name: &str,
    address: Address,
) -> Result<(), SystemCommandError> {
    tracing::info!(
        "resetting Bluetooth adapter ({}) with address \"{:?}\".",
        adapter_name,
        address
    );
    // Set Bluetooth adapter address by adapter name.
    //
    // The user will need to install `apt-get install bluez-utils`.
    // FIXME: update to not use bdaddr
    // run_command({
    //     let mut cmd = Command::new("bdaddr");
    //     cmd.args(&["-i", adapter_name, &address.to_string()]);
    //     cmd
    // })
    // .await?;
    // Reset Bluetooth adapter by running `hciconfig`.
    run_system_command({
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
pub(crate) async fn set_device_class(
    adapter_name: &str,
    class: u32,
) -> Result<u32, SystemCommandError> {
    let str_class: String = format!("0x{:X}", class);
    tracing::info!(
        "setting device class of adapter {:?} to {:?}.",
        adapter_name,
        str_class.as_str()
    );
    run_system_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&[adapter_name, "class", str_class.as_str()]);
        cmd
    })
    .await?;
    Ok(class)
}

#[tracing::instrument(target = "helper")]
pub fn restart_bluetooth_service() -> Result<(), SystemCommandError> {
    systemctl::restart("bluetooth.service")?;
    Ok(())
}

#[derive(Clone, Error, Debug)]
pub enum SystemCommandError {
    #[error("failed to execute a command: {0}")]
    CommandFailed(String),
    #[error("internal error: {0}")]
    Internal(SystemCommandInternalError),
}

#[derive(Clone, Error, Debug)]
pub enum SystemCommandInternalError {
    #[error("utf8: {0}")]
    Utf8Error(std::str::Utf8Error),
    #[error("io: {kind}; {message}")]
    Io {
        kind: std::io::ErrorKind,
        message: String,
    },
}

impl From<std::str::Utf8Error> for SystemCommandError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Internal(SystemCommandInternalError::Utf8Error(err))
    }
}

impl From<std::io::Error> for SystemCommandError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(SystemCommandInternalError::Io {
            kind: err.kind(),
            message: err.to_string(),
        })
    }
}

async fn run_system_command(mut command: Command) -> Result<(), SystemCommandError> {
    let output = command.output().await?;
    if !output.status.success() {
        return Err(SystemCommandError::CommandFailed(
            std::str::from_utf8(&output.stderr)?.to_owned(),
        ));
    }
    Ok(())
}
