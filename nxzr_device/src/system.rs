use crate::Address;
use tokio::{process::Command, time::error::Elapsed};

#[derive(Clone, Debug, thiserror::Error)]
pub enum SysCheckError {
    #[error("privilege error, this program is required to run as root user")]
    RootPrivilegeRequired,
    #[error("failed to check Bluetooth service, possibly the driver for Bluetooth is not loaded or the service is dead")]
    BluetoothNotAvailable,
    #[error("cli tool presence check failed: {0}")]
    CliToolFailed(String),
}

pub async fn check_privileges() -> Result<(), SysCheckError> {
    // Check if the program has been run as root.
    if sudo::check() != sudo::RunningAs::Root {
        return Err(SysCheckError::RootPrivilegeRequired);
    }
    Ok(())
}

pub async fn check_system_requirements() -> Result<(), SysCheckError> {
    tracing::info!("checking system requirements...");
    // Check Bluetooth service is available.
    //
    // This will fail if there's no Kernel support for the Bluetooth drivers. Which essentially leads the Bluetooth service to crash.
    check_bluetooth_service()
        .await
        .map_err(|_| SysCheckError::BluetoothNotAvailable)?;
    // Check if `hciconfig` exists.
    //
    // This will be used to manipulate HCI settings like MAC address, device classes, etc...
    run_system_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&["-h"]);
        cmd
    })
    .await
    .map_err(|_| SysCheckError::CliToolFailed("hciconfig".into()))?;
    // FIXME: Maybe we do not need to change bdaddr
    // Revisit for revise: https://github.com/thxomas/bdaddr
    // Check if `bdaddr` exists
    // helper::run_command({
    //     let mut cmd = Command::new("bdaddr");
    //     cmd.args(&["-h"]);
    //     cmd
    // })
    // .await
    // .map_err(|_| SysCheckError::CliToolFailed("bdaddr".into()))?;
    Ok(())
}

#[tracing::instrument(target = "system")]
pub(crate) async fn set_adapter_address(
    adapter_name: &str,
    address: Address,
) -> Result<(), SystemCommandError> {
    tracing::info!(
        "resetting Bluetooth adapter ({}) with address \"{:?}\".",
        adapter_name,
        address
    );
    // FIXME: Maybe we do not need to change bdaddr
    // Revisit for revise: https://github.com/thxomas/bdaddr
    // Reset Bluetooth adapter address by adapter name.
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
    restart_bluetooth_service().await?;
    Ok(())
}

#[tracing::instrument(target = "system")]
pub(crate) async fn set_device_class(
    adapter_name: &str,
    class: u32,
) -> Result<u32, SystemCommandError> {
    let class_str = format!("0x{:X}", class);
    tracing::info!(
        "setting device class of adapter {:?} to {:?}.",
        adapter_name,
        class_str
    );
    run_system_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&[adapter_name, "class", class_str.as_str()]);
        cmd
    })
    .await?;
    Ok(class)
}

#[tracing::instrument(target = "system")]
pub async fn check_bluetooth_service() -> Result<(), SystemCommandError> {
    tracing::info!("checking for Bluetooth service available.");
    run_system_command({
        let mut cmd = Command::new("service");
        cmd.args(&["bluetooth", "status"]);
        cmd
    })
    .await?;
    Ok(())
}

#[tracing::instrument(target = "system")]
pub async fn restart_bluetooth_service() -> Result<(), SystemCommandError> {
    tracing::info!("attempting to restart Bluetooth service.");
    run_system_command({
        let mut cmd = Command::new("service");
        cmd.args(&["bluetooth", "restart"]);
        cmd
    })
    .await?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum SystemCommandError {
    #[error("failed to execute a command: {0}")]
    CommandFailed(String),
    #[error("timeout: {0}")]
    Timeout(#[from] Elapsed),
    #[error("internal error: {0}")]
    Internal(SystemCommandInternalError),
}

#[derive(Debug, thiserror::Error)]
pub enum SystemCommandInternalError {
    #[error("utf8: {0}")]
    Utf8Error(std::str::Utf8Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<std::str::Utf8Error> for SystemCommandError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Internal(SystemCommandInternalError::Utf8Error(err))
    }
}

impl From<std::io::Error> for SystemCommandError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.into())
    }
}

async fn run_system_command(mut command: Command) -> Result<(), SystemCommandError> {
    command.kill_on_drop(true);
    let output = command.output().await?;
    if !output.status.success() {
        return Err(SystemCommandError::CommandFailed(
            std::str::from_utf8(&output.stderr)?.to_owned(),
        ));
    }
    Ok(())
}
