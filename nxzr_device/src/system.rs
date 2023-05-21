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
    #[error("prepare failed")]
    PrepareFailed,
}

pub async fn check_privileges() -> Result<(), SysCheckError> {
    // Check if the program has been run as root.
    if sudo::check() != sudo::RunningAs::Root {
        return Err(SysCheckError::RootPrivilegeRequired);
    }
    Ok(())
}

pub async fn check_system_requirements() -> Result<(), SysCheckError> {
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
    // Check if the `dbus` service is active.
    if !systemctl::exists("dbus.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::DBusFailed(
            "DBus service does not exist.".to_owned(),
        ));
    };
    if !systemctl::is_active("dbus.service").map_err(|_| SysCheckError::SysctlFailed)? {
        return Err(SysCheckError::DBusFailed(
            "DBus service is not active.".to_owned(),
        ));
    }
    // Check if `hciconfig` exists.
    //
    // This will be used to manipulate HCI settings like MAC address, device classes, etc...
    run_system_command({
        let mut cmd = Command::new("hciconfig");
        cmd.args(&["-h"]);
        cmd
    })
    .await
    .map_err(|_| SysCheckError::CliToolFailed("hciconfig".to_owned()))?;
    // Check if `bluetoothctl` exists.
    run_system_command({
        let mut cmd = Command::new("bluetoothctl");
        cmd.args(&["-h"]);
        cmd
    })
    .await
    .map_err(|_| SysCheckError::CliToolFailed("bluetoothctl".to_owned()))?;
    // FIXME: Maybe we do not need to change bdaddr
    // Revisit for revise: https://github.com/thxomas/bdaddr
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

pub async fn prepare_device() -> Result<(), SysCheckError> {
    check_privileges().await?;
    check_system_requirements().await?;
    prepare_bluetooth_service()
        .await
        .map_err(|_| SysCheckError::PrepareFailed)?;
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
    restart_bluetooth_service()?;
    Ok(())
}

#[tracing::instrument(target = "system")]
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

#[tracing::instrument(target = "system")]
pub fn restart_bluetooth_service() -> Result<(), SystemCommandError> {
    systemctl::restart("bluetooth.service")?;
    Ok(())
}

#[tracing::instrument(target = "system")]
async fn prepare_bluetooth_service() -> Result<(), SystemCommandError> {
    // Turn off bluetooth scanning.
    //
    // This may fail for unknown reason, in that case, just ignore it.
    let _ = run_system_command({
        let mut cmd = Command::new("bluetoothctl");
        cmd.args(&["scan", "off"]);
        cmd
    })
    .await;
    Ok(())
}

#[derive(Clone, Error, Debug)]
pub enum InstallError {
    #[error("package manager failed")]
    PackageManagerFailed(String),
    #[error("other")]
    Other,
}

/// Installs system requirements and modifies configs for the device to work.
#[tracing::instrument(target = "system")]
async fn install_device() -> Result<(), InstallError> {
    // Update the `apt` registry to the latest.
    run_system_command({
        let mut cmd = Command::new("apt");
        cmd.args(&["update"]);
        cmd
    })
    .await
    .map_err(|err| {
        InstallError::PackageManagerFailed(format!(
            "failed to update apt index: {}",
            err.to_string()
        ))
    })?;
    // Run `apt` upgrades.
    run_system_command({
        let mut cmd = Command::new("apt");
        cmd.args(&["upgrade", "-y"]);
        cmd
    })
    .await
    .map_err(|err| {
        InstallError::PackageManagerFailed(format!(
            "failed to perform upgrade: {}",
            err.to_string()
        ))
    })?;
    // Install `bluez`.
    run_system_command({
        let mut cmd = Command::new("apt");
        cmd.args(&["update"]);
        cmd
    })
    .await
    .map_err(|err| {
        InstallError::PackageManagerFailed(format!(
            "failed to update apt index: {}",
            err.to_string()
        ))
    })?;
    // Setup `usbipd`.
    // Setup `dbus-broker`.

    systemctl::Ok(())
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
